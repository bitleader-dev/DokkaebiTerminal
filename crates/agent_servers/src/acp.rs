use acp_thread::{
    AgentConnection, AgentSessionInfo, AgentSessionList, AgentSessionListRequest,
    AgentSessionListResponse,
};
use acp_tools::AcpConnectionRegistry;
use action_log::ActionLog;
use agent_client_protocol::schema::{self as acp, ErrorCode};
use agent_client_protocol::{
    Agent, Client, ConnectionTo, JsonRpcResponse, Lines, Responder, SentRequest,
};
use anyhow::anyhow;
use collections::HashMap;
use feature_flags::{AcpBetaFeatureFlag, FeatureFlagAppExt as _};
use futures::channel::mpsc;
use futures::future::{FutureExt as _, Shared};
use futures::io::BufReader;
use futures::{AsyncBufReadExt as _, Future, StreamExt as _};
use project::agent_server_store::AgentServerCommand;
use project::{AgentId, Project};
use serde::Deserialize;
use task::{Shell, ShellBuilder, SpawnInTerminal};
use util::ResultExt as _;
use util::path_list::PathList;
use util::process::Child;

use std::path::PathBuf;
use std::process::Stdio;
use std::rc::Rc;
use std::sync::Arc;
use std::{any::Any, cell::RefCell};
use thiserror::Error;

use anyhow::{Context as _, Result};
use gpui::{App, AppContext as _, AsyncApp, Entity, SharedString, Task, WeakEntity};

use acp_thread::{AcpThread, AuthRequired, LoadError, TerminalProviderEvent};
use terminal::TerminalBuilder;
use terminal::terminal_settings::{AlternateScroll, CursorShape};

use crate::GEMINI_ID;

pub const GEMINI_TERMINAL_AUTH_METHOD_ID: &str = "spawn-gemini-cli";

/// GPUI 포그라운드 태스크에서 ACP 요청의 응답을 기다린다.
///
/// ACP SDK 는 [`SentRequest`] 를 소비하는 두 가지 방법을 제공한다.
///   - [`SentRequest::block_task`]: 별도 태스크에서 `.await` 로 선형 대기.
///   - [`SentRequest::on_receiving_result`]: 응답 도착 시 호출되는 콜백.
///     콜백 실행 중 다른 인바운드 메시지가 처리되지 않는 것이 보장되므로
///     SDK 핸들러 콜백 내부에서 권장된다 ([`block_task`] 는 이 경우 교착됨).
///
/// 핸들러 측 경로가 단일 요청 대기 헬퍼를 공유할 수 있도록
/// `on_receiving_result` + oneshot 채널 조합을 사용한다. 콜백 자체는 채널
/// 송신 하나뿐이라 디스패치 루프에 주는 순서 제약은 미미하다.
fn into_foreground_future<T: JsonRpcResponse>(
    sent: SentRequest<T>,
) -> impl Future<Output = Result<T, acp::Error>> {
    let (tx, rx) = futures::channel::oneshot::channel();
    let spawn_result = sent.on_receiving_result(async move |result| {
        tx.send(result).ok();
        Ok(())
    });
    async move {
        spawn_result?;
        rx.await.map_err(|_| {
            acp::Error::internal_error()
                .data("response channel cancelled — connection may have dropped")
        })?
    }
}

#[derive(Debug, Error)]
#[error("Unsupported version")]
pub struct UnsupportedVersion;

/// `entity.update(cx, |_, cx| fallible_op(cx))` 에서 나오는 중첩 `Result`
/// 모양을 단일 `Result<T, acp::Error>` 로 평탄화하는 헬퍼.
///
/// `anyhow::Error` 값은 `acp::Error::from` 을 거쳐 변환되며, 내부에 감싸진
/// `acp::Error` 는 다시 추출된다. 그래서 auth-required 같은 타입된 에러도
/// 왕복 과정에서 보존된다.
trait FlattenAcpResult<T> {
    fn flatten_acp(self) -> Result<T, acp::Error>;
}

impl<T> FlattenAcpResult<T> for Result<Result<T, anyhow::Error>, anyhow::Error> {
    fn flatten_acp(self) -> Result<T, acp::Error> {
        match self {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => Err(err.into()),
            Err(err) => Err(err.into()),
        }
    }
}

impl<T> FlattenAcpResult<T> for Result<Result<T, acp::Error>, anyhow::Error> {
    fn flatten_acp(self) -> Result<T, acp::Error> {
        match self {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(err.into()),
        }
    }
}

/// 백그라운드 핸들러 클로저에서 포그라운드 태스크로 넘길 상태를 담는다.
struct ClientContext {
    sessions: Rc<RefCell<HashMap<acp::SessionId, AcpSession>>>,
    session_list: Rc<RefCell<Option<Rc<AcpSessionList>>>>,
}

fn dispatch_queue_closed_error() -> acp::Error {
    acp::Error::internal_error().data("ACP foreground dispatch queue closed")
}

/// `Send` 핸들러 클로저에서 `!Send` 포그라운드 스레드로 넘기는 작업 단위.
trait ForegroundWorkItem: Send {
    fn run(self: Box<Self>, cx: &mut AsyncApp, ctx: &ClientContext);
    fn reject(self: Box<Self>);
}

type ForegroundWork = Box<dyn ForegroundWorkItem>;

struct RequestForegroundWork<Req, Res>
where
    Req: Send + 'static,
    Res: JsonRpcResponse + Send + 'static,
{
    request: Req,
    responder: Responder<Res>,
    handler: fn(Req, Responder<Res>, &mut AsyncApp, &ClientContext),
}

impl<Req, Res> ForegroundWorkItem for RequestForegroundWork<Req, Res>
where
    Req: Send + 'static,
    Res: JsonRpcResponse + Send + 'static,
{
    fn run(self: Box<Self>, cx: &mut AsyncApp, ctx: &ClientContext) {
        let Self {
            request,
            responder,
            handler,
        } = *self;
        handler(request, responder, cx, ctx);
    }

    fn reject(self: Box<Self>) {
        let Self { responder, .. } = *self;
        log::error!("ACP foreground dispatch queue closed while handling inbound request");
        responder
            .respond_with_error(dispatch_queue_closed_error())
            .log_err();
    }
}

struct NotificationForegroundWork<Notif>
where
    Notif: Send + 'static,
{
    notification: Notif,
    connection: ConnectionTo<Agent>,
    handler: fn(Notif, &mut AsyncApp, &ClientContext),
}

impl<Notif> ForegroundWorkItem for NotificationForegroundWork<Notif>
where
    Notif: Send + 'static,
{
    fn run(self: Box<Self>, cx: &mut AsyncApp, ctx: &ClientContext) {
        let Self {
            notification,
            handler,
            ..
        } = *self;
        handler(notification, cx, ctx);
    }

    fn reject(self: Box<Self>) {
        let Self { connection, .. } = *self;
        log::error!("ACP foreground dispatch queue closed while handling inbound notification");
        connection
            .send_error_notification(dispatch_queue_closed_error())
            .log_err();
    }
}

fn enqueue_request<Req, Res>(
    dispatch_tx: &mpsc::UnboundedSender<ForegroundWork>,
    request: Req,
    responder: Responder<Res>,
    handler: fn(Req, Responder<Res>, &mut AsyncApp, &ClientContext),
) where
    Req: Send + 'static,
    Res: JsonRpcResponse + Send + 'static,
{
    let work: ForegroundWork = Box::new(RequestForegroundWork {
        request,
        responder,
        handler,
    });
    if let Err(err) = dispatch_tx.unbounded_send(work) {
        err.into_inner().reject();
    }
}

fn enqueue_notification<Notif>(
    dispatch_tx: &mpsc::UnboundedSender<ForegroundWork>,
    notification: Notif,
    connection: ConnectionTo<Agent>,
    handler: fn(Notif, &mut AsyncApp, &ClientContext),
) where
    Notif: Send + 'static,
{
    let work: ForegroundWork = Box::new(NotificationForegroundWork {
        notification,
        connection,
        handler,
    });
    if let Err(err) = dispatch_tx.unbounded_send(work) {
        err.into_inner().reject();
    }
}

pub struct AcpConnection {
    id: AgentId,
    telemetry_id: SharedString,
    connection: ConnectionTo<Agent>,
    sessions: Rc<RefCell<HashMap<acp::SessionId, AcpSession>>>,
    /// load/resume RPC 진행 중인 세션. 응답 수신 전에 들어온 동일 세션 호출을 병합한다.
    pending_sessions: Rc<RefCell<HashMap<acp::SessionId, PendingSession>>>,
    auth_methods: Vec<acp::AuthMethod>,
    command: AgentServerCommand,
    agent_capabilities: acp::AgentCapabilities,
    default_mode: Option<acp::SessionModeId>,
    default_model: Option<acp::ModelId>,
    default_config_options: HashMap<String, String>,
    child: Child,
    session_list: Option<Rc<AcpSessionList>>,
    _io_task: Task<()>,
    _dispatch_task: Task<()>,
    _wait_task: Task<Result<()>>,
    _stderr_task: Task<Result<()>>,
}

