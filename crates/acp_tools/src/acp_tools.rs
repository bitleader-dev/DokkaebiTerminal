use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use agent_client_protocol::schema as acp;
use collections::HashMap;
use gpui::{
    App, Empty, Entity, EventEmitter, FocusHandle, Focusable, Global, ListAlignment, ListState,
    StyleRefinement, Subscription, Task, TextStyleRefinement, Window, actions, list, prelude::*,
};
use language::LanguageRegistry;
use markdown::{CodeBlockRenderer, Markdown, MarkdownElement, MarkdownStyle};
use project::{AgentId, Project};
use settings::Settings;
use theme_settings::ThemeSettings;
use ui::{CopyButton, Tooltip, WithScrollbar, prelude::*};
use util::ResultExt as _;
use workspace::{
    Item, ItemHandle, ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView, Workspace,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StreamMessageDirection {
    Incoming,
    Outgoing,
    /// 에이전트의 stderr에서 캡처한 라인. JSON-RPC 프로토콜의 일부는 아니지만
    /// 에이전트가 진단 정보를 남기는 데 자주 사용한다.
    Stderr,
}

#[derive(Clone)]
pub enum StreamMessageContent {
    Request {
        id: acp::RequestId,
        method: Arc<str>,
        params: Option<serde_json::Value>,
    },
    Response {
        id: acp::RequestId,
        result: Result<Option<serde_json::Value>, acp::Error>,
    },
    Notification {
        method: Arc<str>,
        params: Option<serde_json::Value>,
    },
    /// 에이전트 프로세스에서 캡처한 원시 stderr 라인.
    Stderr { line: Arc<str> },
}

#[derive(Clone)]
pub struct StreamMessage {
    pub direction: StreamMessageDirection,
    pub message: StreamMessageContent,
}

impl StreamMessage {
    /// 트랜스포트에서 캡처한 원시 라인을 `StreamMessage`로 변환한다.
    ///
    /// `Stderr` 방향은 JSON 파싱 없이 그대로 감싼다. `Incoming`/`Outgoing`은
    /// JSON-RPC로 파싱하며 유효한 JSON-RPC 메시지가 아니면 `None`을 반환한다.
    pub fn from_raw_line(direction: StreamMessageDirection, line: &str) -> Option<Self> {
        if direction == StreamMessageDirection::Stderr {
            return Some(StreamMessage {
                direction,
                message: StreamMessageContent::Stderr {
                    line: Arc::from(line),
                },
            });
        }

        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let obj = value.as_object()?;

        let parsed_id = obj
            .get("id")
            .map(|raw| serde_json::from_value::<acp::RequestId>(raw.clone()));

        let message = if let Some(method) = obj.get("method").and_then(|m| m.as_str()) {
            match parsed_id {
                Some(Ok(id)) => StreamMessageContent::Request {
                    id,
                    method: method.into(),
                    params: obj.get("params").cloned(),
                },
                Some(Err(err)) => {
                    log::warn!("Skipping JSON-RPC message with unparsable id: {err}");
                    return None;
                }
                None => StreamMessageContent::Notification {
                    method: method.into(),
                    params: obj.get("params").cloned(),
                },
            }
        } else if let Some(parsed_id) = parsed_id {
            let id = match parsed_id {
                Ok(id) => id,
                Err(err) => {
                    log::warn!("Skipping JSON-RPC response with unparsable id: {err}");
                    return None;
                }
            };
            if let Some(error) = obj.get("error") {
                let acp_err =
                    serde_json::from_value::<acp::Error>(error.clone()).unwrap_or_else(|err| {
                        log::warn!("Failed to deserialize ACP error: {err}");
                        acp::Error::internal_error().data(error.to_string())
                    });
                StreamMessageContent::Response {
                    id,
                    result: Err(acp_err),
                }
            } else {
                StreamMessageContent::Response {
                    id,
                    result: Ok(obj.get("result").cloned()),
                }
            }
        } else {
            return None;
        };

        Some(StreamMessage { direction, message })
    }
}

actions!(dev, [OpenAcpLogs]);

pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, _window, _cx: &mut Context<Workspace>| {
            workspace.register_action(|workspace, _: &OpenAcpLogs, window, cx| {
                let acp_tools =
                    Box::new(cx.new(|cx| AcpTools::new(workspace.project().clone(), cx)));
                workspace.add_item_to_active_pane(acp_tools, None, true, window, cx);
            });
        },
    )
    .detach();
}

