use crate::handle_open_request;
use crate::restore_or_create_workspace;
use agent_ui::ExternalSourcePrompt;
use anyhow::{Context as _, Result, anyhow};
use cli::{CliRequest, CliResponse, NotifyKind, SubagentPayload, ipc::IpcSender};
use cli::{IpcHandshake, ipc};
use client::parse_zed_link;
use db::kvp::KeyValueStore;
use editor::Editor;
use fs::Fs;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::{mpsc, oneshot};
use futures::future;

use futures::{FutureExt, SinkExt, StreamExt};
use git_ui::{file_diff_view::FileDiffView, multi_diff_view::MultiDiffView};
use gpui::{App, AppContext, AsyncApp, Entity, Global, WindowHandle};
use onboarding::FIRST_OPEN;
use onboarding::show_onboarding_view;
use recent_projects::{RemoteSettings, navigate_to_positions, open_remote_project};
use remote::{RemoteConnectionOptions, WslConnectionOptions};
use settings::Settings;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use ui::SharedString;
use util::ResultExt;
use util::paths::PathWithPosition;
use workspace::PathList;
use workspace::item::{Item, ItemHandle};
use workspace::Workspace;
use workspace::notifications::{NotificationId, simple_message_notification};
use workspace::{AppState, MultiWorkspace, OpenOptions, OpenResult, SerializedWorkspaceLocation};

#[derive(Default, Debug)]
pub struct OpenRequest {
    pub kind: Option<OpenRequestKind>,
    pub open_paths: Vec<String>,
    pub diff_paths: Vec<[String; 2]>,
    pub diff_all: bool,
    pub remote_connection: Option<RemoteConnectionOptions>,
}

#[derive(Debug)]
pub enum OpenRequestKind {
    CliConnection((mpsc::Receiver<CliRequest>, IpcSender<CliResponse>)),
    Extension {
        extension_id: String,
    },
    AgentPanel {
        external_source_prompt: Option<ExternalSourcePrompt>,
    },
    SharedAgentThread {
        session_id: String,
    },
    DockMenuAction {
        index: usize,
    },
    BuiltinJsonSchema {
        schema_path: String,
    },
    Setting {
        /// `None` opens settings without navigating to a specific path.
        setting_path: Option<String>,
    },
    GitClone {
        repo_url: SharedString,
    },
    GitCommit {
        sha: String,
    },
}

impl OpenRequest {
    pub fn parse(request: RawOpenRequest, cx: &App) -> Result<Self> {
        let mut this = Self::default();

        this.diff_paths = request.diff_paths;
        this.diff_all = request.diff_all;
        if let Some(wsl) = request.wsl {
            let (user, distro_name) = if let Some((user, distro)) = wsl.split_once('@') {
                if user.is_empty() {
                    anyhow::bail!("user is empty in wsl argument");
                }
                (Some(user.to_string()), distro.to_string())
            } else {
                (None, wsl)
            };
            this.remote_connection = Some(RemoteConnectionOptions::Wsl(WslConnectionOptions {
                distro_name,
                user,
            }));
        }

        for url in request.urls {
            if let Some(server_name) = url.strip_prefix("zed-cli://") {
                this.kind = Some(OpenRequestKind::CliConnection(connect_to_cli(server_name)?));
            } else if let Some(action_index) = url.strip_prefix("zed-dock-action://") {
                this.kind = Some(OpenRequestKind::DockMenuAction {
                    index: action_index.parse()?,
                });
            } else if let Some(file) = url.strip_prefix("file://") {
                this.parse_file_path(file)
            } else if let Some(file) = url.strip_prefix("zed://file") {
                this.parse_file_path(file)
            } else if let Some(file) = url.strip_prefix("zed://ssh") {
                let ssh_url = "ssh:/".to_string() + file;
                this.parse_ssh_file_path(&ssh_url, cx)?
            } else if let Some(extension_id) = url.strip_prefix("zed://extension/") {
                this.kind = Some(OpenRequestKind::Extension {
                    extension_id: extension_id.to_string(),
                });
            } else if let Some(session_id_str) = url.strip_prefix("zed://agent/shared/") {
                if uuid::Uuid::parse_str(session_id_str).is_ok() {
                    this.kind = Some(OpenRequestKind::SharedAgentThread {
                        session_id: session_id_str.to_string(),
                    });
                } else {
                    log::error!("Invalid session ID in URL: {}", session_id_str);
                }
            } else if let Some(agent_path) = url.strip_prefix("zed://agent") {
                this.parse_agent_url(agent_path)
            } else if let Some(schema_path) = url.strip_prefix("zed://schemas/") {
                this.kind = Some(OpenRequestKind::BuiltinJsonSchema {
                    schema_path: schema_path.to_string(),
                });
            } else if url == "zed://settings" || url == "zed://settings/" {
                this.kind = Some(OpenRequestKind::Setting { setting_path: None });
            } else if let Some(setting_path) = url.strip_prefix("zed://settings/") {
                this.kind = Some(OpenRequestKind::Setting {
                    setting_path: Some(setting_path.to_string()),
                });
            } else if let Some(clone_path) = url.strip_prefix("zed://git/clone") {
                this.parse_git_clone_url(clone_path)?
            } else if let Some(commit_path) = url.strip_prefix("zed://git/commit/") {
                this.parse_git_commit_url(commit_path)?
            } else if url.starts_with("ssh://") {
                this.parse_ssh_file_path(&url, cx)?
            } else if parse_zed_link(&url, cx).is_some() {
                // 채널 URL은 포크 환경에서 미지원 — 무시
            } else {
                log::error!("unhandled url: {}", url);
            }
        }

        Ok(this)
    }

    fn parse_file_path(&mut self, file: &str) {
        if let Some(decoded) = urlencoding::decode(file).log_err() {
            self.open_paths.push(decoded.into_owned())
        }
    }

    fn parse_agent_url(&mut self, agent_path: &str) {
        // Format: "" or "?prompt=<text>"
        let external_source_prompt = agent_path.strip_prefix('?').and_then(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .find_map(|(key, value)| (key == "prompt").then_some(value))
                .and_then(|prompt| ExternalSourcePrompt::new(prompt.as_ref()))
        });
        self.kind = Some(OpenRequestKind::AgentPanel {
            external_source_prompt,
        });
    }

    fn parse_git_clone_url(&mut self, clone_path: &str) -> Result<()> {
        // Format: /?repo=<url> or ?repo=<url>
        let clone_path = clone_path.strip_prefix('/').unwrap_or(clone_path);

        let query = clone_path
            .strip_prefix('?')
            .context("invalid git clone url: missing query string")?;

        let repo_url = url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(key, value)| (key == "repo").then_some(value))
            .filter(|s| !s.is_empty())
            .context("invalid git clone url: missing repo query parameter")?
            .to_string()
            .into();

        self.kind = Some(OpenRequestKind::GitClone { repo_url });

        Ok(())
    }

    fn parse_git_commit_url(&mut self, commit_path: &str) -> Result<()> {
        // Format: <sha>?repo=<path>
        let (sha, query) = commit_path
            .split_once('?')
            .context("invalid git commit url: missing query string")?;
        anyhow::ensure!(!sha.is_empty(), "invalid git commit url: missing sha");

        let repo = url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(key, value)| (key == "repo").then_some(value))
            .filter(|s| !s.is_empty())
            .context("invalid git commit url: missing repo query parameter")?
            .to_string();

        self.open_paths.push(repo);

        self.kind = Some(OpenRequestKind::GitCommit {
            sha: sha.to_string(),
        });

        Ok(())
    }

    fn parse_ssh_file_path(&mut self, file: &str, cx: &App) -> Result<()> {
        let url = url::Url::parse(file)?;
        let host = url
            .host()
            .with_context(|| format!("missing host in ssh url: {file}"))?
            .to_string();
        let username = Some(url.username().to_string()).filter(|s| !s.is_empty());
        let port = url.port();
        anyhow::ensure!(
            self.open_paths.is_empty(),
            "cannot open both local and ssh paths"
        );
        let mut connection_options =
            RemoteSettings::get_global(cx).connection_options_for(host, port, username);
        if let Some(password) = url.password() {
            connection_options.password = Some(password.to_string());
        }

        let connection_options = RemoteConnectionOptions::Ssh(connection_options);
        if let Some(ssh_connection) = &self.remote_connection {
            anyhow::ensure!(
                *ssh_connection == connection_options,
                "cannot open multiple different remote connections"
            );
        }
        self.remote_connection = Some(connection_options);
        self.parse_file_path(url.path());
        Ok(())
    }
}

#[derive(Clone)]
pub struct OpenListener(UnboundedSender<RawOpenRequest>);

#[derive(Default)]
pub struct RawOpenRequest {
    pub urls: Vec<String>,
    pub diff_paths: Vec<[String; 2]>,
    pub diff_all: bool,
    pub wsl: Option<String>,
}

impl Global for OpenListener {}