struct ConfigOptions {
    config_options: Rc<RefCell<Vec<acp::SessionConfigOption>>>,
    tx: Rc<RefCell<watch::Sender<()>>>,
    rx: watch::Receiver<()>,
}

impl ConfigOptions {
    fn new(config_options: Rc<RefCell<Vec<acp::SessionConfigOption>>>) -> Self {
        let (tx, rx) = watch::channel(());
        Self {
            config_options,
            tx: Rc::new(RefCell::new(tx)),
            rx,
        }
    }
}

pub struct AcpSession {
    thread: WeakEntity<AcpThread>,
    suppress_abort_err: bool,
    models: Option<Rc<RefCell<acp::SessionModelState>>>,
    session_modes: Option<Rc<RefCell<acp::SessionModeState>>>,
    config_options: Option<ConfigOptions>,
    /// 이 세션을 참조하는 호출 수. 마지막 `close_session` 호출로 0 이 되면 실제 RPC 를 전송한다.
    ref_count: usize,
}

/// 동일 세션에 대한 동시 load/resume 요청을 병합한다.
/// 진행 중인 RPC task 를 `Shared` 로 공유하고 `ref_count` 로 호출 횟수를 추적한다.
struct PendingSession {
    task: Shared<Task<Result<Entity<AcpThread>, Arc<anyhow::Error>>>>,
    ref_count: usize,
}

pub struct AcpSessionList {
    connection: ConnectionTo<Agent>,
    updates_tx: smol::channel::Sender<acp_thread::SessionListUpdate>,
    updates_rx: smol::channel::Receiver<acp_thread::SessionListUpdate>,
}

impl AcpSessionList {
    fn new(connection: ConnectionTo<Agent>) -> Self {
        let (tx, rx) = smol::channel::unbounded();
        Self {
            connection,
            updates_tx: tx,
            updates_rx: rx,
        }
    }

    fn notify_update(&self) {
        self.updates_tx
            .try_send(acp_thread::SessionListUpdate::Refresh)
            .log_err();
    }

    fn send_info_update(&self, session_id: acp::SessionId, update: acp::SessionInfoUpdate) {
        self.updates_tx
            .try_send(acp_thread::SessionListUpdate::SessionInfo { session_id, update })
            .log_err();
    }
}

impl AgentSessionList for AcpSessionList {
    fn list_sessions(
        &self,
        request: AgentSessionListRequest,
        cx: &mut App,
    ) -> Task<Result<AgentSessionListResponse>> {
        let conn = self.connection.clone();
        cx.foreground_executor().spawn(async move {
            let acp_request = acp::ListSessionsRequest::new()
                .cwd(request.cwd)
                .cursor(request.cursor);
            let response = into_foreground_future(conn.send_request(acp_request))
                .await
                .map_err(map_acp_error)?;
            Ok(AgentSessionListResponse {
                sessions: response
                    .sessions
                    .into_iter()
                    .map(|s| AgentSessionInfo {
                        session_id: s.session_id,
                        work_dirs: Some(PathList::new(&[s.cwd])),
                        title: s.title.map(Into::into),
                        updated_at: s.updated_at.and_then(|date_str| {
                            chrono::DateTime::parse_from_rfc3339(&date_str)
                                .ok()
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                        }),
                        created_at: None,
                        meta: s.meta,
                    })
                    .collect(),
                next_cursor: response.next_cursor,
                meta: response.meta,
            })
        })
    }

    fn watch(
        &self,
        _cx: &mut App,
    ) -> Option<smol::channel::Receiver<acp_thread::SessionListUpdate>> {
        Some(self.updates_rx.clone())
    }

    fn notify_refresh(&self) {
        self.notify_update();
    }

    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

pub async fn connect(
    agent_id: AgentId,
    project: Entity<Project>,
    command: AgentServerCommand,
    default_mode: Option<acp::SessionModeId>,
    default_model: Option<acp::ModelId>,
    default_config_options: HashMap<String, String>,
    cx: &mut AsyncApp,
) -> Result<Rc<dyn AgentConnection>> {
    let conn = AcpConnection::stdio(
        agent_id,
        project,
        command.clone(),
        default_mode,
        default_model,
        default_config_options,
        cx,
    )
    .await?;
    Ok(Rc::new(conn) as _)
}

const MINIMUM_SUPPORTED_VERSION: acp::ProtocolVersion = acp::ProtocolVersion::V1;

/// `transport` 위에 Dokkaebi 의 전체 에이전트→클라이언트 핸들러 세트를
/// 연결해 `Client` 측 연결을 구성한다.
///
/// 모든 인바운드 요청·알림은 `dispatch_tx` 를 통해 포그라운드 디스패치
/// 큐에 실려 `handle_*` 함수에서 GPUI 컨텍스트 위에 처리된다. 반환되는
/// future 는 트랜스포트가 닫힐 때까지 연결을 유지하며, 호출자는 이를
/// 백그라운드 익스큐터에 올려 두는 것을 기대한다. `connection_tx` 는
/// 빌더의 `main_fn` 이 실행되는 즉시 `ConnectionTo<Agent>` 핸들을 전달한다.
fn connect_client_future(
    name: &'static str,
    transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
    dispatch_tx: mpsc::UnboundedSender<ForegroundWork>,
    connection_tx: futures::channel::oneshot::Sender<ConnectionTo<Agent>>,
) -> impl Future<Output = Result<(), acp::Error>> {
    // 각 핸들러는 입력을 포그라운드 디스패치 큐에 전달한다. SDK 는 클로저가
    // `Send` 이기를 요구하므로 `dispatch_tx` 의 복제본을 각각 소유하게 한다.
    macro_rules! on_request {
        ($handler:ident) => {{
            let dispatch_tx = dispatch_tx.clone();
            async move |req, responder, _connection| {
                enqueue_request(&dispatch_tx, req, responder, $handler);
                Ok(())
            }
        }};
    }
    macro_rules! on_notification {
        ($handler:ident) => {{
            let dispatch_tx = dispatch_tx.clone();
            async move |notif, connection| {
                enqueue_notification(&dispatch_tx, notif, connection, $handler);
                Ok(())
            }
        }};
    }

    Client
        .builder()
        .name(name)
        // --- 요청 핸들러 (에이전트→클라이언트) ---
        .on_receive_request(
            on_request!(handle_request_permission),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_write_text_file),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_read_text_file),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_create_terminal),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_kill_terminal),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_release_terminal),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_terminal_output),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            on_request!(handle_wait_for_terminal_exit),
            agent_client_protocol::on_receive_request!(),
        )
        // --- 알림 핸들러 (에이전트→클라이언트) ---
        .on_receive_notification(
            on_notification!(handle_session_notification),
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(
            transport,
            move |connection: ConnectionTo<Agent>| async move {
                if connection_tx.send(connection).is_err() {
                    log::error!("failed to send ACP connection handle — receiver was dropped");
                }
                // 트랜스포트가 닫힐 때까지 연결을 유지한다.
                futures::future::pending::<Result<(), acp::Error>>().await
            },
        )
}