struct GlobalAcpConnectionRegistry(Entity<AcpConnectionRegistry>);

impl Global for GlobalAcpConnectionRegistry {}

/// 트랜스포트(또는 stderr)에서 캡처한 원시 라인과 방향 태그. [`StreamMessage`]로의
/// 역직렬화는 레지스트리의 포그라운드 태스크에서 수행되므로, 링 버퍼가 뒤늦게
/// 구독하는 측에도 재생될 수 있다.
struct RawStreamLine {
    direction: StreamMessageDirection,
    line: Arc<str>,
}

/// ACP 연결의 로그 탭 핸들. [`AcpConnectionRegistry::set_active_connection`]이
/// 반환하며, 연결 측에서 이 탭을 통해 트랜스포트·stderr 라인을 푸시하면 로그
/// 패널의 채널 구조를 몰라도 된다.
///
/// 탭에는 공유 `enabled` 플래그가 있어, 첫 구독자가 나타날 때 레지스트리가
/// 이를 켠다. 그 전까지는 `emit_*` 호출이 사실상 비용이 없다: atomic load 후
/// 바로 반환한다. 따라서 로그 패널이 열리지 않으면 추가 메모리 부담이 없다.
#[derive(Clone)]
pub struct AcpLogTap {
    enabled: Arc<AtomicBool>,
    sender: smol::channel::Sender<RawStreamLine>,
}

impl AcpLogTap {
    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    fn emit(&self, direction: StreamMessageDirection, line: &str) {
        if !self.is_enabled() {
            return;
        }
        self.sender
            .try_send(RawStreamLine {
                direction,
                line: Arc::from(line),
            })
            .log_err();
    }

    /// 에이전트의 stdout에서 읽은 라인을 기록한다.
    pub fn emit_incoming(&self, line: &str) {
        self.emit(StreamMessageDirection::Incoming, line);
    }

    /// 에이전트의 stdin으로 보낸 라인을 기록한다.
    pub fn emit_outgoing(&self, line: &str) {
        self.emit(StreamMessageDirection::Outgoing, line);
    }

    /// 에이전트의 stderr에서 읽은 라인을 기록한다.
    pub fn emit_stderr(&self, line: &str) {
        self.emit(StreamMessageDirection::Stderr, line);
    }
}

/// 레지스트리의 백로그에 보관하는 최대 메시지 개수.
///
/// LSP 로그 스토어의 `MAX_STORED_LOG_ENTRIES`와 동일 크기로, 세션이 한동안
/// 진행된 뒤에 ACP 로그 패널을 열어도 의미 있는 히스토리를 볼 수 있도록 한다.
const MAX_BACKLOG_MESSAGES: usize = 2000;

#[derive(Default)]
pub struct AcpConnectionRegistry {
    active_agent_id: Option<AgentId>,
    generation: u64,
    /// 현재 연결에서 관찰된 모든 메시지의 유한한 링 버퍼. 새 연결이 설정되면
    /// 비워진다.
    backlog: VecDeque<StreamMessage>,
    subscribers: Vec<smol::channel::Sender<StreamMessage>>,
    /// 현재 활성 연결에 전달한 탭. 첫 구독자가 등장했을 때 `enabled` 플래그를
    /// 뒤집을 수 있도록 레지스트리가 보관한다.
    active_tap: Option<AcpLogTap>,
    _broadcast_task: Option<Task<()>>,
}