impl OpenListener {
    pub fn new() -> (Self, UnboundedReceiver<RawOpenRequest>) {
        let (tx, rx) = mpsc::unbounded();
        (OpenListener(tx), rx)
    }

    pub fn open(&self, request: RawOpenRequest) {
        self.0
            .unbounded_send(request)
            .context("no listener for open requests")
            .log_err();
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn listen_for_cli_connections(opener: OpenListener) -> Result<()> {
    use release_channel::RELEASE_CHANNEL_NAME;
    use std::os::unix::net::UnixDatagram;

    let sock_path = paths::data_dir().join(format!("zed-{}.sock", *RELEASE_CHANNEL_NAME));
    // remove the socket if the process listening on it has died
    if let Err(e) = UnixDatagram::unbound()?.connect(&sock_path)
        && e.kind() == std::io::ErrorKind::ConnectionRefused
    {
        std::fs::remove_file(&sock_path)?;
    }
    let listener = UnixDatagram::bind(&sock_path)?;
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(len) = listener.recv(&mut buf) {
            opener.open(RawOpenRequest {
                urls: vec![String::from_utf8_lossy(&buf[..len]).to_string()],
                ..Default::default()
            });
        }
    });
    Ok(())
}

fn connect_to_cli(
    server_name: &str,
) -> Result<(mpsc::Receiver<CliRequest>, IpcSender<CliResponse>)> {
    let handshake_tx = cli::ipc::IpcSender::<IpcHandshake>::connect(server_name.to_string())
        .context("error connecting to cli")?;
    let (request_tx, request_rx) = ipc::channel::<CliRequest>()?;
    let (response_tx, response_rx) = ipc::channel::<CliResponse>()?;

    handshake_tx
        .send(IpcHandshake {
            requests: request_tx,
            responses: response_rx,
        })
        .context("error sending ipc handshake")?;

    let (mut async_request_tx, async_request_rx) =
        futures::channel::mpsc::channel::<CliRequest>(16);
    thread::spawn(move || {
        while let Ok(cli_request) = request_rx.recv() {
            if smol::block_on(async_request_tx.send(cli_request)).is_err() {
                break;
            }
        }
        anyhow::Ok(())
    });

    Ok((async_request_rx, response_tx))
}

pub async fn open_paths_with_positions(
    path_positions: &[PathWithPosition],
    diff_paths: &[[String; 2]],
    diff_all: bool,
    app_state: Arc<AppState>,
    open_options: workspace::OpenOptions,
    cx: &mut AsyncApp,
) -> Result<(
    WindowHandle<MultiWorkspace>,
    Vec<Option<Result<Box<dyn ItemHandle>>>>,
)> {
    let paths = path_positions
        .iter()
        .map(|path_with_position| path_with_position.path.clone())
        .collect::<Vec<_>>();

    let OpenResult {
        window: multi_workspace,
        opened_items: mut items,
        ..
    } = cx
        .update(|cx| workspace::open_paths(&paths, app_state, open_options, cx))
        .await?;

    if diff_all && !diff_paths.is_empty() {
        if let Ok(diff_view) = multi_workspace.update(cx, |multi_workspace, window, cx| {
            multi_workspace.workspace().update(cx, |workspace, cx| {
                MultiDiffView::open(diff_paths.to_vec(), workspace, window, cx)
            })
        }) {
            if let Some(diff_view) = diff_view.await.log_err() {
                items.push(Some(Ok(Box::new(diff_view))));
            }
        }
    } else {
        let workspace_weak = multi_workspace.read_with(cx, |multi_workspace, _cx| {
            multi_workspace.workspace().downgrade()
        })?;
        for diff_pair in diff_paths {
            let old_path = Path::new(&diff_pair[0]).canonicalize()?;
            let new_path = Path::new(&diff_pair[1]).canonicalize()?;
            if let Ok(diff_view) = multi_workspace.update(cx, |_multi_workspace, window, cx| {
                FileDiffView::open(old_path, new_path, workspace_weak.clone(), window, cx)
            }) {
                if let Some(diff_view) = diff_view.await.log_err() {
                    items.push(Some(Ok(Box::new(diff_view))))
                }
            }
        }
    }

    for (item, path) in items.iter_mut().zip(&paths) {
        if let Some(Err(error)) = item {
            *error = anyhow!("error opening {path:?}: {error}");
        }
    }

    let items_for_navigation = items
        .iter()
        .map(|item| item.as_ref().and_then(|r| r.as_ref().ok()).cloned())
        .collect::<Vec<_>>();
    navigate_to_positions(&multi_workspace, items_for_navigation, path_positions, cx);

    Ok((multi_workspace, items))
}

pub async fn handle_cli_connection(
    (mut requests, responses): (mpsc::Receiver<CliRequest>, IpcSender<CliResponse>),
    app_state: Arc<AppState>,
    cx: &mut AsyncApp,
) {
    if let Some(request) = requests.next().await {
        match request {
            CliRequest::Open {
                urls,
                paths,
                diff_paths,
                diff_all,
                wait,
                wsl,
                open_new_workspace,
                reuse,
                env,
                user_data_dir: _,
            } => {
                if !urls.is_empty() {
                    cx.update(|cx| {
                        match OpenRequest::parse(
                            RawOpenRequest {
                                urls,
                                diff_paths,
                                diff_all,
                                wsl,
                            },
                            cx,
                        ) {
                            Ok(open_request) => {
                                handle_open_request(open_request, app_state.clone(), cx);
                                responses.send(CliResponse::Exit { status: 0 }).log_err();
                            }
                            Err(e) => {
                                responses
                                    .send(CliResponse::Stderr {
                                        message: format!("{e}"),
                                    })
                                    .log_err();
                                responses.send(CliResponse::Exit { status: 1 }).log_err();
                            }
                        };
                    });
                    return;
                }

                let open_workspace_result = open_workspaces(
                    paths,
                    diff_paths,
                    diff_all,
                    open_new_workspace,
                    reuse,
                    &responses,
                    wait,
                    app_state.clone(),
                    env,
                    cx,
                )
                .await;

                let status = if open_workspace_result.is_err() { 1 } else { 0 };
                responses.send(CliResponse::Exit { status }).log_err();
            }
            CliRequest::Notify {
                kind,
                cwd,
                pid,
                ancestors,
                notify_prompt,
                notify_response,
                notify_tool_name,
                notify_tool_preview,
                notify_idle_summary,
                subagent,
            } => {
                handle_notify_request(
                    NotifyRequestArgs {
                        kind,
                        cwd,
                        pid,
                        ancestors,
                        notify_prompt,
                        notify_response,
                        notify_tool_name,
                        notify_tool_preview,
                        notify_idle_summary,
                        subagent,
                    },
                    &responses,
                    cx,
                )
                .await;
            }
        }
    }
}

/// Claude Code 플러그인이 cli를 거쳐 전달한 작업 알림 IPC 원본 값을 묶은
/// 구조체. 필드는 `CliRequest::Notify` variant 와 1:1 대응한다.
struct NotifyRequestArgs {
    kind: NotifyKind,
    cwd: Option<String>,
    pid: Option<u32>,
    ancestors: Vec<u32>,
    notify_prompt: Option<String>,
    notify_response: Option<String>,
    notify_tool_name: Option<String>,
    notify_tool_preview: Option<String>,
    notify_idle_summary: Option<String>,
    /// Subagent 전용 payload. SubagentStart/Stop 때만 Some.
    subagent: Option<SubagentPayload>,
}