impl AcpConnection {
    pub async fn stdio(
        agent_id: AgentId,
        project: Entity<Project>,
        command: AgentServerCommand,
        default_mode: Option<acp::SessionModeId>,
        default_model: Option<acp::ModelId>,
        default_config_options: HashMap<String, String>,
        cx: &mut AsyncApp,
    ) -> Result<Self> {
        let builder = ShellBuilder::new(&Shell::System, cfg!(windows)).non_interactive();
        let mut child =
            builder.build_std_command(Some(command.path.display().to_string()), &command.args);
        child.envs(command.env.iter().flatten());
        if let Some(cwd) = project.update(cx, |project, cx| {
            if project.is_local() {
                project
                    .default_path_list(cx)
                    .ordered_paths()
                    .next()
                    .cloned()
            } else {
                None
            }
        }) {
            child.current_dir(cwd);
        }
        let mut child = Child::spawn(child, Stdio::piped(), Stdio::piped(), Stdio::piped())?;

        let stdout = child.stdout.take().context("Failed to take stdout")?;
        let stdin = child.stdin.take().context("Failed to take stdin")?;
        let stderr = child.stderr.take().context("Failed to take stderr")?;
        log::debug!(
            "Spawning external agent server: {:?}, {:?}",
            command.path,
            command.args
        );
        log::trace!("Spawned (pid: {})", child.id());

        let sessions = Rc::new(RefCell::new(HashMap::default()));
        let pending_sessions = Rc::new(RefCell::new(HashMap::default()));

        let (release_channel, version): (Option<&str>, String) = cx.update(|cx| {
            (
                release_channel::ReleaseChannel::try_global(cx)
                    .map(|release_channel| release_channel.display_name()),
                release_channel::AppVersion::global(cx).to_string(),
            )
        });

        let client_session_list: Rc<RefCell<Option<Rc<AcpSessionList>>>> =
            Rc::new(RefCell::new(None));

        // Send 핸들러 클로저에서 !Send 포그라운드 스레드로 넘기는 디스패치 채널.
        let (dispatch_tx, dispatch_rx) = mpsc::unbounded::<ForegroundWork>();

        // 로그 패널 레지스트리에 이 연결을 등록한다. 반환된 탭(tap) 은 옵트인
        // 이라 ACP 로그 패널에 구독자가 없는 동안 `emit_*` 호출 비용은 거의
        // 없다 (원자 연산 + 반환).
        let log_tap = cx.update(|cx| {
            AcpConnectionRegistry::default_global(cx).update(cx, |registry, cx| {
                registry.set_active_connection(agent_id.clone(), cx)
            })
        });

        let incoming_lines = futures::io::BufReader::new(stdout).lines();
        let tapped_incoming = incoming_lines.inspect({
            let log_tap = log_tap.clone();
            move |result| match result {
                Ok(line) => log_tap.emit_incoming(line),
                Err(err) => {
                    // SDK 관점에서 트랜스포트 I/O 에러는 치명적이지만, 별도
                    // 로깅이 없으면 ACP 로그 패널에 연결 종료 흔적이 남지 않는다.
                    log::warn!("ACP transport read error: {err}");
                }
            }
        });

        let tapped_outgoing = futures::sink::unfold(
            (Box::pin(stdin), log_tap.clone()),
            async move |(mut writer, log_tap), line: String| {
                use futures::AsyncWriteExt;
                log_tap.emit_outgoing(&line);
                let mut bytes = line.into_bytes();
                bytes.push(b'\n');
                writer.write_all(&bytes).await?;
                Ok::<_, std::io::Error>((writer, log_tap))
            },
        );

        let transport = Lines::new(tapped_outgoing, tapped_incoming);

        // `connect_client_future` 가 프로덕션 핸들러 세트를 설치하고, 백그라운드
        // 익스큐터에서 구동할 connection-future 와 트랜스포트 핸드셰이크가
        // 완료되면 `ConnectionTo<Agent>` 핸들을 돌려주는 oneshot 수신자를
        // 반환한다.
        let (connection_tx, connection_rx) = futures::channel::oneshot::channel();
        let connection_future =
            connect_client_future("zed", transport, dispatch_tx.clone(), connection_tx);
        let io_task = cx.background_spawn(async move {
            if let Err(err) = connection_future.await {
                log::error!("ACP connection error: {err}");
            }
        });

        let connection: ConnectionTo<Agent> = connection_rx
            .await
            .context("Failed to receive ACP connection handle")?;

        // 핸들러에서 들어온 작업을 처리할 포그라운드 디스패치 루프.
        let dispatch_context = ClientContext {
            sessions: sessions.clone(),
            session_list: client_session_list.clone(),
        };
        let dispatch_task = cx.spawn({
            let mut dispatch_rx = dispatch_rx;
            async move |cx| {
                while let Some(work) = dispatch_rx.next().await {
                    work.run(cx, &dispatch_context);
                }
            }
        });

        let stderr_task = cx.background_spawn({
            let log_tap = log_tap.clone();
            async move {
                let mut stderr = BufReader::new(stderr);
                let mut line = String::new();
                while let Ok(n) = stderr.read_line(&mut line).await
                    && n > 0
                {
                    let trimmed = line.trim_end_matches(['\n', '\r']);
                    log::warn!("agent stderr: {trimmed}");
                    log_tap.emit_stderr(trimmed);
                    line.clear();
                }
                Ok(())
            }
        });

        let wait_task = cx.spawn({
            let sessions = sessions.clone();
            let status_fut = child.status();
            async move |cx| {
                let status = status_fut.await?;

                emit_load_error_to_all_sessions(&sessions, LoadError::Exited { status }, cx);

                anyhow::Ok(())
            }
        });

        let response = into_foreground_future(
            connection.send_request(
                acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                    .client_capabilities(
                        acp::ClientCapabilities::new()
                            .fs(acp::FileSystemCapabilities::new()
                                .read_text_file(true)
                                .write_text_file(true))
                            .terminal(true)
                            .auth(acp::AuthCapabilities::new().terminal(true))
                            // Experimental: 에이전트의 터미널 출력 렌더링 허용
                            .meta(acp::Meta::from_iter([
                                ("terminal_output".into(), true.into()),
                                ("terminal-auth".into(), true.into()),
                            ])),
                    )
                    .client_info(
                        acp::Implementation::new("zed", version)
                            .title(release_channel.map(ToOwned::to_owned)),
                    ),
            ),
        )
        .await?;

        if response.protocol_version < MINIMUM_SUPPORTED_VERSION {
            return Err(UnsupportedVersion.into());
        }

        let telemetry_id = response
            .agent_info
            // 에이전트가 제공하는 이름을 우선 사용한다.
            .map(|info| info.name.into())
            // 없으면 agent id 를 그대로 사용한다.
            .unwrap_or_else(|| agent_id.0.to_string().into());

        let session_list = if response
            .agent_capabilities
            .session_capabilities
            .list
            .is_some()
        {
            let list = Rc::new(AcpSessionList::new(connection.clone()));
            *client_session_list.borrow_mut() = Some(list.clone());
            Some(list)
        } else {
            None
        };

        // TODO: Gemini 팀이 공식 auth 방식을 릴리즈하면 이 우회를 제거한다.
        let auth_methods = if agent_id.0.as_ref() == GEMINI_ID {
            let mut args = command.args.clone();
            args.retain(|a| a != "--experimental-acp" && a != "--acp");
            let value = serde_json::json!({
                "label": "gemini /auth",
                "command": command.path.to_string_lossy().into_owned(),
                "args": args,
                "env": command.env.clone().unwrap_or_default(),
            });
            let meta = acp::Meta::from_iter([("terminal-auth".to_string(), value)]);
            vec![acp::AuthMethod::Agent(
                acp::AuthMethodAgent::new(GEMINI_TERMINAL_AUTH_METHOD_ID, "Login")
                    .description("Login with your Google or Vertex AI account")
                    .meta(meta),
            )]
        } else {
            response.auth_methods
        };
        Ok(Self {
            id: agent_id,
            auth_methods,
            command,
            connection,
            telemetry_id,
            sessions,
            pending_sessions,
            agent_capabilities: response.agent_capabilities,
            default_mode,
            default_model,
            default_config_options,
            session_list,
            _io_task: io_task,
            _dispatch_task: dispatch_task,
            _wait_task: wait_task,
            _stderr_task: stderr_task,
            child,
        })
    }

    pub fn prompt_capabilities(&self) -> &acp::PromptCapabilities {
        &self.agent_capabilities.prompt_capabilities
    }

    fn apply_default_config_options(
        &self,
        session_id: &acp::SessionId,
        config_options: &Rc<RefCell<Vec<acp::SessionConfigOption>>>,
        cx: &mut AsyncApp,
    ) {
        let id = self.id.clone();
        let defaults_to_apply: Vec<_> = {
            let config_opts_ref = config_options.borrow();
            config_opts_ref
                .iter()
                .filter_map(|config_option| {
                    let default_value = self.default_config_options.get(&*config_option.id.0)?;

                    let is_valid = match &config_option.kind {
                        acp::SessionConfigKind::Select(select) => match &select.options {
                            acp::SessionConfigSelectOptions::Ungrouped(options) => options
                                .iter()
                                .any(|opt| &*opt.value.0 == default_value.as_str()),
                            acp::SessionConfigSelectOptions::Grouped(groups) => {
                                groups.iter().any(|g| {
                                    g.options
                                        .iter()
                                        .any(|opt| &*opt.value.0 == default_value.as_str())
                                })
                            }
                            _ => false,
                        },
                        _ => false,
                    };

                    if is_valid {
                        let initial_value = match &config_option.kind {
                            acp::SessionConfigKind::Select(select) => {
                                Some(select.current_value.clone())
                            }
                            _ => None,
                        };
                        Some((
                            config_option.id.clone(),
                            default_value.clone(),
                            initial_value,
                        ))
                    } else {
                        log::warn!(
                            "`{}` is not a valid value for config option `{}` in {}",
                            default_value,
                            config_option.id.0,
                            id
                        );
                        None
                    }
                })
                .collect()
        };

        for (config_id, default_value, initial_value) in defaults_to_apply {
            cx.spawn({
                let default_value_id = acp::SessionConfigValueId::new(default_value.clone());
                let session_id = session_id.clone();
                let config_id_clone = config_id.clone();
                let config_opts = config_options.clone();
                let conn = self.connection.clone();
                async move |_| {
                    let result = into_foreground_future(conn.send_request(
                        acp::SetSessionConfigOptionRequest::new(
                            session_id,
                            config_id_clone.clone(),
                            default_value_id,
                        ),
                    ))
                    .await
                    .log_err();

                    if result.is_none() {
                        if let Some(initial) = initial_value {
                            let mut opts = config_opts.borrow_mut();
                            if let Some(opt) = opts.iter_mut().find(|o| o.id == config_id_clone) {
                                if let acp::SessionConfigKind::Select(select) = &mut opt.kind {
                                    select.current_value = initial;
                                }
                            }
                        }
                    }
                }
            })
            .detach();

            let mut opts = config_options.borrow_mut();
            if let Some(opt) = opts.iter_mut().find(|o| o.id == config_id) {
                if let acp::SessionConfigKind::Select(select) = &mut opt.kind {
                    select.current_value = acp::SessionConfigValueId::new(default_value);
                }
            }
        }
    }
}