impl AcpConnectionRegistry {
    pub fn default_global(cx: &mut App) -> Entity<Self> {
        if cx.has_global::<GlobalAcpConnectionRegistry>() {
            cx.global::<GlobalAcpConnectionRegistry>().0.clone()
        } else {
            let registry = cx.new(|_cx| AcpConnectionRegistry::default());
            cx.set_global(GlobalAcpConnectionRegistry(registry.clone()));
            registry
        }
    }

    /// 새 활성 연결을 등록하고 [`AcpLogTap`]을 반환한다. 연결 측은 이 탭을
    /// 트랜스포트와 stderr 리더에 전달해야 한다.
    ///
    /// 탭은 초기에 비활성 상태다: [`Self::subscribe`]를 통해 누군가 구독할
    /// 때까지 트랜스포트 라인을 저렴하게 버리고, 구독이 발생하면 탭이 켜져
    /// 이후 라인이 현재·미래의 구독자 모두에게 브로드캐스트된다.
    pub fn set_active_connection(
        &mut self,
        agent_id: AgentId,
        cx: &mut Context<Self>,
    ) -> AcpLogTap {
        let (sender, raw_rx) = smol::channel::unbounded::<RawStreamLine>();
        let tap = AcpLogTap {
            enabled: Arc::new(AtomicBool::new(false)),
            sender,
        };

        self.active_agent_id = Some(agent_id);
        self.generation += 1;
        self.backlog.clear();
        self.subscribers.clear();
        self.active_tap = Some(tap.clone());

        self._broadcast_task = Some(cx.spawn(async move |this, cx| {
            while let Ok(raw) = raw_rx.recv().await {
                this.update(cx, |this, _cx| {
                    let Some(message) = StreamMessage::from_raw_line(raw.direction, &raw.line)
                    else {
                        return;
                    };

                    if this.backlog.len() == MAX_BACKLOG_MESSAGES {
                        this.backlog.pop_front();
                    }
                    this.backlog.push_back(message.clone());

                    this.subscribers.retain(|sender| !sender.is_closed());
                    for sender in &this.subscribers {
                        sender.try_send(message.clone()).log_err();
                    }
                })
                .log_err();
            }

            // 트랜스포트가 닫히면 상태를 비워 관찰자(예: ACP 로그 탭)가
            // 끊긴 상태로 되돌아갈 수 있도록 한다.
            this.update(cx, |this, cx| {
                this.active_agent_id = None;
                this.subscribers.clear();
                this.active_tap = None;
                cx.notify();
            })
            .log_err();
        }));

        cx.notify();
        tap
    }

    /// 현재 연결에 대해 보관 중인 메시지 히스토리를 지우고, 관찰자들이 다시
    /// 구독하도록 강제해 로컬 상관 상태도 초기화한다.
    pub fn clear_messages(&mut self, cx: &mut Context<Self>) {
        self.backlog.clear();
        self.generation += 1;
        self.subscribers.clear();
        cx.notify();
    }

    /// 현재 연결의 메시지를 구독한다.
    ///
    /// 이미 관찰된 메시지의 백로그와 새 메시지용 수신자를 함께 반환한다.
    /// 호출자는 수신자에서 메시지를 꺼내기 전에 백로그를 로컬 상태로 먼저
    /// 반영해야 스냅샷과 라이브 구독 사이에서 메시지가 누락되지 않는다.
    ///
    /// 첫 구독은 연결의 로그 탭을 활성화한다. 그 이전 메시지는 기록되지 않는다.
    /// 이는 의도된 동작이다: ACP 로그 패널을 아무도 열지 않은 기본 경우에는
    /// 메시지당 부가 작업을 전혀 하지 않도록 탭을 opt-in으로 유지한다.
    pub fn subscribe(&mut self) -> (Vec<StreamMessage>, smol::channel::Receiver<StreamMessage>) {
        if let Some(tap) = &self.active_tap {
            tap.enable();
        }
        let backlog = self.backlog.iter().cloned().collect();
        let (sender, receiver) = smol::channel::unbounded();
        self.subscribers.push(sender);
        (backlog, receiver)
    }
}