/// Claude Code 플러그인 → Dokkaebi 작업 알림 IPC 처리.
/// 설정:
/// - `claude_code.task_alert`(기본 true): false면 토스트/dot/배지 모두 차단
/// - `claude_code.task_alert_toast`(기본 true): false면 토스트만 생략하고
///   dot 인디케이터와 비활성 그룹 배지는 계속 표시
/// - `claude_code.subagent_view`(기본 true): false면 서브에이전트 이벤트 수신 시
///   탭 자동 생성 차단. 이미 열린 탭은 유지
async fn handle_notify_request(
    args: NotifyRequestArgs,
    responses: &IpcSender<CliResponse>,
    cx: &mut AsyncApp,
) {
    // Subagent 이벤트는 토스트 알림과 무관한 별도 경로(서브에이전트 뷰 탭).
    // 기존 task_alert/task_alert_toast 설정과 충돌하지 않도록 먼저 분기 처리.
    if matches!(
        args.kind,
        NotifyKind::SubagentStart | NotifyKind::SubagentStop
    ) {
        handle_subagent_request(args, responses, cx).await;
        return;
    }
    // 전역 on/off (`task_alert`)와 토스트 전용 on/off (`task_alert_toast`),
    // 토스트 auto-dismiss 시간(`toast_display_seconds`, 5~300 clamp, 기본 5)을
    // 한 번에 읽는다.
    let (task_alert_enabled, toast_enabled, toast_display_secs) = cx.update(|cx| {
        let settings = claude_subagent_view::claude_code_settings(cx);
        (
            settings.and_then(|n| n.task_alert).unwrap_or(true),
            settings.and_then(|n| n.task_alert_toast).unwrap_or(true),
            settings
                .and_then(|n| n.toast_display_seconds)
                .unwrap_or(5)
                .clamp(5, 300),
        )
    });

    if task_alert_enabled {
        let id_name: SharedString = match args.kind {
            NotifyKind::Stop => "claude_code.stop".into(),
            NotifyKind::Idle => "claude_code.idle".into(),
            NotifyKind::Permission => "claude_code.permission".into(),
            // Subagent* 는 handle_notify_request 상단 early-return 으로 도달 불가.
            NotifyKind::SubagentStart | NotifyKind::SubagentStop => {
                unreachable!("subagent 변이는 handle_subagent_request 로 분기됨")
            }
        };
        let id = NotificationId::named(id_name);

        // 제목/본문은 본체가 UI 언어에 맞춰 i18n 으로 생성한다. destructure 전에
        // `&args` 참조로 먼저 조립해야 compose 호출 이후에도 나머지 필드를 이동할 수 있다.
        let (display_title, display_body) = cx.update(|cx| {
            compose_claude_notification_text(&args, cx)
        });

        let NotifyRequestArgs {
            kind,
            cwd,
            pid,
            ancestors,
            ..
        } = args;

        // dot 인디케이터 + 비활성 그룹 배지 적용 및 매칭 터미널의 "그룹 / 탭"
        // 라벨 + 소속 (윈도우, 워크스페이스) 타겟 수집.
        // cli가 보낸 ancestors가 비어있으면 pid 기반 sysinfo 폴백.
        let target = mark_bell_for_notification(ancestors, pid, cwd.as_deref(), cx);

        // 토스트 팝업은 `task_alert_toast` 가 true 이고 발신 터미널이 속한
        // 타겟 워크스페이스가 식별된 경우에만 그 워크스페이스 하나에만 표시.
        // 매칭 실패 시 토스트를 생략해 발신과 무관한 윈도우에 팝업이 뜨는
        // 문제(동일 이름 탭을 가진 창 2개 환경)를 차단한다.
        if toast_enabled
            && let Some(target) = target
        {
            let NotifyTarget {
                location_label,
                window,
                workspace,
                ..
            } = target;

            // Stop 알림 수신 시 같은 워크스페이스의 Idle 토스트를 먼저 정리한다.
            // Idle 은 "1분 이상 입력 없음" 트리거이므로 작업 완료(Stop) 시점에는
            // 더 이상 유효하지 않고, 두 토스트가 동시에 쌓이는 것을 방지한다.
            if matches!(kind, NotifyKind::Stop) {
                let idle_id = NotificationId::named("claude_code.idle".into());
                let workspace_for_stop = workspace.clone();
                window
                    .update(cx, move |_, _, cx| {
                        workspace_for_stop.update(cx, |ws, cx| {
                            ws.dismiss_notification(&idle_id, cx);
                        });
                    })
                    .log_err();
            }

            // 매칭된 터미널의 "그룹 / 탭" 라벨을 본문 앞에 덧붙여 발신 위치를
            // 토스트에 표기. 라벨이 비면 원본 본문만 표시.
            let message_for_show = if location_label.is_empty() {
                display_body
            } else {
                format!("{}\n{}", location_label, display_body)
            };

            let id_for_show = id.clone();
            let title_for_show = display_title;
            let workspace_for_show = workspace.clone();
            let window_for_show = window;
            window_for_show
                .update(cx, move |_, _, cx| {
                    workspace_for_show.update(cx, move |ws, cx| {
                        ws.show_notification(id_for_show, cx, move |cx| {
                            let title_clone = title_for_show.clone();
                            let message_clone = message_for_show.clone();
                            cx.new(|cx| {
                                simple_message_notification::MessageNotification::new(
                                    message_clone,
                                    cx,
                                )
                                .with_title(title_clone)
                                .show_close_button(true)
                            })
                        });
                    });
                })
                .log_err();

            // Stop/Idle 토스트는 `toast_display_seconds` 경과 후 자동 dismiss.
            // 값은 설정에서 읽어 5~300 범위로 clamp 된 `toast_display_secs` 를 사용.
            // Permission 은 승인 응답이 필요하므로 자동 dismiss 하지 않고
            // 사용자가 직접 닫을 때까지 유지.
            if matches!(kind, NotifyKind::Stop | NotifyKind::Idle) {
                let id_for_dismiss = id.clone();
                let window_for_dismiss = window;
                let workspace_for_dismiss = workspace;
                let dismiss_after = Duration::from_secs(toast_display_secs as u64);
                cx.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(dismiss_after)
                        .await;
                    let _ = cx.update(|cx| {
                        window_for_dismiss
                            .update(cx, move |_, _, cx| {
                                workspace_for_dismiss.update(cx, |ws, cx| {
                                    ws.dismiss_notification(&id_for_dismiss, cx);
                                });
                            })
                            .log_err();
                    });
                })
                .detach();
            }
        }
    }

    responses.send(CliResponse::Exit { status: 0 }).log_err();
}

/// dispatch.sh 의 `jq @tsv` 이스케이프(`\n`/`\t`/`\r`/`\\`) 를 원문으로 복원한다.
/// @tsv 는 실제 개행을 `\n` 리터럴(2글자 `\` + `n`) 로 바꿔 한 행에 담도록 설계돼 있어,
/// bash read 로 받은 문자열을 그대로 cli → 본체로 전달하면 서브에이전트 뷰에
/// 백슬래시-n 이 보인다. 복원 순서 주의 — 백슬래시 자체는 마커로 잠시 빼둔 뒤 마지막에
/// 되돌려야 원문에 들어있던 literal `\n` 두 글자가 개행으로 오역되지 않는다.
fn unescape_tsv_value(s: &str) -> String {
    // Private-Use Area(U+E000) — 실사용 문자열에 등장할 확률 0. 임시 마커로 사용.
    const BACKSLASH_MARKER: &str = "\u{E000}";
    s.replace("\\\\", BACKSLASH_MARKER)
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace(BACKSLASH_MARKER, "\\")
}