// 프로세스 종료 시 모든 세션에 로드 에러를 emit 한다. sessions.borrow() 를
// 그대로 돌며 session.thread.update 를 호출하면 update 내부에서 sessions 를
// 다시 borrow 해 double borrow panic 이 발생하므로 thread 를 먼저 clone 해
// borrow 를 해제한 뒤 순회한다.
fn emit_load_error_to_all_sessions(
    sessions: &Rc<RefCell<HashMap<acp::SessionId, AcpSession>>>,
    error: LoadError,
    cx: &mut AsyncApp,
) {
    let threads: Vec<_> = sessions
        .borrow()
        .values()
        .map(|session| session.thread.clone())
        .collect();

    for thread in threads {
        thread
            .update(cx, |thread, cx| thread.emit_load_error(error.clone(), cx))
            .ok();
    }
}

impl Drop for AcpConnection {
    fn drop(&mut self) {
        self.child.kill().log_err();
    }
}

fn terminal_auth_task_id(agent_id: &AgentId, method_id: &acp::AuthMethodId) -> String {
    format!("external-agent-{}-{}-login", agent_id.0, method_id.0)
}

fn terminal_auth_task(
    command: &AgentServerCommand,
    agent_id: &AgentId,
    method: &acp::AuthMethodTerminal,
) -> SpawnInTerminal {
    let mut args = command.args.clone();
    args.extend(method.args.clone());

    let mut env = command.env.clone().unwrap_or_default();
    env.extend(method.env.clone());

    acp_thread::build_terminal_auth_task(
        terminal_auth_task_id(agent_id, &method.id),
        method.name.clone(),
        command.path.to_string_lossy().into_owned(),
        args,
        env,
    )
}

/// 안정화 전 _meta 경로의 터미널 인증을 지원하기 위한 헬퍼.
fn meta_terminal_auth_task(
    agent_id: &AgentId,
    method_id: &acp::AuthMethodId,
    method: &acp::AuthMethod,
) -> Option<SpawnInTerminal> {
    #[derive(Deserialize)]
    struct MetaTerminalAuth {
        label: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    }

    let meta = match method {
        acp::AuthMethod::EnvVar(env_var) => env_var.meta.as_ref(),
        acp::AuthMethod::Terminal(terminal) => terminal.meta.as_ref(),
        acp::AuthMethod::Agent(agent) => agent.meta.as_ref(),
        _ => None,
    }?;
    let terminal_auth =
        serde_json::from_value::<MetaTerminalAuth>(meta.get("terminal-auth")?.clone()).ok()?;

    Some(acp_thread::build_terminal_auth_task(
        terminal_auth_task_id(agent_id, method_id),
        terminal_auth.label.clone(),
        terminal_auth.command,
        terminal_auth.args,
        terminal_auth.env,
    ))
}

impl AgentConnection for AcpConnection {
    fn agent_id(&self) -> AgentId {
        self.id.clone()
    }

    fn telemetry_id(&self) -> SharedString {
        self.telemetry_id.clone()
    }

    fn new_session(
        self: Rc<Self>,
        project: Entity<Project>,
        work_dirs: PathList,
        cx: &mut App,
    ) -> Task<Result<Entity<AcpThread>>> {
        // TODO: ACP 가 다중 작업 디렉터리를 지원하면 제거한다.
        let Some(cwd) = work_dirs.ordered_paths().next().cloned() else {
            return Task::ready(Err(anyhow!("Working directory cannot be empty")));
        };
        let name = self.id.0.clone();
        let mcp_servers = mcp_servers_for_project(&project, cx);

        cx.spawn(async move |cx| {
            let response = into_foreground_future(
                self.connection
                    .send_request(acp::NewSessionRequest::new(cwd.clone()).mcp_servers(mcp_servers)),
            )
            .await
            .map_err(map_acp_error)?;

            let (modes, models, config_options) = config_state(response.modes, response.models, response.config_options);

            if let Some(default_mode) = self.default_mode.clone() {
                if let Some(modes) = modes.as_ref() {
                    let mut modes_ref = modes.borrow_mut();
                    let has_mode = modes_ref.available_modes.iter().any(|mode| mode.id == default_mode);

                    if has_mode {
                        let initial_mode_id = modes_ref.current_mode_id.clone();

                        cx.spawn({
                            let default_mode = default_mode.clone();
                            let session_id = response.session_id.clone();
                            let modes = modes.clone();
                            let conn = self.connection.clone();
                            async move |_| {
                                let result = into_foreground_future(conn.send_request(
                                    acp::SetSessionModeRequest::new(session_id, default_mode),
                                ))
                                .await
                                .log_err();

                                if result.is_none() {
                                    modes.borrow_mut().current_mode_id = initial_mode_id;
                                }
                            }
                        }).detach();

                        modes_ref.current_mode_id = default_mode;
                    } else {
                        let available_modes = modes_ref
                            .available_modes
                            .iter()
                            .map(|mode| format!("- `{}`: {}", mode.id, mode.name))
                            .collect::<Vec<_>>()
                            .join("\n");

                        log::warn!(
                            "`{default_mode}` is not valid {name} mode. Available options:\n{available_modes}",
                        );
                    }
                }
            }

            if let Some(default_model) = self.default_model.clone() {
                if let Some(models) = models.as_ref() {
                    let mut models_ref = models.borrow_mut();
                    let has_model = models_ref.available_models.iter().any(|model| model.model_id == default_model);

                    if has_model {
                        let initial_model_id = models_ref.current_model_id.clone();

                        cx.spawn({
                            let default_model = default_model.clone();
                            let session_id = response.session_id.clone();
                            let models = models.clone();
                            let conn = self.connection.clone();
                            async move |_| {
                                let result = into_foreground_future(conn.send_request(
                                    acp::SetSessionModelRequest::new(session_id, default_model),
                                ))
                                .await
                                .log_err();

                                if result.is_none() {
                                    models.borrow_mut().current_model_id = initial_model_id;
                                }
                            }
                        }).detach();

                        models_ref.current_model_id = default_model;
                    } else {
                        let available_models = models_ref
                            .available_models
                            .iter()
                            .map(|model| format!("- `{}`: {}", model.model_id, model.name))
                            .collect::<Vec<_>>()
                            .join("\n");

                        log::warn!(
                            "`{default_model}` is not a valid {name} model. Available options:\n{available_models}",
                        );
                    }
                }
            }

            if let Some(config_opts) = config_options.as_ref() {
                self.apply_default_config_options(&response.session_id, config_opts, cx);
            }

            let action_log = cx.new(|_| ActionLog::new(project.clone()));
            let thread: Entity<AcpThread> = cx.new(|cx| {
                AcpThread::new(
                    None,
                    None,
                    Some(work_dirs),
                    self.clone(),
                    project,
                    action_log,
                    response.session_id.clone(),
                    // ACP 는 현재 세션별 prompt capability 변경을 지원하지 않는다.
                    watch::Receiver::constant(self.agent_capabilities.prompt_capabilities.clone()),
                    cx,
                )
            });

            self.sessions.borrow_mut().insert(
                response.session_id,
                AcpSession {
                    thread: thread.downgrade(),
                    suppress_abort_err: false,
                    session_modes: modes,
                    models,
                    config_options: config_options.map(ConfigOptions::new),
                    ref_count: 1,
                },
            );

            Ok(thread)
        })
    }

    fn supports_load_session(&self) -> bool {
        self.agent_capabilities.load_session
    }

    fn supports_resume_session(&self) -> bool {
        self.agent_capabilities
            .session_capabilities
            .resume
            .is_some()
    }