struct AcpTools {
    project: Entity<Project>,
    focus_handle: FocusHandle,
    expanded: HashSet<usize>,
    watched_connection: Option<WatchedConnection>,
    connection_registry: Entity<AcpConnectionRegistry>,
    _subscription: Subscription,
}

struct WatchedConnection {
    agent_id: AgentId,
    generation: u64,
    messages: Vec<WatchedConnectionMessage>,
    list_state: ListState,
    incoming_request_methods: HashMap<acp::RequestId, Arc<str>>,
    outgoing_request_methods: HashMap<acp::RequestId, Arc<str>>,
    _task: Task<()>,
}

impl AcpTools {
    fn new(project: Entity<Project>, cx: &mut Context<Self>) -> Self {
        let connection_registry = AcpConnectionRegistry::default_global(cx);

        let subscription = cx.observe(&connection_registry, |this, _, cx| {
            this.update_connection(cx);
            cx.notify();
        });

        let mut this = Self {
            project,
            focus_handle: cx.focus_handle(),
            expanded: HashSet::default(),
            watched_connection: None,
            connection_registry,
            _subscription: subscription,
        };
        this.update_connection(cx);
        this
    }

    fn update_connection(&mut self, cx: &mut Context<Self>) {
        let (generation, agent_id) = {
            let registry = self.connection_registry.read(cx);
            (registry.generation, registry.active_agent_id.clone())
        };

        let Some(agent_id) = agent_id else {
            self.watched_connection = None;
            self.expanded.clear();
            return;
        };

        if let Some(watched) = self.watched_connection.as_ref() {
            if watched.generation == generation {
                return;
            }
        }

        self.expanded.clear();

        let (backlog, messages_rx) = self
            .connection_registry
            .update(cx, |registry, _cx| registry.subscribe());

        let task = cx.spawn(async move |this, cx| {
            while let Ok(message) = messages_rx.recv().await {
                this.update(cx, |this, cx| {
                    this.push_stream_message(message, cx);
                })
                .log_err();
            }
        });

        self.watched_connection = Some(WatchedConnection {
            agent_id,
            generation,
            messages: vec![],
            list_state: ListState::new(0, ListAlignment::Bottom, px(2048.)),
            incoming_request_methods: HashMap::default(),
            outgoing_request_methods: HashMap::default(),
            _task: task,
        });

        for message in backlog {
            self.push_stream_message(message, cx);
        }
    }