/// Claude Code 서브에이전트 시작/종료 IPC 전용 처리.
/// - Start: `ClaudeSubagentStore::upsert_start` + 대상 워크스페이스에서 탭 오픈
/// - Stop:  `ClaudeSubagentStore::mark_stopped`. 탭은 사용자가 닫을 때까지 유지
/// 설정 `claude_code.subagent_view` 가 false 면 탭 자동 생성만 차단(상태는 기록).
async fn handle_subagent_request(
    args: NotifyRequestArgs,
    responses: &IpcSender<CliResponse>,
    cx: &mut AsyncApp,
) {
    use claude_subagent_view::{
        SubagentPanelPosition, contains as store_contains, mark_stopped, open_subagent_view,
        upsert_start,
    };

    // 진단용 트레이스. 필요 시 `RUST_LOG=Dokkaebi=debug` 로 활성화.
    log::debug!(
        "[subagent-diag] handle_subagent_request 진입 kind={:?} has_payload={} cwd={:?} pid={:?} ancestors_len={}",
        args.kind,
        args.subagent.is_some(),
        args.cwd,
        args.pid,
        args.ancestors.len()
    );

    let Some(payload) = args.subagent.as_ref() else {
        // SubagentStart/Stop 인데 payload 가 없는 것은 IPC 오용 (구 cli 와이어 포맷 등) → 조용히 종료.
        log::debug!("[subagent-diag] subagent payload 누락 — IPC 와이어 불일치 가능(cli 재빌드 필요?)");
        responses.send(CliResponse::Exit { status: 0 }).log_err();
        return;
    };
    let Some(subagent_id) = payload.subagent_id.clone().filter(|s| !s.is_empty()) else {
        // id 생성 실패(dispatch.sh @tsv 파싱 실패 등). 서브에이전트 매칭 불가 → 조용히 종료.
        log::debug!("[subagent-diag] subagent 이벤트에 id 누락. 무시.");
        responses.send(CliResponse::Exit { status: 0 }).log_err();
        return;
    };

    // 설정값 읽기 — subagent_view 토글 + 패널 위치.
    let (auto_open, panel_position) = cx.update(|cx| {
        let settings = claude_subagent_view::claude_code_settings(cx);
        let auto_open = settings.and_then(|n| n.subagent_view).unwrap_or(false);
        let position = settings
            .and_then(|n| n.subagent_panel_position)
            .map(|p| match p {
                settings::SubagentPanelPositionContent::Right => SubagentPanelPosition::Right,
                settings::SubagentPanelPositionContent::Bottom => SubagentPanelPosition::Bottom,
            })
            .unwrap_or_default();
        (auto_open, position)
    });
    log::debug!(
        "[subagent-diag] settings auto_open={} panel_position={:?}",
        auto_open,
        panel_position
    );

    match args.kind {
        NotifyKind::SubagentStart => {
            let session_id = payload.session_id.clone();
            let subagent_type = payload.subagent_type.clone().unwrap_or_default();
            // dispatch.sh 의 `jq @tsv` 는 실제 개행을 `\n` 리터럴로 이스케이프해 전달한다.
            // 본체 렌더 전에 복원해야 뷰에 백슬래시-n 이 그대로 보이지 않는다.
            let description = payload
                .description
                .as_deref()
                .map(unescape_tsv_value)
                .unwrap_or_default();
            let prompt = payload
                .prompt
                .as_deref()
                .map(unescape_tsv_value)
                .unwrap_or_default();
            let transcript_path_opt = payload.transcript_path.clone();
            let cwd = args.cwd.clone();
            let pid = args.pid;

            // 재진입(동일 subagent Start IPC 재수신) 시 tail task 를 중복 spawn 하지 않도록
            // upsert 전에 기존 엔트리 존재 여부를 확인한다. 기존 엔트리가 있으면 이전 Start 때
            // 이미 tail 이 돌고 있으므로 두 번째 spawn 을 건너뛴다.
            let is_reentry = cx.update(|cx| store_contains(cx, &subagent_id));
            let tail_path = transcript_path_opt
                .clone()
                .filter(|p| !is_reentry && !p.is_empty());
            // id 는 이후 tail/open 호출에서 필요한 만큼만 clone. upsert_start 로 원본을 소비.
            let id_for_tail = tail_path.as_ref().map(|_| subagent_id.clone());
            let id_for_open = auto_open.then(|| subagent_id.clone());
            let _ = cx.update(|cx| {
                upsert_start(
                    cx,
                    subagent_id,
                    session_id,
                    subagent_type,
                    description,
                    prompt,
                    transcript_path_opt,
                    cwd,
                    pid,
                );
            });

            // transcript tail 백그라운드 task 시작 — 서브에이전트 완료 + grace 후 자동 종료.
            // 재진입이면 동일 id 의 tail 이 이미 동작 중이므로 spawn 생략.
            if let Some(id) = id_for_tail
                && let Some(path_str) = tail_path
            {
                let path = std::path::PathBuf::from(path_str);
                crate::zed::claude_subagent_tail::spawn_transcript_tail(id, path, cx);
            }

            if let Some(id_for_open) = id_for_open {
                // 탭 오픈 타겟 워크스페이스는 기존 ancestor/PID 매칭으로 식별한 것과
                // 동일 워크스페이스(= 발신 터미널이 속한 MultiWorkspace 의 active).
                // mark_bell_for_notification 은 내부에서 cx.update 를 사용하므로
                // AsyncApp 을 그대로 넘긴다. 다만 이 호출은 dot/배지도 함께 찍으므로
                // 서브에이전트만 추적하는 가벼운 타겟 탐색이 아닌 기존 notify 흐름과
                // 동일한 동작이라는 점을 유의(중복 호출 없이 Start 시점 1회만 수행).
                let ancestors_for_match = args.ancestors.clone();
                let cwd_for_match = args.cwd.clone();
                let pid_for_match = args.pid;
                let target = mark_bell_for_notification(
                    ancestors_for_match,
                    pid_for_match,
                    cwd_for_match.as_deref(),
                    cx,
                );
                log::debug!(
                    "[subagent-diag] mark_bell_for_notification target_found={}",
                    target.is_some()
                );
                if let Some(NotifyTarget { window, workspace, group_idx, .. }) = target {
                    let _ = cx.update(|cx| {
                        window
                            .update(cx, move |_, window, cx| {
                                workspace.update(cx, move |ws, cx| {
                                    open_subagent_view(
                                        id_for_open,
                                        panel_position,
                                        group_idx,
                                        ws,
                                        window,
                                        cx,
                                    );
                                });
                            })
                            .log_err();
                    });
                }
            } else {
                log::debug!(
                    "[subagent-diag] auto_open=false 또는 id 누락 — 탭 생성 건너뜀 (auto_open={})",
                    auto_open
                );
            }
        }
        NotifyKind::SubagentStop => {
            let id_for_mark = subagent_id;
            // dispatch.sh `jq @tsv` 이스케이프 복원 — 결과 텍스트의 개행이 리터럴로 노출되지 않도록.
            let result = payload.result.as_deref().map(unescape_tsv_value);
            let _ = cx.update(|cx| {
                mark_stopped(cx, id_for_mark, result);
            });
        }
        _ => {
            // early-return 이므로 여기는 도달하지 않음.
        }
    }

    responses.send(CliResponse::Exit { status: 0 }).log_err();
}

/// Claude Code 알림 토스트의 제목/본문을 현재 UI 언어에 맞춰 i18n 으로 조립한다.
/// 동적 필드가 비었으면 종류별 `default_body` 키로 폴백.
fn compose_claude_notification_text(
    args: &NotifyRequestArgs,
    cx: &App,
) -> (String, String) {
    let prompt = args.notify_prompt.as_deref();
    let response = args.notify_response.as_deref();
    let tool_name = args.notify_tool_name.as_deref();
    let tool_preview = args.notify_tool_preview.as_deref();
    let idle_summary = args.notify_idle_summary.as_deref();

    match args.kind {
        NotifyKind::Stop => {
            let title = i18n::t("claude_code.notify.stop.title", cx).to_string();
            let body = match (prompt, response) {
                (Some(q), Some(r)) if !q.is_empty() && !r.is_empty() => format!("{} → {}", q, r),
                (Some(q), _) if !q.is_empty() => q.to_string(),
                (_, Some(r)) if !r.is_empty() => r.to_string(),
                _ => i18n::t("claude_code.notify.stop.default_body", cx).to_string(),
            };
            (title, body)
        }
        NotifyKind::Idle => {
            let title = i18n::t("claude_code.notify.idle.title", cx).to_string();
            let body = match idle_summary {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => i18n::t("claude_code.notify.idle.default_body", cx).to_string(),
            };
            (title, body)
        }
        NotifyKind::Permission => {
            let title = i18n::t("claude_code.notify.permission.title", cx).to_string();
            let label = i18n::t("claude_code.notify.permission.label", cx);
            let body = match (tool_name, tool_preview) {
                (Some(tool), Some(preview)) if !tool.is_empty() && !preview.is_empty() => {
                    format!("{}: {} ({})", label, tool, preview)
                }
                (Some(tool), _) if !tool.is_empty() => format!("{}: {}", label, tool),
                _ => i18n::t("claude_code.notify.permission.default_body", cx).to_string(),
            };
            (title, body)
        }
        // Subagent 이벤트는 호출처에서 별도 분기로 처리하므로 여기 도달하지 않는다.
        NotifyKind::SubagentStart | NotifyKind::SubagentStop => {
            unreachable!("subagent 변이는 handle_subagent_request 로 분기됨")
        }
    }
}

/// 프로세스 parent/children 관계 snapshot. 한 번의 `capture()` 결과로 ancestor
/// chain 추적(`ancestors_of`)과 descendants BFS(`descendants_of`)를 모두 수행할
/// 수 있어, 알림 1건에서 sysinfo 전체 리프레시를 두 번 돌리던 기존 코드의
/// 중복 O(N) 비용(N=시스템 프로세스 수)을 단일 스캔으로 축소한다.
/// - Windows: Toolhelp snapshot 1회 (parent 정보만 필요하므로 sysinfo보다 가벼움)
/// - 기타 OS: sysinfo fallback (상류 호환 목적, 실사용 대상 아님)
struct ProcessSnapshot {
    /// PID → parent PID 매핑. 루트 프로세스는 map에서 누락되거나 0으로 기록.
    parent_of: std::collections::HashMap<u32, u32>,
    /// children_of 는 `descendants_of` 호출 시 최초 1회 lazy 빌드.
    children_of: std::cell::OnceCell<std::collections::HashMap<u32, Vec<u32>>>,
}

impl ProcessSnapshot {
    #[cfg(target_os = "windows")]
    fn capture() -> Self {
        use std::collections::HashMap;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        };