    fn load_session(
        self: Rc<Self>,
        session_id: acp::SessionId,
        project: Entity<Project>,
        work_dirs: PathList,
        title: Option<SharedString>,
        cx: &mut App,
    ) -> Task<Result<Entity<AcpThread>>> {
        if !self.agent_capabilities.load_session {
            return Task::ready(Err(anyhow!(LoadError::Other(
                "Loading sessions is not supported by this agent.".into()
            ))));
        }

        // 이미 load 진행 중인 세션이면 동일 task 를 공유하고 ref_count 만 증가시킨다.
        // 이렇게 하면 동시 호출이 중복 RPC / 중복 thread 를 만들지 않는다.
        if let Some(pending) = self.pending_sessions.borrow_mut().get_mut(&session_id) {
            pending.ref_count += 1;
            let task = pending.task.clone();
            return cx
                .foreground_executor()
                .spawn(async move { task.await.map_err(|err| anyhow!(err)) });
        }

        // 이미 완료된 세션이면 기존 thread 를 재사용하고 ref_count 만 증가시킨다.
        if let Some(session) = self.sessions.borrow_mut().get_mut(&session_id) {
            session.ref_count += 1;
            if let Some(thread) = session.thread.upgrade() {
                return Task::ready(Ok(thread));
            }
        }

        // TODO: ACP 가 다중 작업 디렉터리를 지원하면 제거한다.
        let Some(cwd) = work_dirs.ordered_paths().next().cloned() else {
            return Task::ready(Err(anyhow!("Working directory cannot be empty")));
        };

        let mcp_servers = mcp_servers_for_project(&project, cx);
        let action_log = cx.new(|_| ActionLog::new(project.clone()));
        let thread: Entity<AcpThread> = cx.new(|cx| {
            AcpThread::new(
                None,
                title,
                Some(work_dirs.clone()),
                self.clone(),
                project,
                action_log,
                session_id.clone(),
                watch::Receiver::constant(self.agent_capabilities.prompt_capabilities.clone()),
                cx,
            )
        });

        // RPC 를 기다리기 전에 session 을 등록한다.
        // `session/load` 호출 중 도착한 `session/update` replay 알림이 thread 를 찾을 수 있도록.
        self.sessions.borrow_mut().insert(
            session_id.clone(),
            AcpSession {
                thread: thread.downgrade(),
                suppress_abort_err: false,
                session_modes: None,
                models: None,
                config_options: None,
                ref_count: 1,
            },
        );

        let this = self.clone();
        let session_id_for_task = session_id.clone();
        let thread_for_task = thread.clone();
        let raw_task: Task<Result<Entity<AcpThread>, Arc<anyhow::Error>>> = cx
            .spawn(async move |cx| {
                let response = match into_foreground_future(this.connection.send_request(
                    acp::LoadSessionRequest::new(session_id_for_task.clone(), cwd)
                        .mcp_servers(mcp_servers),
                ))
                .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        this.sessions.borrow_mut().remove(&session_id_for_task);
                        this.pending_sessions.borrow_mut().remove(&session_id_for_task);
                        return Err(Arc::new(map_acp_error(err)));
                    }
                };

                let (modes, models, config_options) =
                    config_state(response.modes, response.models, response.config_options);

                if let Some(config_opts) = config_options.as_ref() {
                    this.apply_default_config_options(&session_id_for_task, config_opts, cx);
                }

                // pending_sessions 제거하면서 최종 ref_count 를 계산한다.
                let ref_count = this
                    .pending_sessions
                    .borrow_mut()
                    .remove(&session_id_for_task)
                    .map_or(1, |pending| pending.ref_count);

                if let Some(session) =
                    this.sessions.borrow_mut().get_mut(&session_id_for_task)
                {
                    session.session_modes = modes;
                    session.models = models;
                    session.config_options = config_options.map(ConfigOptions::new);
                    session.ref_count = ref_count;
                }

                Ok(thread_for_task)
            });
        let shared_task = raw_task.shared();

        self.pending_sessions.borrow_mut().insert(
            session_id.clone(),
            PendingSession {
                task: shared_task.clone(),
                ref_count: 1,
            },
        );

        cx.foreground_executor()
            .spawn(async move { shared_task.await.map_err(|err| anyhow!(err)) })
    }

    fn resume_session(
        self: Rc<Self>,
        session_id: acp::SessionId,
        project: Entity<Project>,
        work_dirs: PathList,
        title: Option<SharedString>,
        cx: &mut App,
    ) -> Task<Result<Entity<AcpThread>>> {
        if self
            .agent_capabilities
            .session_capabilities
            .resume
            .is_none()
        {
            return Task::ready(Err(anyhow!(LoadError::Other(
                "Resuming sessions is not supported by this agent.".into()
            ))));
        }

        // 이미 resume 진행 중인 세션이면 동일 task 를 공유하고 ref_count 만 증가시킨다.
        if let Some(pending) = self.pending_sessions.borrow_mut().get_mut(&session_id) {
            pending.ref_count += 1;
            let task = pending.task.clone();
            return cx
                .foreground_executor()
                .spawn(async move { task.await.map_err(|err| anyhow!(err)) });
        }

        // 이미 완료된 세션이면 기존 thread 를 재사용하고 ref_count 만 증가시킨다.
        if let Some(session) = self.sessions.borrow_mut().get_mut(&session_id) {
            session.ref_count += 1;
            if let Some(thread) = session.thread.upgrade() {
                return Task::ready(Ok(thread));
            }
        }

        // TODO: ACP 가 다중 작업 디렉터리를 지원하면 제거한다.
        let Some(cwd) = work_dirs.ordered_paths().next().cloned() else {
            return Task::ready(Err(anyhow!("Working directory cannot be empty")));
        };

        let mcp_servers = mcp_servers_for_project(&project, cx);
        let action_log = cx.new(|_| ActionLog::new(project.clone()));
        let thread: Entity<AcpThread> = cx.new(|cx| {
            AcpThread::new(
                None,
                title,
                Some(work_dirs),
                self.clone(),
                project,
                action_log,
                session_id.clone(),
                watch::Receiver::constant(self.agent_capabilities.prompt_capabilities.clone()),
                cx,
            )
        });

        // RPC 를 기다리기 전에 session 을 등록한다 (replay notification 수신 대비).
        self.sessions.borrow_mut().insert(
            session_id.clone(),
            AcpSession {
                thread: thread.downgrade(),
                suppress_abort_err: false,
                session_modes: None,
                models: None,
                config_options: None,
                ref_count: 1,
            },
        );

        let this = self.clone();
        let session_id_for_task = session_id.clone();
        let thread_for_task = thread.clone();
        let raw_task: Task<Result<Entity<AcpThread>, Arc<anyhow::Error>>> = cx
            .spawn(async move |cx| {
                let response = match into_foreground_future(this.connection.send_request(
                    acp::ResumeSessionRequest::new(session_id_for_task.clone(), cwd)
                        .mcp_servers(mcp_servers),
                ))
                .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        this.sessions.borrow_mut().remove(&session_id_for_task);
                        this.pending_sessions.borrow_mut().remove(&session_id_for_task);
                        return Err(Arc::new(map_acp_error(err)));
                    }
                };

                let (modes, models, config_options) =
                    config_state(response.modes, response.models, response.config_options);

                if let Some(config_opts) = config_options.as_ref() {
                    this.apply_default_config_options(&session_id_for_task, config_opts, cx);
                }

                let ref_count = this
                    .pending_sessions
                    .borrow_mut()
                    .remove(&session_id_for_task)
                    .map_or(1, |pending| pending.ref_count);

                if let Some(session) =
                    this.sessions.borrow_mut().get_mut(&session_id_for_task)
                {
                    session.session_modes = modes;
                    session.models = models;
                    session.config_options = config_options.map(ConfigOptions::new);
                    session.ref_count = ref_count;
                }

                Ok(thread_for_task)
            });
        let shared_task = raw_task.shared();

        self.pending_sessions.borrow_mut().insert(
            session_id.clone(),
            PendingSession {
                task: shared_task.clone(),
                ref_count: 1,
            },
        );

        cx.foreground_executor()
            .spawn(async move { shared_task.await.map_err(|err| anyhow!(err)) })
    }

    fn supports_close_session(&self) -> bool {
        self.agent_capabilities.session_capabilities.close.is_some()
    }

    fn close_session(
        self: Rc<Self>,
        session_id: &acp::SessionId,
        cx: &mut App,
    ) -> Task<Result<()>> {
        if !self.supports_close_session() {
            return Task::ready(Err(anyhow!(LoadError::Other(
                "Closing sessions is not supported by this agent.".into()
            ))));
        }

        // ref_count 를 감소시키고 0 이 될 때만 실제 close RPC 를 전송한다.
        // 이렇게 하면 동일 세션을 여러 번 load/resume 한 호출들이 서로의 close 를 밀지 않는다.
        let should_close = {
            let mut sessions = self.sessions.borrow_mut();
            if let Some(session) = sessions.get_mut(session_id) {
                session.ref_count = session.ref_count.saturating_sub(1);
                if session.ref_count == 0 {
                    sessions.remove(session_id);
                    true
                } else {
                    false
                }
            } else {
                // 세션이 이미 제거된 경우(로드 실패 등) 에도 상위 호출자에게 성공을 알린다.
                false
            }
        };
        if !should_close {
            return Task::ready(Ok(()));
        }

        let conn = self.connection.clone();
        let session_id = session_id.clone();
        cx.foreground_executor().spawn(async move {
            into_foreground_future(
                conn.send_request(acp::CloseSessionRequest::new(session_id.clone())),
            )
            .await?;
            Ok(())
        })
    }

    fn auth_methods(&self) -> &[acp::AuthMethod] {
        &self.auth_methods
    }

    fn terminal_auth_task(
        &self,
        method_id: &acp::AuthMethodId,
        cx: &App,
    ) -> Option<SpawnInTerminal> {
        let method = self
            .auth_methods
            .iter()
            .find(|method| method.id() == method_id)?;

        match method {
            acp::AuthMethod::Terminal(terminal) if cx.has_flag::<AcpBetaFeatureFlag>() => {
                Some(terminal_auth_task(&self.command, &self.id, terminal))
            }
            _ => meta_terminal_auth_task(&self.id, method_id, method),
        }
    }

    fn authenticate(&self, method_id: acp::AuthMethodId, cx: &mut App) -> Task<Result<()>> {
        let conn = self.connection.clone();
        cx.foreground_executor().spawn(async move {
            into_foreground_future(conn.send_request(acp::AuthenticateRequest::new(method_id)))
                .await?;
            Ok(())
        })
    }

    fn prompt(
        &self,
        _id: Option<acp_thread::UserMessageId>,
        params: acp::PromptRequest,
        cx: &mut App,
    ) -> Task<Result<acp::PromptResponse>> {
        let conn = self.connection.clone();
        let sessions = self.sessions.clone();
        let session_id = params.session_id.clone();
        cx.foreground_executor().spawn(async move {
            let result = into_foreground_future(conn.send_request(params)).await;

            let mut suppress_abort_err = false;

            if let Some(session) = sessions.borrow_mut().get_mut(&session_id) {
                suppress_abort_err = session.suppress_abort_err;
                session.suppress_abort_err = false;
            }

            match result {
                Ok(response) => Ok(response),
                Err(err) => {
                    if err.code == acp::ErrorCode::AuthRequired {
                        return Err(anyhow!(acp::Error::auth_required()));
                    }

                    if err.code != ErrorCode::InternalError {
                        anyhow::bail!(err)
                    }

                    let Some(data) = &err.data else {
                        anyhow::bail!(err)
                    };

                    // 다음 PR 이 일반 제공되기 전까지의 임시 우회:
                    // https://github.com/google-gemini/gemini-cli/pull/6656

                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct ErrorDetails {
                        details: Box<str>,
                    }

                    match serde_json::from_value(data.clone()) {
                        Ok(ErrorDetails { details }) => {
                            if suppress_abort_err
                                && (details.contains("This operation was aborted")
                                    || details.contains("The user aborted a request"))
                            {
                                Ok(acp::PromptResponse::new(acp::StopReason::Cancelled))
                            } else {
                                Err(anyhow!(details))
                            }
                        }
                        Err(_) => Err(anyhow!(err)),
                    }
                }
            }
        })
    }

    fn cancel(&self, session_id: &acp::SessionId, _cx: &mut App) {
        if let Some(session) = self.sessions.borrow_mut().get_mut(session_id) {
            session.suppress_abort_err = true;
        }
        let params = acp::CancelNotification::new(session_id.clone());
        self.connection.send_notification(params).log_err();
    }

    fn session_modes(
        &self,
        session_id: &acp::SessionId,
        _cx: &App,
    ) -> Option<Rc<dyn acp_thread::AgentSessionModes>> {
        let sessions = self.sessions.clone();
        let sessions_ref = sessions.borrow();
        let Some(session) = sessions_ref.get(session_id) else {
            return None;
        };

        if let Some(modes) = session.session_modes.as_ref() {
            Some(Rc::new(AcpSessionModes {
                connection: self.connection.clone(),
                session_id: session_id.clone(),
                state: modes.clone(),
            }) as _)
        } else {
            None
        }
    }

    fn model_selector(
        &self,
        session_id: &acp::SessionId,
    ) -> Option<Rc<dyn acp_thread::AgentModelSelector>> {
        let sessions = self.sessions.clone();
        let sessions_ref = sessions.borrow();
        let Some(session) = sessions_ref.get(session_id) else {
            return None;
        };

        if let Some(models) = session.models.as_ref() {
            Some(Rc::new(AcpModelSelector::new(
                session_id.clone(),
                self.connection.clone(),
                models.clone(),
            )) as _)
        } else {
            None
        }
    }

    fn session_config_options(
        &self,
        session_id: &acp::SessionId,
        _cx: &App,
    ) -> Option<Rc<dyn acp_thread::AgentSessionConfigOptions>> {
        let sessions = self.sessions.borrow();
        let session = sessions.get(session_id)?;

        let config_opts = session.config_options.as_ref()?;

        Some(Rc::new(AcpSessionConfigOptions {
            session_id: session_id.clone(),
            connection: self.connection.clone(),
            state: config_opts.config_options.clone(),
            watch_tx: config_opts.tx.clone(),
            watch_rx: config_opts.rx.clone(),
        }) as _)
    }

    fn session_list(&self, _cx: &mut App) -> Option<Rc<dyn AgentSessionList>> {
        self.session_list.clone().map(|s| s as _)
    }

    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