    fn push_stream_message(&mut self, stream_message: StreamMessage, cx: &mut Context<Self>) {
        let Some(connection) = self.watched_connection.as_mut() else {
            return;
        };
        let language_registry = self.project.read(cx).languages().clone();
        let index = connection.messages.len();

        let (request_id, method, message_type, params) = match stream_message.message {
            StreamMessageContent::Request { id, method, params } => {
                let method_map = match stream_message.direction {
                    StreamMessageDirection::Incoming => &mut connection.incoming_request_methods,
                    StreamMessageDirection::Outgoing => &mut connection.outgoing_request_methods,
                    // stderr 라인은 요청/응답 상관이 없다.
                    StreamMessageDirection::Stderr => return,
                };

                method_map.insert(id.clone(), method.clone());
                (Some(id), method.into(), MessageType::Request, Ok(params))
            }
            StreamMessageContent::Response { id, result } => {
                let method_map = match stream_message.direction {
                    StreamMessageDirection::Incoming => &mut connection.outgoing_request_methods,
                    StreamMessageDirection::Outgoing => &mut connection.incoming_request_methods,
                    StreamMessageDirection::Stderr => return,
                };

                if let Some(method) = method_map.remove(&id) {
                    (Some(id), method.into(), MessageType::Response, result)
                } else {
                    (
                        Some(id),
                        "[unrecognized response]".into(),
                        MessageType::Response,
                        result,
                    )
                }
            }
            StreamMessageContent::Notification { method, params } => {
                (None, method.into(), MessageType::Notification, Ok(params))
            }
            StreamMessageContent::Stderr { line } => {
                // stderr는 JSON-RPC 트래픽과 함께 플레인 텍스트로 렌더링된다.
                // 실제 메서드와 동일하게 헤더에 노출되도록 `stderr`를 의사
                // 메서드 이름으로 사용한다.
                (
                    None,
                    "stderr".into(),
                    MessageType::Stderr,
                    Ok(Some(serde_json::Value::String(line.to_string()))),
                )
            }
        };

        let message = WatchedConnectionMessage {
            name: method,
            message_type,
            request_id,
            direction: stream_message.direction,
            collapsed_params_md: match params.as_ref() {
                Ok(params) => params
                    .as_ref()
                    .map(|params| collapsed_params_md(params, &language_registry, cx)),
                Err(err) => {
                    if let Ok(err) = &serde_json::to_value(err) {
                        Some(collapsed_params_md(&err, &language_registry, cx))
                    } else {
                        None
                    }
                }
            },

            expanded_params_md: None,
            params,
        };

        connection.messages.push(message);
        connection.list_state.splice(index..index, 1);
        cx.notify();
    }

    fn serialize_observed_messages(&self) -> Option<String> {
        let connection = self.watched_connection.as_ref()?;

        let messages: Vec<serde_json::Value> = connection
            .messages
            .iter()
            .filter_map(|message| {
                let params = match &message.params {
                    Ok(Some(params)) => params.clone(),
                    Ok(None) => serde_json::Value::Null,
                    Err(err) => serde_json::to_value(err).ok()?,
                };
                Some(serde_json::json!({
                    "_direction": match message.direction {
                        StreamMessageDirection::Incoming => "incoming",
                        StreamMessageDirection::Outgoing => "outgoing",
                        StreamMessageDirection::Stderr => "stderr",
                    },
                    "_type": message.message_type.to_string().to_lowercase(),
                    "id": message.request_id,
                    "method": message.name.to_string(),
                    "params": params,
                }))
            })
            .collect();

        serde_json::to_string_pretty(&messages).ok()
    }

    fn clear_messages(&mut self, cx: &mut Context<Self>) {
        if let Some(connection) = self.watched_connection.as_mut() {
            connection.messages.clear();
            connection.list_state.reset(0);
            connection.incoming_request_methods.clear();
            connection.outgoing_request_methods.clear();
            self.expanded.clear();
            cx.notify();
        }
    }