        let mut parent_of: HashMap<u32, u32> = HashMap::new();
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return Self {
                    parent_of,
                    children_of: std::cell::OnceCell::new(),
                };
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            if Process32FirstW(snap, &mut entry).is_ok() {
                loop {
                    parent_of.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                    if Process32NextW(snap, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }
        Self {
            parent_of,
            children_of: std::cell::OnceCell::new(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn capture() -> Self {
        use std::collections::HashMap;
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

        let mut sys = System::new();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );
        let mut parent_of: HashMap<u32, u32> = HashMap::new();
        for (pid, proc) in sys.processes() {
            if let Some(parent) = proc.parent() {
                parent_of.insert(pid.as_u32(), parent.as_u32());
            }
        }
        Self {
            parent_of,
            children_of: std::cell::OnceCell::new(),
        }
    }

    /// `start` 자신부터 parent chain을 따라 PID 벡터를 수집한다.
    /// 시스템 루트 도달, cycle 감지, 최대 깊이(64) 중 하나가 맞으면 중단.
    fn ancestors_of(&self, start: u32) -> Vec<u32> {
        let mut chain = Vec::with_capacity(8);
        let mut current = start;
        let mut guard = 0usize;
        loop {
            if guard > 64 {
                break;
            }
            guard += 1;
            chain.push(current);
            let Some(&parent) = self.parent_of.get(&current) else {
                break;
            };
            if parent == 0 || parent == current || chain.contains(&parent) {
                break;
            }
            current = parent;
        }
        chain
    }

    fn children_map(&self) -> &std::collections::HashMap<u32, Vec<u32>> {
        self.children_of.get_or_init(|| {
            let mut children: std::collections::HashMap<u32, Vec<u32>> =
                std::collections::HashMap::new();
            for (&child, &parent) in &self.parent_of {
                children.entry(parent).or_default().push(child);
            }
            children
        })
    }

    /// `root` 로부터 BFS 로 descendants 집합 수집. 최대 방문 1024 로 cycle 가드.
    fn descendants_of(&self, root: u32) -> std::collections::HashSet<u32> {
        let children = self.children_map();
        let mut result = std::collections::HashSet::new();
        let mut queue = vec![root];
        let mut guard = 0usize;
        while let Some(p) = queue.pop() {
            if guard > 1024 {
                break;
            }
            guard += 1;
            if !result.insert(p) {
                continue;
            }
            if let Some(kids) = children.get(&p) {
                queue.extend(kids.iter().copied());
            }
        }
        result
    }
}

/// 발신 터미널 매칭 결과. 토스트를 발신 터미널이 속한 (윈도우, 워크스페이스)
/// 한 쌍에만 표시하기 위해 `mark_bell_for_notification` 이 반환한다.
/// 매칭 실패 시 반환값은 `None` 이고, 이 경우 토스트/dot/그룹 배지 모두
/// 표시되지 않는다.
struct NotifyTarget {
    /// 토스트 본문 앞에 붙일 "그룹 이름 / 탭 이름" 라벨. 그룹명이 비면 탭명만.
    location_label: String,
    /// 발신 터미널이 속한 MultiWorkspace 윈도우 핸들.
    window: WindowHandle<MultiWorkspace>,
    /// 발신 터미널이 속한 Workspace 엔티티.
    workspace: Entity<Workspace>,
    /// 발신 터미널이 속한 워크스페이스 그룹 인덱스.
    /// 서브에이전트 탭을 활성 그룹이 아니라 발신 터미널이 속한 그룹에 부착하기
    /// 위해 사용한다. 토스트 알림 경로는 이 값을 사용하지 않는다.
    group_idx: usize,
}

/// Claude Code 작업 알림으로 터미널 dot 인디케이터 + 비활성 그룹 배지를 설정한다.
///
/// 매칭은 양방향 PID 검사를 모두 수행한다:
///   1) 터미널 shell_pid가 cli의 ancestor chain에 포함(위 방향)
///   2) 터미널 shell의 descendants 트리에 ancestor chain의 어떤 PID가 포함(아래 방향)
///
/// 둘 중 어느 쪽이든 일치하면 해당 터미널만 dot + 그룹 배지를 받는다. cwd 기반
/// fallback은 동명 탭이 여러 개일 때 과다 매칭을 유발하므로 제거했다. 매칭
/// 실패 시 토스트/dot/배지 모두 표시되지 않는다.
/// 반환값은 매칭된 첫 터미널의 라벨과 소속 (윈도우, 워크스페이스) 타겟을 담은
/// `NotifyTarget` 이다. 호출자는 이 타겟을 이용해 발신과 무관한 다른 윈도우에
/// 토스트가 브로드캐스트되지 않도록 해당 워크스페이스 1곳에만 알림을 띄운다.
fn mark_bell_for_notification(
    ancestors: Vec<u32>,
    pid: Option<u32>,
    _cwd_str: Option<&str>,
    cx: &mut AsyncApp,
) -> Option<NotifyTarget> {
    use std::collections::HashSet;
    use terminal_view::TerminalView;

    // 프로세스 관계 snapshot 1회 캡처. ancestor fallback 과 descendants BFS 를
    // 동일 snapshot 위에서 수행해 기존의 sysinfo 이중 리프레시 중복을 제거한다.
    let snapshot = ProcessSnapshot::capture();

    // cli가 보낸 ancestors가 비어있지 않으면 그대로 사용 (bash exit 후에도 유효).
    // 비어있으면 구 cli 호환을 위해 본체 snapshot 으로 재수집.
    let ancestor_pids: Vec<u32> = if !ancestors.is_empty() {
        ancestors
    } else {
        match pid {
            Some(p) => snapshot.ancestors_of(p),
            None => Vec::new(),
        }
    };

    if ancestor_pids.is_empty() {
        return None;
    }

    let ancestor_set: HashSet<u32> = ancestor_pids.iter().copied().collect();

    let by_pid = |tv: &gpui::Entity<TerminalView>, cx: &App| -> bool {
        let Some(s) = tv.read(cx).entity().read(cx).shell_pid() else {
            return false;
        };
        // 1) shell_pid가 ancestors에 포함(cli로부터 위로 올라간 chain에 shell이 있음).
        if ancestor_set.contains(&s) {
            return true;
        }
        // 2) shell의 descendants 트리에 ancestors 원소 포함(shell로부터 아래로
        //    내려간 자식 트리에 cli 혹은 중간 프로세스가 있음). Toolhelp chain이
        //    중간에서 끊기는 경우까지 커버한다.
        let desc = snapshot.descendants_of(s);
        ancestor_set.iter().any(|a| desc.contains(a))
    };

    // 매칭 터미널의 (엔티티, 그룹 인덱스) 를 수집하고, 첫 매칭 터미널의 라벨 +
    // 소속 (윈도우, 워크스페이스) 를 타겟으로 캡처해 호출자가 토스트를 해당
    // 워크스페이스 한 곳에만 띄울 수 있게 한다.
    let mut target: Option<NotifyTarget> = None;

    cx.update(|cx| {
        for window in cx.windows() {
            let Some(multi_handle) = window.downcast::<MultiWorkspace>() else {
                continue;
            };
            // nested closure 에서 workspace 캡처 시 move 되지 않도록 outer 에서
            // 클론해둔다. WindowHandle 은 가벼운 ID 기반이라 비용 무시.
            let window_for_target = multi_handle;
            multi_handle
                .update(cx, |multi_ws, _window, cx| {
                    let workspaces: Vec<_> = multi_ws.workspaces().to_vec();
                    for workspace_entity in workspaces {
                        let workspace_for_target = workspace_entity.clone();
                        workspace_entity.update(cx, |workspace, cx| {
                            let active_index = workspace.active_group_index();
                            // (TerminalView, 그룹 인덱스) 쌍으로 수집
                            let mut matched: Vec<(gpui::Entity<TerminalView>, usize)> =
                                workspace
                                    .items_of_type::<TerminalView>(cx)
                                    .filter(|tv| by_pid(tv, cx))
                                    .map(|tv| (tv, active_index))
                                    .collect();
                            {
                                let groups = workspace.workspace_groups();
                                for (i, group) in groups.iter().enumerate() {
                                    if i == active_index {
                                        continue;
                                    }
                                    for pane in &group.panes {
                                        for tv in
                                            pane.read(cx).items_of_type::<TerminalView>()
                                        {
                                            if by_pid(&tv, cx) {
                                                matched.push((tv, i));
                                            }
                                        }
                                    }
                                }
                            }

                            // 첫 매칭에서 "그룹 / 탭" 라벨 + 소속 윈도우/워크스페이스
                            // + 발신 터미널이 속한 그룹 인덱스를 기록. 이후 매칭들은
                            // bell/배지만 적용한다. group_idx 는 서브에이전트 탭이
                            // 활성 그룹이 아니라 발신 그룹에 정확히 부착되도록 하는데
                            // 사용된다.
                            if target.is_none()
                                && let Some((tv, group_idx)) = matched.first()
                            {
                                let tab_name =
                                    tv.read(cx).tab_content_text(0, cx).to_string();
                                let group_name = workspace
                                    .workspace_groups()
                                    .get(*group_idx)
                                    .map(|g| g.name.clone())
                                    .unwrap_or_default();
                                let location_label = if group_name.is_empty() {
                                    tab_name
                                } else {
                                    format!("{} / {}", group_name, tab_name)
                                };
                                target = Some(NotifyTarget {
                                    location_label,
                                    window: window_for_target,
                                    workspace: workspace_for_target,
                                    group_idx: *group_idx,
                                });
                            }

                            for (tv, _) in matched {
                                let item_id = tv.entity_id();
                                tv.update(cx, |terminal_view, cx| {
                                    terminal_view.set_has_bell(cx);
                                });
                                workspace.notify_bell_for_item(item_id, cx);
                            }
                        });
                    }
                })
                .log_err();
        }
    });

    target
}

async fn open_workspaces(
    paths: Vec<String>,
    diff_paths: Vec<[String; 2]>,
    diff_all: bool,
    open_new_workspace: Option<bool>,
    reuse: bool,
    responses: &IpcSender<CliResponse>,
    wait: bool,
    app_state: Arc<AppState>,
    env: Option<collections::HashMap<String, String>>,
    cx: &mut AsyncApp,
) -> Result<()> {
    if paths.is_empty() && diff_paths.is_empty() && open_new_workspace != Some(true) {
        return restore_or_create_workspace(app_state, cx).await;
    }

    let grouped_locations: Vec<(SerializedWorkspaceLocation, PathList)> =
        if paths.is_empty() && diff_paths.is_empty() {
            Vec::new()
        } else {
            vec![(
                SerializedWorkspaceLocation::Local,
                PathList::new(&paths.into_iter().map(PathBuf::from).collect::<Vec<_>>()),
            )]
        };

    if grouped_locations.is_empty() {
        // If we have no paths to open, show the welcome screen if this is the first launch
        let kvp = cx.update(|cx| KeyValueStore::global(cx));
        if matches!(kvp.read_kvp(FIRST_OPEN), Ok(None)) {
            cx.update(|cx| show_onboarding_view(app_state, cx).detach());
        }
        // If not the first launch, show an empty window with empty editor
        else {
            cx.update(|cx| {
                let open_options = OpenOptions {
                    env,
                    ..Default::default()
                };
                workspace::open_new(open_options, app_state, cx, |workspace, window, cx| {
                    Editor::new_file(workspace, &Default::default(), window, cx)
                })
                .detach_and_log_err(cx);
            });
        }
        return Ok(());
    }
    // If there are paths to open, open a workspace for each grouping of paths
    let mut errored = false;

    for (location, workspace_paths) in grouped_locations {
        // If reuse flag is passed, open a new workspace in an existing window.
        let (open_new_workspace, replace_window) = if reuse {
            (
                Some(true),
                cx.update(|cx| {
                    workspace::workspace_windows_for_location(&location, cx)
                        .into_iter()
                        .next()
                }),
            )
        } else {
            (open_new_workspace, None)
        };
        let open_options = workspace::OpenOptions {
            open_new_workspace,
            replace_window,
            wait,
            env: env.clone(),
            ..Default::default()
        };

        match location {
            SerializedWorkspaceLocation::Local => {
                let workspace_paths = workspace_paths
                    .paths()
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();

                let workspace_failed_to_open = open_local_workspace(
                    workspace_paths,
                    diff_paths.clone(),
                    diff_all,
                    open_options,
                    responses,
                    &app_state,
                    cx,
                )
                .await;

                if workspace_failed_to_open {
                    errored = true
                }
            }
            SerializedWorkspaceLocation::Remote(mut connection) => {
                let app_state = app_state.clone();
                if let RemoteConnectionOptions::Ssh(options) = &mut connection {
                    cx.update(|cx| {
                        RemoteSettings::get_global(cx)
                            .fill_connection_options_from_settings(options)
                    });
                }
                cx.spawn(async move |cx| {
                    open_remote_project(
                        connection,
                        workspace_paths.paths().to_vec(),
                        app_state,
                        open_options,
                        cx,
                    )
                    .await
                    .log_err();
                })
                .detach();
            }
        }
    }

    anyhow::ensure!(!errored, "failed to open a workspace");

    Ok(())
}

async fn open_local_workspace(
    workspace_paths: Vec<String>,
    diff_paths: Vec<[String; 2]>,
    diff_all: bool,
    open_options: workspace::OpenOptions,
    responses: &IpcSender<CliResponse>,
    app_state: &Arc<AppState>,
    cx: &mut AsyncApp,
) -> bool {
    let paths_with_position =
        derive_paths_with_position(app_state.fs.as_ref(), workspace_paths).await;

    let (workspace, items) = match open_paths_with_positions(
        &paths_with_position,
        &diff_paths,
        diff_all,
        app_state.clone(),
        open_options.clone(),
        cx,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            responses
                .send(CliResponse::Stderr {
                    message: format!("error opening {paths_with_position:?}: {error}"),
                })
                .log_err();
            return true;
        }
    };

    let mut errored = false;
    let mut item_release_futures = Vec::new();
    let mut subscriptions = Vec::new();
    // If --wait flag is used with no paths, or a directory, then wait until
    // the entire workspace is closed.
    if open_options.wait {
        let mut wait_for_window_close = paths_with_position.is_empty() && diff_paths.is_empty();
        for path_with_position in &paths_with_position {
            if app_state.fs.is_dir(&path_with_position.path).await {
                wait_for_window_close = true;
                break;
            }
        }

        if wait_for_window_close {
            let (release_tx, release_rx) = oneshot::channel();
            item_release_futures.push(release_rx);
            subscriptions.push(workspace.update(cx, |_, _, cx| {
                cx.on_release(move |_, _| {
                    let _ = release_tx.send(());
                })
            }));
        }
    }

    for item in items {
        match item {
            Some(Ok(item)) => {
                if open_options.wait {
                    let (release_tx, release_rx) = oneshot::channel();
                    item_release_futures.push(release_rx);
                    subscriptions.push(Ok(cx.update(|cx| {
                        item.on_release(
                            cx,
                            Box::new(move |_| {
                                release_tx.send(()).ok();
                            }),
                        )
                    })));
                }
            }
            Some(Err(err)) => {
                responses
                    .send(CliResponse::Stderr {
                        message: err.to_string(),
                    })
                    .log_err();
                errored = true;
            }
            None => {}
        }
    }

    if open_options.wait {
        let wait = async move {
            let _subscriptions = subscriptions;
            let _ = future::try_join_all(item_release_futures).await;
        }
        .fuse();
        futures::pin_mut!(wait);

        let background = cx.background_executor().clone();
        loop {
            // Repeatedly check if CLI is still open to avoid wasting resources
            // waiting for files or workspaces to close.
            let mut timer = background.timer(Duration::from_secs(1)).fuse();
            futures::select_biased! {
                _ = wait => break,
                _ = timer => {
                    if responses.send(CliResponse::Ping).is_err() {
                        break;
                    }
                }
            }
        }
    }

    errored
}

pub async fn derive_paths_with_position(
    fs: &dyn Fs,
    path_strings: impl IntoIterator<Item = impl AsRef<str>>,
) -> Vec<PathWithPosition> {
    let path_strings: Vec<_> = path_strings.into_iter().collect();
    let mut result = Vec::with_capacity(path_strings.len());
    for path_str in path_strings {
        let original_path = Path::new(path_str.as_ref());
        let mut parsed = PathWithPosition::parse_str(path_str.as_ref());

        // If the unparsed path string actually points to a file, use that file instead of parsing out the line/col number.
        // Note: The colon syntax is also used to open NTFS alternate data streams (e.g., `file.txt:stream`), which would cause issues.
        // However, the colon is not valid in NTFS file names, so we can just skip this logic.
        if !cfg!(windows)
            && parsed.row.is_some()
            && parsed.path != original_path
            && fs.is_file(original_path).await
        {
            parsed = PathWithPosition::from_path(original_path.to_path_buf());
        }

        if let Ok(canonicalized) = fs.canonicalize(&parsed.path).await {
            parsed.path = canonicalized;
        }

        result.push(parsed);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zed::{open_listener::open_local_workspace, tests::init_test};
    use cli::{
        CliResponse,
        ipc::{self},
    };
    use editor::Editor;
    use futures::poll;
    use gpui::{AppContext as _, TestAppContext};
    use language::LineEnding;
    use remote::SshConnectionOptions;
    use rope::Rope;
    use serde_json::json;
    use std::{sync::Arc, task::Poll};
    use util::path;
    use workspace::{AppState, MultiWorkspace};

    #[gpui::test]
    fn test_parse_ssh_url(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);
        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec!["ssh://me@localhost:/".into()],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });
        assert_eq!(
            request.remote_connection.unwrap(),
            RemoteConnectionOptions::Ssh(SshConnectionOptions {
                host: "localhost".into(),
                username: Some("me".into()),
                port: None,
                password: None,
                args: None,
                port_forwards: None,
                nickname: None,
                upload_binary_over_ssh: false,
                connection_timeout: None,
            })
        );
        assert_eq!(request.open_paths, vec!["/"]);
    }