fn map_acp_error(err: acp::Error) -> anyhow::Error {
    if err.code == acp::ErrorCode::AuthRequired {
        let mut error = AuthRequired::new();

        if err.message != acp::ErrorCode::AuthRequired.to_string() {
            error = error.with_description(err.message);
        }

        anyhow!(error)
    } else {
        anyhow!(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_auth_task_reuses_command_and_merges_args_and_env() {
        let command = AgentServerCommand {
            path: "/path/to/agent".into(),
            args: vec!["--acp".into(), "--verbose".into()],
            env: Some(HashMap::from_iter([
                ("BASE".into(), "1".into()),
                ("SHARED".into(), "base".into()),
            ])),
        };
        let method = acp::AuthMethodTerminal::new("login", "Login")
            .args(vec!["/auth".into()])
            .env(std::collections::HashMap::from_iter([
                ("EXTRA".into(), "2".into()),
                ("SHARED".into(), "override".into()),
            ]));

        let terminal_auth_task = terminal_auth_task(&command, &AgentId::new("test-agent"), &method);

        assert_eq!(
            terminal_auth_task.command.as_deref(),
            Some("/path/to/agent")
        );
        assert_eq!(terminal_auth_task.args, vec!["--acp", "--verbose", "/auth"]);
        assert_eq!(
            terminal_auth_task.env,
            HashMap::from_iter([
                ("BASE".into(), "1".into()),
                ("SHARED".into(), "override".into()),
                ("EXTRA".into(), "2".into()),
            ])
        );
        assert_eq!(terminal_auth_task.label, "Login");
        assert_eq!(terminal_auth_task.command_label, "Login");
    }

    #[test]
    fn legacy_terminal_auth_task_parses_meta_and_retries_session() {
        let method_id = acp::AuthMethodId::new("legacy-login");
        let method = acp::AuthMethod::Agent(
            acp::AuthMethodAgent::new(method_id.clone(), "Login").meta(acp::Meta::from_iter([(
                "terminal-auth".to_string(),
                serde_json::json!({
                    "label": "legacy /auth",
                    "command": "legacy-agent",
                    "args": ["auth", "--interactive"],
                    "env": {
                        "AUTH_MODE": "interactive",
                    },
                }),
            )])),
        );

        let terminal_auth_task =
            meta_terminal_auth_task(&AgentId::new("test-agent"), &method_id, &method)
                .expect("expected legacy terminal auth task");

        assert_eq!(
            terminal_auth_task.id.0,
            "external-agent-test-agent-legacy-login-login"
        );
        assert_eq!(terminal_auth_task.command.as_deref(), Some("legacy-agent"));
        assert_eq!(terminal_auth_task.args, vec!["auth", "--interactive"]);
        assert_eq!(
            terminal_auth_task.env,
            HashMap::from_iter([("AUTH_MODE".into(), "interactive".into())])
        );
        assert_eq!(terminal_auth_task.label, "legacy /auth");
    }

    #[test]
    fn legacy_terminal_auth_task_returns_none_for_invalid_meta() {
        let method_id = acp::AuthMethodId::new("legacy-login");
        let method = acp::AuthMethod::Agent(
            acp::AuthMethodAgent::new(method_id.clone(), "Login").meta(acp::Meta::from_iter([(
                "terminal-auth".to_string(),
                serde_json::json!({
                    "label": "legacy /auth",
                }),
            )])),
        );

        assert!(
            meta_terminal_auth_task(&AgentId::new("test-agent"), &method_id, &method).is_none()
        );
    }

    #[test]
    fn first_class_terminal_auth_takes_precedence_over_legacy_meta() {
        let method_id = acp::AuthMethodId::new("login");
        let method = acp::AuthMethod::Terminal(
            acp::AuthMethodTerminal::new(method_id, "Login")
                .args(vec!["/auth".into()])
                .env(std::collections::HashMap::from_iter([(
                    "AUTH_MODE".into(),
                    "first-class".into(),
                )]))
                .meta(acp::Meta::from_iter([(
                    "terminal-auth".to_string(),
                    serde_json::json!({
                        "label": "legacy /auth",
                        "command": "legacy-agent",
                        "args": ["legacy-auth"],
                        "env": {
                            "AUTH_MODE": "legacy",
                        },
                    }),
                )])),
        );

        let command = AgentServerCommand {
            path: "/path/to/agent".into(),
            args: vec!["--acp".into()],
            env: Some(HashMap::from_iter([("BASE".into(), "1".into())])),
        };

        let terminal_auth_task = match &method {
            acp::AuthMethod::Terminal(terminal) => {
                terminal_auth_task(&command, &AgentId::new("test-agent"), terminal)
            }
            _ => unreachable!(),
        };

        assert_eq!(
            terminal_auth_task.command.as_deref(),
            Some("/path/to/agent")
        );
        assert_eq!(terminal_auth_task.args, vec!["--acp", "/auth"]);
        assert_eq!(
            terminal_auth_task.env,
            HashMap::from_iter([
                ("BASE".into(), "1".into()),
                ("AUTH_MODE".into(), "first-class".into()),
            ])
        );
        assert_eq!(terminal_auth_task.label, "Login");
    }
}

fn mcp_servers_for_project(project: &Entity<Project>, cx: &App) -> Vec<acp::McpServer> {
    let context_server_store = project.read(cx).context_server_store().read(cx);
    let is_local = project.read(cx).is_local();
    context_server_store
        .configured_server_ids()
        .iter()
        .filter_map(|id| {
            let configuration = context_server_store.configuration_for_server(id)?;
            match &*configuration {
                project::context_server_store::ContextServerConfiguration::Custom {
                    command,
                    remote,
                    ..
                }
                | project::context_server_store::ContextServerConfiguration::Extension {
                    command,
                    remote,
                    ..
                } if is_local || *remote => Some(acp::McpServer::Stdio(
                    acp::McpServerStdio::new(id.0.to_string(), &command.path)
                        .args(command.args.clone())
                        .env(if let Some(env) = command.env.as_ref() {
                            env.iter()
                                .map(|(name, value)| acp::EnvVariable::new(name, value))
                                .collect()
                        } else {
                            vec![]
                        }),
                )),
                project::context_server_store::ContextServerConfiguration::Http {
                    url,
                    headers,
                    timeout: _,
                } => Some(acp::McpServer::Http(
                    acp::McpServerHttp::new(id.0.to_string(), url.to_string()).headers(
                        headers
                            .iter()
                            .map(|(name, value)| acp::HttpHeader::new(name, value))
                            .collect(),
                    ),
                )),
                _ => None,
            }
        })
        .collect()
}

fn config_state(
    modes: Option<acp::SessionModeState>,
    models: Option<acp::SessionModelState>,
    config_options: Option<Vec<acp::SessionConfigOption>>,
) -> (
    Option<Rc<RefCell<acp::SessionModeState>>>,
    Option<Rc<RefCell<acp::SessionModelState>>>,
    Option<Rc<RefCell<Vec<acp::SessionConfigOption>>>>,
) {
    if let Some(opts) = config_options {
        return (None, None, Some(Rc::new(RefCell::new(opts))));
    }

    let modes = modes.map(|modes| Rc::new(RefCell::new(modes)));
    let models = models.map(|models| Rc::new(RefCell::new(models)));
    (modes, models, None)
}

struct AcpSessionModes {
    session_id: acp::SessionId,
    connection: ConnectionTo<Agent>,
    state: Rc<RefCell<acp::SessionModeState>>,
}

impl acp_thread::AgentSessionModes for AcpSessionModes {
    fn current_mode(&self) -> acp::SessionModeId {
        self.state.borrow().current_mode_id.clone()
    }

    fn all_modes(&self) -> Vec<acp::SessionMode> {
        self.state.borrow().available_modes.clone()
    }

    fn set_mode(&self, mode_id: acp::SessionModeId, cx: &mut App) -> Task<Result<()>> {
        let connection = self.connection.clone();
        let session_id = self.session_id.clone();
        let old_mode_id;
        {
            let mut state = self.state.borrow_mut();
            old_mode_id = state.current_mode_id.clone();
            state.current_mode_id = mode_id.clone();
        };
        let state = self.state.clone();
        cx.foreground_executor().spawn(async move {
            let result = into_foreground_future(
                connection.send_request(acp::SetSessionModeRequest::new(session_id, mode_id)),
            )
            .await;

            if result.is_err() {
                state.borrow_mut().current_mode_id = old_mode_id;
            }

            result?;

            Ok(())
        })
    }
}

struct AcpModelSelector {
    session_id: acp::SessionId,
    connection: ConnectionTo<Agent>,
    state: Rc<RefCell<acp::SessionModelState>>,
}

impl AcpModelSelector {
    fn new(
        session_id: acp::SessionId,
        connection: ConnectionTo<Agent>,
        state: Rc<RefCell<acp::SessionModelState>>,
    ) -> Self {
        Self {
            session_id,
            connection,
            state,
        }
    }
}

impl acp_thread::AgentModelSelector for AcpModelSelector {
    fn list_models(&self, _cx: &mut App) -> Task<Result<acp_thread::AgentModelList>> {
        Task::ready(Ok(acp_thread::AgentModelList::Flat(
            self.state
                .borrow()
                .available_models
                .clone()
                .into_iter()
                .map(acp_thread::AgentModelInfo::from)
                .collect(),
        )))
    }

    fn select_model(&self, model_id: acp::ModelId, cx: &mut App) -> Task<Result<()>> {
        let connection = self.connection.clone();
        let session_id = self.session_id.clone();
        let old_model_id;
        {
            let mut state = self.state.borrow_mut();
            old_model_id = state.current_model_id.clone();
            state.current_model_id = model_id.clone();
        };
        let state = self.state.clone();
        cx.foreground_executor().spawn(async move {
            let result = into_foreground_future(
                connection.send_request(acp::SetSessionModelRequest::new(session_id, model_id)),
            )
            .await;

            if result.is_err() {
                state.borrow_mut().current_model_id = old_model_id;
            }

            result?;

            Ok(())
        })
    }

    fn selected_model(&self, _cx: &mut App) -> Task<Result<acp_thread::AgentModelInfo>> {
        let state = self.state.borrow();
        Task::ready(
            state
                .available_models
                .iter()
                .find(|m| m.model_id == state.current_model_id)
                .cloned()
                .map(acp_thread::AgentModelInfo::from)
                .ok_or_else(|| anyhow::anyhow!("Model not found")),
        )
    }
}

struct AcpSessionConfigOptions {
    session_id: acp::SessionId,
    connection: ConnectionTo<Agent>,
    state: Rc<RefCell<Vec<acp::SessionConfigOption>>>,
    watch_tx: Rc<RefCell<watch::Sender<()>>>,
    watch_rx: watch::Receiver<()>,
}

impl acp_thread::AgentSessionConfigOptions for AcpSessionConfigOptions {
    fn config_options(&self) -> Vec<acp::SessionConfigOption> {
        self.state.borrow().clone()
    }

    fn set_config_option(
        &self,
        config_id: acp::SessionConfigId,
        value: acp::SessionConfigValueId,
        cx: &mut App,
    ) -> Task<Result<Vec<acp::SessionConfigOption>>> {
        let connection = self.connection.clone();
        let session_id = self.session_id.clone();
        let state = self.state.clone();

        let watch_tx = self.watch_tx.clone();

        cx.foreground_executor().spawn(async move {
            let response = into_foreground_future(connection.send_request(
                acp::SetSessionConfigOptionRequest::new(session_id, config_id, value),
            ))
            .await?;

            *state.borrow_mut() = response.config_options.clone();
            watch_tx.borrow_mut().send(()).ok();
            Ok(response.config_options)
        })
    }

    fn watch(&self, _cx: &mut App) -> Option<watch::Receiver<()>> {
        Some(self.watch_rx.clone())
    }
}

// ---------------------------------------------------------------------------
// 백그라운드 핸들러 클로저에서 ForegroundWork 채널을 통해 포그라운드 스레드로
// 디스패치되는 핸들러 함수들.
// ---------------------------------------------------------------------------

fn session_thread(
    ctx: &ClientContext,
    session_id: &acp::SessionId,
) -> Result<WeakEntity<AcpThread>, acp::Error> {
    let sessions = ctx.sessions.borrow();
    sessions
        .get(session_id)
        .map(|session| session.thread.clone())
        .ok_or_else(|| acp::Error::internal_error().data(format!("unknown session: {session_id}")))
}

fn respond_err<T: JsonRpcResponse>(responder: Responder<T>, err: acp::Error) {
    // 실제 반환하는 에러를 로그에 남긴다. 남기지 않으면 에이전트 측에서는
    // 와이어로 내려가는 일반 internal error 만 보게 되어 원인 추적이 힘들다.
    log::warn!(
        "Responding to ACP request `{method}` with error: {err:?}",
        method = responder.method()
    );
    responder.respond_with_error(err).log_err();
}

fn handle_request_permission(
    args: acp::RequestPermissionRequest,
    responder: Responder<acp::RequestPermissionResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    cx.spawn(async move |cx| {
        let result: Result<_, acp::Error> = async {
            let task = thread
                .update(cx, |thread, cx| {
                    thread.request_tool_call_authorization(
                        args.tool_call,
                        acp_thread::PermissionOptions::Flat(args.options),
                        cx,
                    )
                })
                .flatten_acp()?;
            Ok(task.await)
        }
        .await;

        match result {
            Ok(outcome) => {
                responder
                    .respond(acp::RequestPermissionResponse::new(outcome.into()))
                    .log_err();
            }
            Err(e) => respond_err(responder, e),
        }
    })
    .detach();
}

fn handle_write_text_file(
    args: acp::WriteTextFileRequest,
    responder: Responder<acp::WriteTextFileResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    cx.spawn(async move |cx| {
        let result: Result<_, acp::Error> = async {
            thread
                .update(cx, |thread, cx| {
                    thread.write_text_file(args.path, args.content, cx)
                })
                .map_err(acp::Error::from)?
                .await?;
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                responder
                    .respond(acp::WriteTextFileResponse::default())
                    .log_err();
            }
            Err(e) => respond_err(responder, e),
        }
    })
    .detach();
}

fn handle_read_text_file(
    args: acp::ReadTextFileRequest,
    responder: Responder<acp::ReadTextFileResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    cx.spawn(async move |cx| {
        let result: Result<_, acp::Error> = async {
            thread
                .update(cx, |thread, cx| {
                    thread.read_text_file(args.path, args.line, args.limit, false, cx)
                })
                .map_err(acp::Error::from)?
                .await
        }
        .await;

        match result {
            Ok(content) => {
                responder
                    .respond(acp::ReadTextFileResponse::new(content))
                    .log_err();
            }
            Err(e) => respond_err(responder, e),
        }
    })
    .detach();
}

fn handle_session_notification(
    notification: acp::SessionNotification,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    // 세션을 잠깐 빌린 뒤 필요한 값들을 복제해 가져온다.
    let (thread, session_modes, config_opts_data) = {
        let sessions = ctx.sessions.borrow();
        let Some(session) = sessions.get(&notification.session_id) else {
            log::warn!(
                "Received session notification for unknown session: {:?}",
                notification.session_id
            );
            return;
        };
        (
            session.thread.clone(),
            session.session_modes.clone(),
            session
                .config_options
                .as_ref()
                .map(|opts| (opts.config_options.clone(), opts.tx.clone())),
        )
    };
    // 여기서 borrow 가 해제된다.

    // borrow 를 잡지 않은 상태에서 mode/config/session_list 업데이트를 적용한다.
    if let acp::SessionUpdate::CurrentModeUpdate(acp::CurrentModeUpdate {
        current_mode_id, ..
    }) = &notification.update
    {
        if let Some(session_modes) = &session_modes {
            session_modes.borrow_mut().current_mode_id = current_mode_id.clone();
        }
    }

    if let acp::SessionUpdate::ConfigOptionUpdate(acp::ConfigOptionUpdate {
        config_options, ..
    }) = &notification.update
    {
        if let Some((config_opts_cell, tx_cell)) = &config_opts_data {
            *config_opts_cell.borrow_mut() = config_options.clone();
            tx_cell.borrow_mut().send(()).ok();
        }
    }

    if let acp::SessionUpdate::SessionInfoUpdate(info_update) = &notification.update
        && let Some(session_list) = ctx.session_list.borrow().as_ref()
    {
        session_list.send_info_update(notification.session_id.clone(), info_update.clone());
    }

    // Pre-handle: ToolCall 가 terminal_info 를 실어오면 표시 전용 터미널을
    // 만들어 등록한다.
    if let acp::SessionUpdate::ToolCall(tc) = &notification.update {
        if let Some(meta) = &tc.meta {
            if let Some(terminal_info) = meta.get("terminal_info") {
                if let Some(id_str) = terminal_info.get("terminal_id").and_then(|v| v.as_str()) {
                    let terminal_id = acp::TerminalId::new(id_str);
                    let cwd = terminal_info
                        .get("cwd")
                        .and_then(|v| v.as_str().map(PathBuf::from));

                    thread
                        .update(cx, |thread, cx| {
                            let builder = TerminalBuilder::new_display_only(
                                CursorShape::default(),
                                AlternateScroll::On,
                                None,
                                0,
                                cx.background_executor(),
                                thread.project().read(cx).path_style(cx),
                            )?;
                            let lower = cx.new(|cx| builder.subscribe(cx));
                            thread.on_terminal_provider_event(
                                TerminalProviderEvent::Created {
                                    terminal_id,
                                    label: tc.title.clone(),
                                    cwd,
                                    output_byte_limit: None,
                                    terminal: lower,
                                },
                                cx,
                            );
                            anyhow::Ok(())
                        })
                        .log_err();
                }
            }
        }
    }

    // 평소처럼 업데이트를 acp_thread 로 전달한다.
    if let Err(err) = thread
        .update(cx, |thread, cx| {
            thread.handle_session_update(notification.update.clone(), cx)
        })
        .flatten_acp()
    {
        log::error!(
            "Failed to handle session update for {:?}: {err:?}",
            notification.session_id
        );
    }

    // Post-handle: ToolCallUpdate meta 에 출력/종료 정보가 있으면 스트리밍한다.
    if let acp::SessionUpdate::ToolCallUpdate(tcu) = &notification.update {
        if let Some(meta) = &tcu.meta {
            if let Some(term_out) = meta.get("terminal_output") {
                if let Some(id_str) = term_out.get("terminal_id").and_then(|v| v.as_str()) {
                    let terminal_id = acp::TerminalId::new(id_str);
                    if let Some(s) = term_out.get("data").and_then(|v| v.as_str()) {
                        let data = s.as_bytes().to_vec();
                        thread
                            .update(cx, |thread, cx| {
                                thread.on_terminal_provider_event(
                                    TerminalProviderEvent::Output { terminal_id, data },
                                    cx,
                                );
                            })
                            .log_err();
                    }
                }
            }

            // terminal_exit
            if let Some(term_exit) = meta.get("terminal_exit") {
                if let Some(id_str) = term_exit.get("terminal_id").and_then(|v| v.as_str()) {
                    let terminal_id = acp::TerminalId::new(id_str);
                    let status = acp::TerminalExitStatus::new()
                        .exit_code(
                            term_exit
                                .get("exit_code")
                                .and_then(|v| v.as_u64())
                                .map(|i| i as u32),
                        )
                        .signal(
                            term_exit
                                .get("signal")
                                .and_then(|v| v.as_str().map(|s| s.to_string())),
                        );

                    thread
                        .update(cx, |thread, cx| {
                            thread.on_terminal_provider_event(
                                TerminalProviderEvent::Exit {
                                    terminal_id,
                                    status,
                                },
                                cx,
                            );
                        })
                        .log_err();
                }
            }
        }
    }
}

fn handle_create_terminal(
    args: acp::CreateTerminalRequest,
    responder: Responder<acp::CreateTerminalResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };
    let project = match thread
        .read_with(cx, |thread, _cx| thread.project().clone())
        .map_err(acp::Error::from)
    {
        Ok(p) => p,
        Err(e) => return respond_err(responder, e),
    };

    cx.spawn(async move |cx| {
        let result: Result<_, acp::Error> = async {
            let terminal_entity = acp_thread::create_terminal_entity(
                args.command.clone(),
                &args.args,
                args.env
                    .into_iter()
                    .map(|env| (env.name, env.value))
                    .collect(),
                args.cwd.clone(),
                &project,
                cx,
            )
            .await?;

            let terminal_entity = thread.update(cx, |thread, cx| {
                thread.register_terminal_created(
                    acp::TerminalId::new(uuid::Uuid::new_v4().to_string()),
                    format!("{} {}", args.command, args.args.join(" ")),
                    args.cwd.clone(),
                    args.output_byte_limit,
                    terminal_entity,
                    cx,
                )
            })?;
            let terminal_id = terminal_entity.read_with(cx, |terminal, _| terminal.id().clone());
            Ok(terminal_id)
        }
        .await;

        match result {
            Ok(terminal_id) => {
                responder
                    .respond(acp::CreateTerminalResponse::new(terminal_id))
                    .log_err();
            }
            Err(e) => respond_err(responder, e),
        }
    })
    .detach();
}