    fn render_message(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(connection) = self.watched_connection.as_ref() else {
            return Empty.into_any();
        };

        let Some(message) = connection.messages.get(index) else {
            return Empty.into_any();
        };

        let base_size = TextSize::Editor.rems(cx);

        let theme_settings = ThemeSettings::get_global(cx);
        let text_style = window.text_style();

        let colors = cx.theme().colors();
        let expanded = self.expanded.contains(&index);

        v_flex()
            .id(index)
            .group("message")
            .font_buffer(cx)
            .w_full()
            .py_3()
            .pl_4()
            .pr_5()
            .gap_2()
            .items_start()
            .text_size(base_size)
            .border_color(colors.border)
            .border_b_1()
            .hover(|this| this.bg(colors.element_background.opacity(0.5)))
            .child(
                h_flex()
                    .id(("acp-log-message-header", index))
                    .w_full()
                    .gap_2()
                    .flex_shrink_0()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.expanded.contains(&index) {
                            this.expanded.remove(&index);
                        } else {
                            this.expanded.insert(index);
                            let Some(connection) = &mut this.watched_connection else {
                                return;
                            };
                            let Some(message) = connection.messages.get_mut(index) else {
                                return;
                            };
                            message.expanded(this.project.read(cx).languages().clone(), cx);
                            connection.list_state.scroll_to_reveal_item(index);
                        }
                        cx.notify()
                    }))
                    .child(match message.direction {
                        StreamMessageDirection::Incoming => Icon::new(IconName::ArrowDown)
                            .color(Color::Error)
                            .size(IconSize::Small),
                        StreamMessageDirection::Outgoing => Icon::new(IconName::ArrowUp)
                            .color(Color::Success)
                            .size(IconSize::Small),
                        StreamMessageDirection::Stderr => Icon::new(IconName::Warning)
                            .color(Color::Warning)
                            .size(IconSize::Small),
                    })
                    .child(
                        Label::new(message.name.clone())
                            .buffer_font(cx)
                            .color(Color::Muted),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .child(ui::Chip::new(message.message_type.to_string()))
                            .visible_on_hover("message"),
                    )
                    .children(
                        message
                            .request_id
                            .as_ref()
                            .map(|req_id| div().child(ui::Chip::new(req_id.to_string()))),
                    ),
            )
            // I'm aware using markdown is a hack. Trying to get something working for the demo.
            // Will clean up soon!
            .when_some(
                if expanded {
                    message.expanded_params_md.clone()
                } else {
                    message.collapsed_params_md.clone()
                },
                |this, params| {
                    this.child(
                        div().pl_6().w_full().child(
                            MarkdownElement::new(
                                params,
                                MarkdownStyle {
                                    base_text_style: text_style,
                                    selection_background_color: colors.element_selection_background,
                                    syntax: cx.theme().syntax().clone(),
                                    code_block_overflow_x_scroll: true,
                                    code_block: StyleRefinement {
                                        text: TextStyleRefinement {
                                            font_family: Some(
                                                theme_settings.buffer_font.family.clone(),
                                            ),
                                            font_size: Some((base_size * 0.8).into()),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            )
                            .code_block_renderer(
                                CodeBlockRenderer::Default {
                                    copy_button: false,
                                    copy_button_on_hover: expanded,
                                    border: false,
                                },
                            ),
                        ),
                    )
                },
            )
            .into_any()
    }
}

struct WatchedConnectionMessage {
    name: SharedString,
    request_id: Option<acp::RequestId>,
    direction: StreamMessageDirection,
    message_type: MessageType,
    params: Result<Option<serde_json::Value>, acp::Error>,
    collapsed_params_md: Option<Entity<Markdown>>,
    expanded_params_md: Option<Entity<Markdown>>,
}

impl WatchedConnectionMessage {
    fn expanded(&mut self, language_registry: Arc<LanguageRegistry>, cx: &mut App) {
        let params_md = match &self.params {
            Ok(Some(params)) => Some(expanded_params_md(params, &language_registry, cx)),
            Err(err) => {
                if let Some(err) = &serde_json::to_value(err).log_err() {
                    Some(expanded_params_md(&err, &language_registry, cx))
                } else {
                    None
                }
            }
            _ => None,
        };
        self.expanded_params_md = params_md;
    }
}

fn collapsed_params_md(
    params: &serde_json::Value,
    language_registry: &Arc<LanguageRegistry>,
    cx: &mut App,
) -> Entity<Markdown> {
    let params_json = serde_json::to_string(params).unwrap_or_default();
    let mut spaced_out_json = String::with_capacity(params_json.len() + params_json.len() / 4);

    for ch in params_json.chars() {
        match ch {
            '{' => spaced_out_json.push_str("{ "),
            '}' => spaced_out_json.push_str(" }"),
            ':' => spaced_out_json.push_str(": "),
            ',' => spaced_out_json.push_str(", "),
            c => spaced_out_json.push(c),
        }
    }

    let params_md = format!("```json\n{}\n```", spaced_out_json);
    cx.new(|cx| Markdown::new(params_md.into(), Some(language_registry.clone()), None, cx))
}

fn expanded_params_md(
    params: &serde_json::Value,
    language_registry: &Arc<LanguageRegistry>,
    cx: &mut App,
) -> Entity<Markdown> {
    let params_json = serde_json::to_string_pretty(params).unwrap_or_default();
    let params_md = format!("```json\n{}\n```", params_json);
    cx.new(|cx| Markdown::new(params_md.into(), Some(language_registry.clone()), None, cx))
}

enum MessageType {
    Request,
    Response,
    Notification,
    Stderr,
}

impl Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Request => write!(f, "Request"),
            MessageType::Response => write!(f, "Response"),
            MessageType::Notification => write!(f, "Notification"),
            MessageType::Stderr => write!(f, "Stderr"),
        }
    }
}