    #[gpui::test]
    fn test_parse_agent_url(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec!["zed://agent".into()],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::AgentPanel {
                external_source_prompt,
            }) => {
                assert_eq!(external_source_prompt, None);
            }
            _ => panic!("Expected AgentPanel kind"),
        }
    }

    fn agent_url_with_prompt(prompt: &str) -> String {
        let mut serializer = url::form_urlencoded::Serializer::new("zed://agent?".to_string());
        serializer.append_pair("prompt", prompt);
        serializer.finish()
    }

    #[gpui::test]
    fn test_parse_agent_url_with_prompt(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);
        let prompt = "Write me a script\nThanks";

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec![agent_url_with_prompt(prompt)],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::AgentPanel {
                external_source_prompt,
            }) => {
                assert_eq!(
                    external_source_prompt
                        .as_ref()
                        .map(ExternalSourcePrompt::as_str),
                    Some("Write me a script\nThanks")
                );
            }
            _ => panic!("Expected AgentPanel kind"),
        }
    }

    #[gpui::test]
    fn test_parse_agent_url_with_empty_prompt(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec![agent_url_with_prompt("")],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::AgentPanel {
                external_source_prompt,
            }) => {
                assert_eq!(external_source_prompt, None);
            }
            _ => panic!("Expected AgentPanel kind"),
        }
    }

    #[gpui::test]
    fn test_parse_shared_agent_thread_url(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);
        let session_id = "123e4567-e89b-12d3-a456-426614174000";

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec![format!("zed://agent/shared/{session_id}")],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::SharedAgentThread {
                session_id: parsed_session_id,
            }) => {
                assert_eq!(parsed_session_id, session_id);
            }
            _ => panic!("Expected SharedAgentThread kind"),
        }
    }

    #[gpui::test]
    fn test_parse_shared_agent_thread_url_with_invalid_uuid(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec!["zed://agent/shared/not-a-uuid".into()],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        assert!(request.kind.is_none());
    }

    #[gpui::test]
    fn test_parse_git_commit_url(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        // Test basic git commit URL
        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec!["zed://git/commit/abc123?repo=path/to/repo".into()],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind.unwrap() {
            OpenRequestKind::GitCommit { sha } => {
                assert_eq!(sha, "abc123");
            }
            _ => panic!("expected GitCommit variant"),
        }
        // Verify path was added to open_paths for workspace routing
        assert_eq!(request.open_paths, vec!["path/to/repo"]);

        // Test with URL encoded path
        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec!["zed://git/commit/def456?repo=path%20with%20spaces".into()],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind.unwrap() {
            OpenRequestKind::GitCommit { sha } => {
                assert_eq!(sha, "def456");
            }
            _ => panic!("expected GitCommit variant"),
        }
        assert_eq!(request.open_paths, vec!["path with spaces"]);

        // Test with empty path
        cx.update(|cx| {
            assert!(
                OpenRequest::parse(
                    RawOpenRequest {
                        urls: vec!["zed://git/commit/abc123?repo=".into()],
                        ..Default::default()
                    },
                    cx,
                )
                .unwrap_err()
                .to_string()
                .contains("missing repo")
            );
        });

        // Test error case: missing SHA
        let result = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec!["zed://git/commit/abc123?foo=bar".into()],
                    ..Default::default()
                },
                cx,
            )
        });
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing repo query parameter")
        );
    }

    #[gpui::test]
    async fn test_open_workspace_with_directory(cx: &mut TestAppContext) {
        let app_state = init_test(cx);

        app_state
            .fs
            .as_fake()
            .insert_tree(
                path!("/root"),
                json!({
                    "dir1": {
                        "file1.txt": "content1",
                        "file2.txt": "content2",
                    },
                }),
            )
            .await;

        assert_eq!(cx.windows().len(), 0);

        // First open the workspace directory
        open_workspace_file(path!("/root/dir1"), None, app_state.clone(), cx).await;

        assert_eq!(cx.windows().len(), 1);
        let multi_workspace = cx.windows()[0].downcast::<MultiWorkspace>().unwrap();
        multi_workspace
            .update(cx, |multi_workspace, _, cx| {
                multi_workspace.workspace().update(cx, |workspace, cx| {
                    assert!(workspace.active_item_as::<Editor>(cx).is_none())
                });
            })
            .unwrap();

        // Now open a file inside that workspace
        open_workspace_file(path!("/root/dir1/file1.txt"), None, app_state.clone(), cx).await;

        assert_eq!(cx.windows().len(), 1);
        multi_workspace
            .update(cx, |multi_workspace, _, cx| {
                multi_workspace.workspace().update(cx, |workspace, cx| {
                    assert!(workspace.active_item_as::<Editor>(cx).is_some());
                });
            })
            .unwrap();

        // Now open a file inside that workspace, but tell Zed to open a new window
        open_workspace_file(
            path!("/root/dir1/file1.txt"),
            Some(true),
            app_state.clone(),
            cx,
        )
        .await;

        assert_eq!(cx.windows().len(), 2);

        let multi_workspace_2 = cx.windows()[1].downcast::<MultiWorkspace>().unwrap();
        multi_workspace_2
            .update(cx, |multi_workspace, _, cx| {
                multi_workspace.workspace().update(cx, |workspace, cx| {
                    assert!(workspace.active_item_as::<Editor>(cx).is_some());
                    let items = workspace.items(cx).collect::<Vec<_>>();
                    assert_eq!(items.len(), 1, "Workspace should have two items");
                });
            })
            .unwrap();
    }

    #[gpui::test]
    async fn test_wait_with_directory_waits_for_window_close(cx: &mut TestAppContext) {
        let app_state = init_test(cx);

        app_state
            .fs
            .as_fake()
            .insert_tree(
                path!("/root"),
                json!({
                    "dir1": {
                        "file1.txt": "content1",
                    },
                }),
            )
            .await;

        let (response_tx, _) = ipc::channel::<CliResponse>().unwrap();
        let workspace_paths = vec![path!("/root/dir1").to_owned()];

        let (done_tx, mut done_rx) = futures::channel::oneshot::channel();
        cx.spawn({
            let app_state = app_state.clone();
            move |mut cx| async move {
                let errored = open_local_workspace(
                    workspace_paths,
                    vec![],
                    false,
                    workspace::OpenOptions {
                        wait: true,
                        ..Default::default()
                    },
                    &response_tx,
                    &app_state,
                    &mut cx,
                )
                .await;
                let _ = done_tx.send(errored);
            }
        })
        .detach();

        cx.background_executor.run_until_parked();
        assert_eq!(cx.windows().len(), 1);
        assert!(matches!(poll!(&mut done_rx), Poll::Pending));

        let window = cx.windows()[0];
        cx.update_window(window, |_, window, _| window.remove_window())
            .unwrap();
        cx.background_executor.run_until_parked();

        let errored = done_rx.await.unwrap();
        assert!(!errored);
    }

    #[gpui::test]
    async fn test_open_workspace_with_nonexistent_files(cx: &mut TestAppContext) {
        let app_state = init_test(cx);

        app_state
            .fs
            .as_fake()
            .insert_tree(path!("/root"), json!({}))
            .await;

        assert_eq!(cx.windows().len(), 0);

        // Test case 1: Open a single file that does not exist yet
        open_workspace_file(path!("/root/file5.txt"), None, app_state.clone(), cx).await;

        assert_eq!(cx.windows().len(), 1);
        let multi_workspace_1 = cx.windows()[0].downcast::<MultiWorkspace>().unwrap();
        multi_workspace_1
            .update(cx, |multi_workspace, _, cx| {
                multi_workspace.workspace().update(cx, |workspace, cx| {
                    assert!(workspace.active_item_as::<Editor>(cx).is_some())
                });
            })
            .unwrap();

        // Test case 2: Open a single file that does not exist yet,
        // but tell Zed to add it to the current workspace
        open_workspace_file(path!("/root/file6.txt"), Some(false), app_state.clone(), cx).await;

        assert_eq!(cx.windows().len(), 1);
        multi_workspace_1
            .update(cx, |multi_workspace, _, cx| {
                multi_workspace.workspace().update(cx, |workspace, cx| {
                    let items = workspace.items(cx).collect::<Vec<_>>();
                    assert_eq!(items.len(), 2, "Workspace should have two items");
                });
            })
            .unwrap();

        // Test case 3: Open a single file that does not exist yet,
        // but tell Zed to NOT add it to the current workspace
        open_workspace_file(path!("/root/file7.txt"), Some(true), app_state.clone(), cx).await;

        assert_eq!(cx.windows().len(), 2);
        let multi_workspace_2 = cx.windows()[1].downcast::<MultiWorkspace>().unwrap();
        multi_workspace_2
            .update(cx, |multi_workspace, _, cx| {
                multi_workspace.workspace().update(cx, |workspace, cx| {
                    let items = workspace.items(cx).collect::<Vec<_>>();
                    assert_eq!(items.len(), 1, "Workspace should have two items");
                });
            })
            .unwrap();
    }

    async fn open_workspace_file(
        path: &str,
        open_new_workspace: Option<bool>,
        app_state: Arc<AppState>,
        cx: &TestAppContext,
    ) {
        let (response_tx, _) = ipc::channel::<CliResponse>().unwrap();

        let workspace_paths = vec![path.to_owned()];

        let errored = cx
            .spawn(|mut cx| async move {
                open_local_workspace(
                    workspace_paths,
                    vec![],
                    false,
                    workspace::OpenOptions {
                        open_new_workspace,
                        ..Default::default()
                    },
                    &response_tx,
                    &app_state,
                    &mut cx,
                )
                .await
            })
            .await;

        assert!(!errored);
    }

    #[gpui::test]
    async fn test_reuse_flag_functionality(cx: &mut TestAppContext) {
        let app_state = init_test(cx);

        let root_dir = if cfg!(windows) { "C:\\root" } else { "/root" };
        let file1_path = if cfg!(windows) {
            "C:\\root\\file1.txt"
        } else {
            "/root/file1.txt"
        };
        let file2_path = if cfg!(windows) {
            "C:\\root\\file2.txt"
        } else {
            "/root/file2.txt"
        };

        app_state.fs.create_dir(Path::new(root_dir)).await.unwrap();
        app_state
            .fs
            .create_file(Path::new(file1_path), Default::default())
            .await
            .unwrap();
        app_state
            .fs
            .save(
                Path::new(file1_path),
                &Rope::from("content1"),
                LineEnding::Unix,
            )
            .await
            .unwrap();
        app_state
            .fs
            .create_file(Path::new(file2_path), Default::default())
            .await
            .unwrap();
        app_state
            .fs
            .save(
                Path::new(file2_path),
                &Rope::from("content2"),
                LineEnding::Unix,
            )
            .await
            .unwrap();

        // First, open a workspace normally
        let (response_tx, _response_rx) = ipc::channel::<CliResponse>().unwrap();
        let workspace_paths = vec![file1_path.to_string()];

        let _errored = cx
            .spawn({
                let app_state = app_state.clone();
                let response_tx = response_tx.clone();
                |mut cx| async move {
                    open_local_workspace(
                        workspace_paths,
                        vec![],
                        false,
                        workspace::OpenOptions::default(),
                        &response_tx,
                        &app_state,
                        &mut cx,
                    )
                    .await
                }
            })
            .await;

        // Now test the reuse functionality - should replace the existing workspace
        let workspace_paths_reuse = vec![file1_path.to_string()];
        let paths: Vec<PathBuf> = workspace_paths_reuse.iter().map(PathBuf::from).collect();
        let window_to_replace = workspace::find_existing_workspace(
            &paths,
            &workspace::OpenOptions::default(),
            &workspace::SerializedWorkspaceLocation::Local,
            &mut cx.to_async(),
        )
        .await
        .0
        .unwrap()
        .0;

        let errored_reuse = cx
            .spawn({
                let app_state = app_state.clone();
                let response_tx = response_tx.clone();
                |mut cx| async move {
                    open_local_workspace(
                        workspace_paths_reuse,
                        vec![],
                        false,
                        workspace::OpenOptions {
                            replace_window: Some(window_to_replace),
                            ..Default::default()
                        },
                        &response_tx,
                        &app_state,
                        &mut cx,
                    )
                    .await
                }
            })
            .await;

        assert!(!errored_reuse);
    }

    #[gpui::test]
    fn test_parse_git_clone_url(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec![
                        "zed://git/clone/?repo=https://example.com/example/repo.git".into(),
                    ],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::GitClone { repo_url }) => {
                assert_eq!(repo_url, "https://example.com/example/repo.git");
            }
            _ => panic!("Expected GitClone kind"),
        }
    }

    #[gpui::test]
    fn test_parse_git_clone_url_without_slash(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec![
                        "zed://git/clone?repo=https://example.com/example/repo.git".into(),
                    ],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::GitClone { repo_url }) => {
                assert_eq!(repo_url, "https://example.com/example/repo.git");
            }
            _ => panic!("Expected GitClone kind"),
        }
    }

    #[gpui::test]
    fn test_parse_git_clone_url_with_encoding(cx: &mut TestAppContext) {
        let _app_state = init_test(cx);

        let request = cx.update(|cx| {
            OpenRequest::parse(
                RawOpenRequest {
                    urls: vec![
                        "zed://git/clone/?repo=https%3A%2F%2Fexample.com%2Fexample%2Frepo.git"
                            .into(),
                    ],
                    ..Default::default()
                },
                cx,
            )
            .unwrap()
        });

        match request.kind {
            Some(OpenRequestKind::GitClone { repo_url }) => {
                assert_eq!(repo_url, "https://example.com/example/repo.git");
            }
            _ => panic!("Expected GitClone kind"),
        }
    }

    #[gpui::test]
    async fn test_add_flag_prefers_focused_window(cx: &mut TestAppContext) {
        let app_state = init_test(cx);

        let root_dir = if cfg!(windows) { "C:\\root" } else { "/root" };
        let file1_path = if cfg!(windows) {
            "C:\\root\\file1.txt"
        } else {
            "/root/file1.txt"
        };
        let file2_path = if cfg!(windows) {
            "C:\\root\\file2.txt"
        } else {
            "/root/file2.txt"
        };

        app_state.fs.create_dir(Path::new(root_dir)).await.unwrap();
        app_state
            .fs
            .create_file(Path::new(file1_path), Default::default())
            .await
            .unwrap();
        app_state
            .fs
            .save(
                Path::new(file1_path),
                &Rope::from("content1"),
                LineEnding::Unix,
            )
            .await
            .unwrap();
        app_state
            .fs
            .create_file(Path::new(file2_path), Default::default())
            .await
            .unwrap();
        app_state
            .fs
            .save(
                Path::new(file2_path),
                &Rope::from("content2"),
                LineEnding::Unix,
            )
            .await
            .unwrap();

        let (response_tx, _response_rx) = ipc::channel::<CliResponse>().unwrap();

        // Open first workspace
        let workspace_paths_1 = vec![file1_path.to_string()];
        let _errored = cx
            .spawn({
                let app_state = app_state.clone();
                let response_tx = response_tx.clone();
                |mut cx| async move {
                    open_local_workspace(
                        workspace_paths_1,
                        Vec::new(),
                        false,
                        workspace::OpenOptions::default(),
                        &response_tx,
                        &app_state,
                        &mut cx,
                    )
                    .await
                }
            })
            .await;

        assert_eq!(cx.windows().len(), 1);
        let multi_workspace_1 = cx.windows()[0].downcast::<MultiWorkspace>().unwrap();

        // Open second workspace in a new window
        let workspace_paths_2 = vec![file2_path.to_string()];
        let _errored = cx
            .spawn({
                let app_state = app_state.clone();
                let response_tx = response_tx.clone();
                |mut cx| async move {
                    open_local_workspace(
                        workspace_paths_2,
                        Vec::new(),
                        false,
                        workspace::OpenOptions {
                            open_new_workspace: Some(true), // Force new window
                            ..Default::default()
                        },
                        &response_tx,
                        &app_state,
                        &mut cx,
                    )
                    .await
                }
            })
            .await;

        assert_eq!(cx.windows().len(), 2);
        let multi_workspace_2 = cx.windows()[1].downcast::<MultiWorkspace>().unwrap();

        // Focus window2
        multi_workspace_2
            .update(cx, |_, window, _| {
                window.activate_window();
            })
            .unwrap();

        // Now use --add flag (open_new_workspace = Some(false)) to add a new file
        // It should open in the focused window (window2), not an arbitrary window
        let new_file_path = if cfg!(windows) {
            "C:\\root\\new_file.txt"
        } else {
            "/root/new_file.txt"
        };
        app_state
            .fs
            .create_file(Path::new(new_file_path), Default::default())
            .await
            .unwrap();

        let workspace_paths_add = vec![new_file_path.to_string()];
        let _errored = cx
            .spawn({
                let app_state = app_state.clone();
                let response_tx = response_tx.clone();
                |mut cx| async move {
                    open_local_workspace(
                        workspace_paths_add,
                        Vec::new(),
                        false,
                        workspace::OpenOptions {
                            open_new_workspace: Some(false), // --add flag
                            ..Default::default()
                        },
                        &response_tx,
                        &app_state,
                        &mut cx,
                    )
                    .await
                }
            })
            .await;

        // Should still have 2 windows (file added to existing focused window)
        assert_eq!(cx.windows().len(), 2);

        // Verify the file was added to window2 (the focused one)
        multi_workspace_2
            .update(cx, |workspace, _, cx| {
                let items = workspace.workspace().read(cx).items(cx).collect::<Vec<_>>();
                // Should have 2 items now (file2.txt and new_file.txt)
                assert_eq!(items.len(), 2, "Focused window should have 2 items");
            })
            .unwrap();

        // Verify window1 still has only 1 item
        multi_workspace_1
            .update(cx, |workspace, _, cx| {
                let items = workspace.workspace().read(cx).items(cx).collect::<Vec<_>>();
                assert_eq!(items.len(), 1, "Other window should still have 1 item");
            })
            .unwrap();
    }
}