fn handle_kill_terminal(
    args: acp::KillTerminalRequest,
    responder: Responder<acp::KillTerminalResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    match thread
        .update(cx, |thread, cx| thread.kill_terminal(args.terminal_id, cx))
        .flatten_acp()
    {
        Ok(()) => {
            responder
                .respond(acp::KillTerminalResponse::default())
                .log_err();
        }
        Err(e) => respond_err(responder, e),
    }
}

fn handle_release_terminal(
    args: acp::ReleaseTerminalRequest,
    responder: Responder<acp::ReleaseTerminalResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    match thread
        .update(cx, |thread, cx| {
            thread.release_terminal(args.terminal_id, cx)
        })
        .flatten_acp()
    {
        Ok(()) => {
            responder
                .respond(acp::ReleaseTerminalResponse::default())
                .log_err();
        }
        Err(e) => respond_err(responder, e),
    }
}

fn handle_terminal_output(
    args: acp::TerminalOutputRequest,
    responder: Responder<acp::TerminalOutputResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    match thread
        .read_with(cx, |thread, cx| -> anyhow::Result<_> {
            let out = thread
                .terminal(args.terminal_id)?
                .read(cx)
                .current_output(cx);
            Ok(out)
        })
        .flatten_acp()
    {
        Ok(output) => {
            responder.respond(output).log_err();
        }
        Err(e) => respond_err(responder, e),
    }
}

fn handle_wait_for_terminal_exit(
    args: acp::WaitForTerminalExitRequest,
    responder: Responder<acp::WaitForTerminalExitResponse>,
    cx: &mut AsyncApp,
    ctx: &ClientContext,
) {
    let thread = match session_thread(ctx, &args.session_id) {
        Ok(t) => t,
        Err(e) => return respond_err(responder, e),
    };

    cx.spawn(async move |cx| {
        let result: Result<_, acp::Error> = async {
            let exit_status = thread
                .update(cx, |thread, cx| {
                    anyhow::Ok(thread.terminal(args.terminal_id)?.read(cx).wait_for_exit())
                })
                .flatten_acp()?
                .await;
            Ok(exit_status)
        }
        .await;

        match result {
            Ok(exit_status) => {
                responder
                    .respond(acp::WaitForTerminalExitResponse::new(exit_status))
                    .log_err();
            }
            Err(e) => respond_err(responder, e),
        }
    })
    .detach();
}