enum AcpToolsEvent {}

impl EventEmitter<AcpToolsEvent> for AcpTools {}

impl Item for AcpTools {
    type Event = AcpToolsEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> ui::SharedString {
        format!(
            "ACP: {}",
            self.watched_connection
                .as_ref()
                .map_or("Disconnected", |connection| connection.agent_id.0.as_ref())
        )
        .into()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(ui::Icon::new(IconName::Thread))
    }
}

impl Focusable for AcpTools {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AcpTools {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(match self.watched_connection.as_ref() {
                Some(connection) => {
                    if connection.messages.is_empty() {
                        h_flex()
                            .size_full()
                            .justify_center()
                            .items_center()
                            .child("No messages recorded yet")
                            .into_any()
                    } else {
                        div()
                            .size_full()
                            .flex_grow()
                            .child(
                                list(
                                    connection.list_state.clone(),
                                    cx.processor(Self::render_message),
                                )
                                .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
                                .size_full(),
                            )
                            .vertical_scrollbar_for(&connection.list_state, window, cx)
                            .into_any()
                    }
                }
                None => h_flex()
                    .size_full()
                    .justify_center()
                    .items_center()
                    .child("No active connection")
                    .into_any(),
            })
    }
}

pub struct AcpToolsToolbarItemView {
    acp_tools: Option<Entity<AcpTools>>,
}

impl AcpToolsToolbarItemView {
    pub fn new() -> Self {
        Self { acp_tools: None }
    }
}

impl Render for AcpToolsToolbarItemView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(acp_tools) = self.acp_tools.as_ref() else {
            return Empty.into_any_element();
        };

        let acp_tools = acp_tools.clone();
        let connection_registry = acp_tools.read(cx).connection_registry.clone();
        let has_messages = acp_tools
            .read(cx)
            .watched_connection
            .as_ref()
            .is_some_and(|connection| !connection.messages.is_empty());

        h_flex()
            .gap_2()
            .child({
                let message = acp_tools
                    .read(cx)
                    .serialize_observed_messages()
                    .unwrap_or_default();

                CopyButton::new("copy-all-messages", message)
                    .tooltip_label("Copy All Messages")
                    .disabled(!has_messages)
            })
            .child(
                IconButton::new("clear_messages", IconName::Trash)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Clear Messages"))
                    .disabled(!has_messages)
                    .on_click(cx.listener(move |_this, _, _window, cx| {
                        connection_registry.update(cx, |registry, cx| {
                            registry.clear_messages(cx);
                        });
                        acp_tools.update(cx, |acp_tools, cx| {
                            acp_tools.clear_messages(cx);
                        });
                    })),
            )
            .into_any()
    }
}

impl EventEmitter<ToolbarItemEvent> for AcpToolsToolbarItemView {}

impl ToolbarItemView for AcpToolsToolbarItemView {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarItemLocation {
        if let Some(item) = active_pane_item
            && let Some(acp_tools) = item.downcast::<AcpTools>()
        {
            self.acp_tools = Some(acp_tools);
            cx.notify();
            return ToolbarItemLocation::PrimaryRight;
        }
        if self.acp_tools.take().is_some() {
            cx.notify();
        }
        ToolbarItemLocation::Hidden
    }
}
