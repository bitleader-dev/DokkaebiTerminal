// 메모장 사이드 패널
// 오른쪽에 도킹되는 간단한 텍스트 메모장 패널

use anyhow::Result;
use editor::{
    actions::{Copy, Cut, Paste},
    Editor, EditorMode, MultiBufferOffset, SizingBehavior,
};
use gpui::{
    actions, anchored, deferred, div, px, App, AsyncWindowContext, Context, DismissEvent, Entity,
    EventEmitter, FocusHandle, Focusable, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Pixels, Point, Render, Styled, Subscription, Task, WeakEntity, Window,
};
use i18n::t;
use language::language_settings::SoftWrap;
use project::debounced_delay::DebouncedDelay;
use serde::{Deserialize, Serialize};
use settings::{RegisterSetting, Settings, SettingsStore};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terminal_view::{terminal_panel::TerminalPanel, TerminalView};
use ui::{prelude::*, ContextMenu, IconName, Label};
use uuid::Uuid;
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Item, Workspace,
};

// 메모장 패널 설정
#[derive(Debug, Clone, PartialEq, RegisterSetting)]
pub struct NotepadPanelSettings {
    pub button: bool,
    pub dock: DockPosition,
    pub default_width: Pixels,
    pub restore: bool,
    pub horizontal_scroll: bool,
    /// 워크스페이스 그룹마다 메모장을 분리해 표시할지 여부
    pub multi_memo: bool,
}

impl Settings for NotepadPanelSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        let notepad_panel = content.notepad_panel.clone().unwrap();
        Self {
            button: notepad_panel.button.unwrap(),
            dock: notepad_panel.dock.unwrap().into(),
            default_width: px(notepad_panel.default_width.unwrap()),
            restore: notepad_panel.restore.unwrap(),
            horizontal_scroll: notepad_panel.horizontal_scroll.unwrap(),
            multi_memo: notepad_panel.multi_memo.unwrap(),
        }
    }
}

/// 단일 모드 저장 파일 경로: data_dir()/notepad.json
fn single_save_path() -> PathBuf {
    paths::data_dir().join("notepad.json")
}

/// 멀티 모드 저장 디렉터리: data_dir()/notepad/
fn multi_dir() -> PathBuf {
    paths::data_dir().join("notepad")
}

/// 멀티 모드 그룹별 파일 경로: data_dir()/notepad/<uuid>.json
fn group_file_path(uuid: &Uuid) -> PathBuf {
    multi_dir().join(format!("{}.json", uuid))
}

/// 디스크에 기록되는 형태(말미 개행 제거 후)의 텍스트 해시. flush_save 의 변경 감지 가드용.
fn text_hash_for_disk(text: &str) -> u64 {
    let trimmed = text.trim_end_matches(|c: char| c == '\n' || c == '\r');
    let mut hasher = DefaultHasher::new();
    trimmed.hash(&mut hasher);
    hasher.finish()
}

/// 키스트로크 자동 저장 디바운스 시간.
/// 키스트로크마다 동기 디스크 쓰기를 유발하던 기존 동작 → 디바운스 후 1 회로 축소해
/// UI 스레드 블로킹 위험을 줄인다. swap·마이그레이션·드랍 시점에는 즉시 flush 한다.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

// 메모장 패널 토글 액션
actions!(notepad_panel, [ToggleFocus]);

/// 메모장 패널 초기화
pub fn init(cx: &mut App) {
    NotepadPanelSettings::register(cx);

    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<NotepadPanel>(window, cx);
        });
    })
    .detach();
}

/// 메모장 데이터 저장 구조
#[derive(Serialize, Deserialize, Default)]
struct NotepadData {
    content: String,
}

/// 메모장 패널 구조체
pub struct NotepadPanel {
    /// 텍스트 편집기
    editor: Entity<Editor>,
    /// 파일 시스템 (설정 저장용)
    fs: Arc<dyn fs::Fs>,
    /// 상위 워크스페이스 약한 참조 (컨텍스트 메뉴에서 터미널 패널 조회용)
    workspace: WeakEntity<Workspace>,
    /// 현재 표시 중인 우클릭 컨텍스트 메뉴와 표시 좌표/DismissEvent 구독.
    /// 패널 render에서 deferred anchored로 직접 그리고, 메뉴 dismiss 시 자동 해제된다.
    context_menu: Option<(Entity<ContextMenu>, Point<Pixels>, Subscription)>,
    /// 옵저버 변경 감지용 이전 설정값
    last_horizontal_scroll: bool,
    /// 현재 모드 캐시 (글로벌 설정과 별개로 마이그레이션 진행 중 명시 제어용)
    multi_memo: bool,
    /// 멀티 모드에서 마지막으로 표시한 활성 그룹 UUID
    last_active_uuid: Option<Uuid>,
    /// 멀티 모드에서 마지막으로 확인한 전체 그룹 UUID 집합 (그룹 삭제 감지용)
    last_known_uuids: HashSet<Uuid>,
    /// set_text 등 마이그레이션·swap 중 BufferEdited 가 다시 save_content 를 호출하는
    /// 재진입을 막기 위한 일시적 저장 억제 플래그.
    suppress_save: bool,
    /// 키스트로크 자동 저장 디바운스. 새 입력이 들어오면 이전 fire 가 oneshot 으로 cancel 되고
    /// 새 timer 가 등록된다. swap·마이그레이션·release 시점에는 cancel + 동기 flush.
    save_debouncer: DebouncedDelay<Self>,
    /// 마지막으로 디스크에 기록한 텍스트의 해시. 디바운스 만료마다 동일 콘텐츠를
    /// 매번 다시 쓰는 낭비를 막는다. set_text_suppressed 가 swap·마이그레이션 직후
    /// 디스크와 동일한 해시로 갱신하므로 그 직후의 BufferEdited 로 트리거되는 저장도 skip 된다.
    last_saved_hash: Option<u64>,
}

impl NotepadPanel {
    pub fn new(
        workspace: &Workspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let fs = workspace.app_state().fs.clone();

        // 초기 모드·그룹 정보 캡처
        let multi_memo = NotepadPanelSettings::get_global(cx).multi_memo;
        let restore = NotepadPanelSettings::get_global(cx).restore;
        let active_uuid = workspace.active_group_uuid();
        let all_uuids: HashSet<Uuid> = workspace.group_uuids().collect();

        // 초기 표시 텍스트 결정
        let initial_text = if !restore {
            String::new()
        } else if multi_memo {
            // 멀티 모드: 활성 그룹의 파일 콘텐츠
            active_uuid.map(Self::load_for_uuid).unwrap_or_default()
        } else {
            // 단일 모드: 단일 파일 콘텐츠
            Self::load_from_file(&single_save_path())
        };
        // 방금 읽어온 디스크 콘텐츠라 hash 동일. 이후 schedule_save 의 동일 콘텐츠 저장 skip 가드용.
        let initial_hash = text_hash_for_disk(&initial_text);

        // 멀티라인 에디터 생성
        let editor = cx.new(|cx| {
            let mut editor = Editor::multi_line(window, cx);
            editor.set_placeholder_text(
                &t("notepad_panel.placeholder", cx).to_string(),
                window,
                cx,
            );
            // 메모장에서 불필요한 거터 요소 비활성화 → 라인 번호를 왼쪽에 밀착
            editor.set_show_runnables(false, cx);
            editor.set_show_code_actions(false, cx);
            editor.set_show_git_diff_gutter(false, cx);
            // 가로 스크롤 활성화 시 줄바꿈을 끄고, 비활성화 시 에디터 너비에 맞춰 줄바꿈
            let horizontal_scroll = NotepadPanelSettings::get_global(cx).horizontal_scroll;
            if horizontal_scroll {
                editor.set_soft_wrap_mode(SoftWrap::None, cx);
            } else {
                editor.set_soft_wrap_mode(SoftWrap::EditorWidth, cx);
            }
            // 에디터 설정의 scroll_beyond_last_line에 따라 overscroll 적용
            editor.set_mode(EditorMode::Full {
                scale_ui_elements_with_buffer_font_size: true,
                show_active_line_background: true,
                sizing_behavior: SizingBehavior::Default,
            });
            if !initial_text.is_empty() {
                editor.set_text(initial_text, window, cx);
            }
            editor
        });

        // 설정 변경 감지 → 가로 스크롤 + multi_memo 변화 처리
        cx.observe_global_in::<SettingsStore>(window, |this, window, cx| {
            this.handle_settings_change(window, cx);
        })
        .detach();

        // 워크스페이스 entity 변경 감지 → 그룹 전환·삭제 추적
        if let Some(workspace_entity) = workspace.weak_handle().upgrade() {
            cx.observe_in(&workspace_entity, window, |this, ws, window, cx| {
                this.handle_workspace_notify(ws, window, cx);
            })
            .detach();
        }

        // 에디터 변경 감지 → 디바운스 자동 저장 (키스트로크 폭주 시 디스크 I/O 누적 방지)
        cx.subscribe_in(&editor, window, |this, _editor, event: &editor::EditorEvent, _window, cx| {
            if matches!(event, editor::EditorEvent::BufferEdited { .. }) {
                this.schedule_save(cx);
            }
        })
        .detach();

        // 패널 release 시 pending 디바운스를 즉시 flush — 종료/창 닫힘 시 마지막 입력
        // 손실 방지. self.save_debounce_task 의 drop 으로 timer future 가 cancel 되므로
        // 타이머가 끝나기 전에 직접 flush_save 를 부른다.
        cx.on_release(|this: &mut Self, cx: &mut App| {
            this.flush_save(cx);
        })
        .detach();

        Self {
            editor,
            fs,
            workspace: workspace.weak_handle(),
            context_menu: None,
            last_horizontal_scroll: NotepadPanelSettings::get_global(cx).horizontal_scroll,
            multi_memo,
            last_active_uuid: active_uuid,
            last_known_uuids: all_uuids,
            suppress_save: false,
            save_debouncer: DebouncedDelay::new(),
            last_saved_hash: Some(initial_hash),
        }
    }

    /// 현재 모드·활성 그룹에 대응하는 저장 경로.
    /// 멀티 모드에서 활성 그룹 UUID 가 없으면 None — 저장이 skip 된다.
    fn current_save_path(&self) -> Option<PathBuf> {
        if self.multi_memo {
            self.last_active_uuid.map(|uuid| group_file_path(&uuid))
        } else {
            Some(single_save_path())
        }
    }

    /// 파일에서 메모 내용 로드 (없거나 손상이면 빈 문자열)
    fn load_from_file(path: &PathBuf) -> String {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(notepad_data) = serde_json::from_str::<NotepadData>(&data) {
                return notepad_data.content;
            }
        }
        String::new()
    }

    /// 멀티 모드에서 그룹 UUID 에 해당하는 파일 콘텐츠 로드.
    fn load_for_uuid(uuid: Uuid) -> String {
        Self::load_from_file(&group_file_path(&uuid))
    }

    /// 지정한 콘텐츠를 파일에 기록 (부모 디렉터리 자동 생성, 빈 콘텐츠도 기록).
    /// 사용자 데이터 손실로 직결되므로 실패는 log::warn 으로 노출한다.
    fn write_to_file(path: &PathBuf, content: &str) {
        let trimmed = content.trim_end_matches(|c: char| c == '\n' || c == '\r');
        let data = NotepadData { content: trimmed.to_string() };
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::warn!(
                    "notepad_panel: 디렉터리 생성 실패 {:?}: {}",
                    parent,
                    e
                );
            }
        }
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    log::warn!("notepad_panel: 쓰기 실패 {:?}: {}", path, e);
                }
            }
            Err(e) => {
                log::warn!("notepad_panel: JSON 직렬화 실패 {:?}: {}", path, e);
            }
        }
    }

    /// 현재 에디터 내용을 즉시 동기로 디스크에 기록.
    /// suppress_save, 멀티 모드의 활성 UUID 없음, 디스크와 동일한 콘텐츠 — 어느 하나라도 해당하면 skip.
    fn flush_save(&mut self, cx: &App) {
        if self.suppress_save {
            return;
        }
        let Some(save_path) = self.current_save_path() else {
            return;
        };
        let text = self.editor.read(cx).text(cx);
        let hash = text_hash_for_disk(&text);
        if Some(hash) == self.last_saved_hash {
            return;
        }
        Self::write_to_file(&save_path, &text);
        self.last_saved_hash = Some(hash);
    }

    /// 마이그레이션·swap 직후 새 콘텐츠로 에디터를 갱신하면서 디스크와의 hash 도 동기화.
    /// suppress_save 토글로 set_text 가 트리거하는 BufferEdited 의 재진입을 막는다.
    fn set_text_suppressed(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let hash = text_hash_for_disk(&text);
        self.suppress_save = true;
        self.editor.update(cx, |editor, cx| {
            editor.set_text(text, window, cx);
        });
        self.suppress_save = false;
        self.last_saved_hash = Some(hash);
    }

    /// 디바운스 큐에 저장을 예약. 이전 fire 는 DebouncedDelay 가 oneshot 으로 cancel.
    /// 키스트로크 폭주 시 SAVE_DEBOUNCE 동안 한 번만 실제 디스크 I/O 가 일어난다.
    ///
    /// 디바운스 경로는 `fs::Fs::atomic_write` 를 사용 — 임시파일 + atomic rename 으로
    /// 전원 단절 시 corruption 방어. on_release / 마이그레이션 / swap 의 동기 flush 는
    /// 데이터 정합성(앱 종료/모드 전환 락스텝) 보장을 위해 `flush_save` 의 std::fs 동기 호출 유지.
    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        if self.suppress_save {
            return;
        }
        self.save_debouncer
            .fire_new(SAVE_DEBOUNCE, cx, |this, cx| {
                // 동기 가드 + 직렬화는 main thread 에서 수행 (entity 접근 필요).
                if this.suppress_save {
                    return Task::ready(());
                }
                let Some(save_path) = this.current_save_path() else {
                    return Task::ready(());
                };
                let text = this.editor.read(cx).text(cx);
                let hash = text_hash_for_disk(&text);
                if Some(hash) == this.last_saved_hash {
                    return Task::ready(());
                }
                let trimmed = text.trim_end_matches(|c: char| c == '\n' || c == '\r');
                let data = NotepadData {
                    content: trimmed.to_string(),
                };
                let json = match serde_json::to_string_pretty(&data) {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!(
                            "notepad_panel: JSON 직렬화 실패 {:?}: {}",
                            save_path,
                            e
                        );
                        return Task::ready(());
                    }
                };
                let fs = this.fs.clone();
                cx.spawn(async move |this, cx| {
                    // Windows atomic_write 는 path.parent() 에 임시파일을 만들므로
                    // 부모 디렉터리 존재 보장 필요. fs::Fs::create_dir 은 내부적으로 create_dir_all 동작.
                    if let Some(parent) = save_path.parent() {
                        if let Err(e) = fs.create_dir(parent).await {
                            log::warn!(
                                "notepad_panel: 디렉터리 생성 실패 {:?}: {}",
                                parent,
                                e
                            );
                            // 다음 디바운스에서 재시도하도록 hash 무효화.
                            this.update(cx, |this, _cx| {
                                this.last_saved_hash = None;
                            })
                            .ok();
                            return;
                        }
                    }

                    if let Err(e) = fs.atomic_write(save_path.clone(), json).await {
                        log::warn!(
                            "notepad_panel: 비동기 쓰기 실패 {:?}: {}",
                            save_path,
                            e
                        );
                        this.update(cx, |this, _cx| {
                            this.last_saved_hash = None;
                        })
                        .ok();
                        return;
                    }
                    this.update(cx, |this, _cx| {
                        this.last_saved_hash = Some(hash);
                    })
                    .ok();
                })
            });
    }

    /// pending 디바운스를 취소하고 즉시 저장.
    /// 그룹 swap·모드 마이그레이션처럼 다음 동작 전에 현재 텍스트를 확정해야 하는 시점에 사용.
    fn flush_pending_save(&mut self, cx: &App) {
        self.save_debouncer.cancel();
        self.flush_save(cx);
    }

    /// 설정 변경 핸들러 (가로 스크롤 + multi_memo 마이그레이션)
    fn handle_settings_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // get_global 의 immutable borrow 가 길어지지 않도록 값만 즉시 추출
        let (horizontal_scroll, new_multi_memo) = {
            let settings = NotepadPanelSettings::get_global(cx);
            (settings.horizontal_scroll, settings.multi_memo)
        };

        // 1) 가로 스크롤 변경
        if horizontal_scroll != self.last_horizontal_scroll {
            self.last_horizontal_scroll = horizontal_scroll;
            self.editor.update(cx, |editor, cx| {
                if horizontal_scroll {
                    editor.set_soft_wrap_mode(SoftWrap::None, cx);
                } else {
                    editor.set_soft_wrap_mode(SoftWrap::EditorWidth, cx);
                }
            });
        }

        // 2) multi_memo 변경 → 마이그레이션
        if new_multi_memo != self.multi_memo {
            if new_multi_memo {
                self.migrate_single_to_multi(window, cx);
            } else {
                self.migrate_multi_to_single(window, cx);
            }
        }
    }

    /// 워크스페이스 entity 의 cx.notify() 신호 핸들러.
    /// 멀티 모드일 때만 동작:
    ///   1) 사라진 그룹 UUID → 메모 파일 삭제
    ///   2) 활성 그룹 UUID 변경 시 옛 파일 저장 → 새 파일 로드
    fn handle_workspace_notify(
        &mut self,
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.multi_memo {
            return;
        }

        // 빠른 가드: 그룹 수와 활성 UUID 둘 다 변하지 않았으면 HashSet 구축·diff 단계 skip.
        // workspace.cx.notify() 는 탭 전환·포커스 등으로 자주 발생하므로 핫 패스 부담 회피.
        let (group_count, active_uuid) = workspace.read_with(cx, |ws, _| {
            (ws.workspace_group_count(), ws.active_group_uuid())
        });
        if group_count == self.last_known_uuids.len() && active_uuid == self.last_active_uuid {
            return;
        }

        let current_uuids: HashSet<Uuid> =
            workspace.read_with(cx, |ws, _| ws.group_uuids().collect());

        for old_uuid in self.last_known_uuids.difference(&current_uuids) {
            let path = group_file_path(old_uuid);
            match std::fs::remove_file(&path) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => log::warn!("notepad_panel: 메모 파일 삭제 실패 {:?}: {}", path, e),
            }
        }

        if active_uuid != self.last_active_uuid {
            // 옛 활성 파일이 잘못된 키로 덮어쓰이지 않도록 active_uuid 변경 직전 확정.
            self.flush_pending_save(cx);
            self.last_active_uuid = active_uuid;

            let new_content = active_uuid.map(Self::load_for_uuid).unwrap_or_default();
            self.set_text_suppressed(new_content, window, cx);
        }

        self.last_known_uuids = current_uuids;
    }

    /// 단일 → 멀티 모드 마이그레이션.
    /// 기존 단일 메모를 첫 번째 그룹의 파일로 이전한 뒤 활성 그룹 콘텐츠로 표시 갱신.
    fn migrate_single_to_multi(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 다음 단계에서 단일 콘텐츠를 복사하기 전에 디스크의 단일 파일을 최신화.
        self.flush_pending_save(cx);

        let Some(workspace) = self.workspace.upgrade() else {
            log::warn!(
                "notepad_panel: migrate_single_to_multi 워크스페이스 upgrade 실패, 모드만 갱신"
            );
            self.multi_memo = true;
            self.last_active_uuid = None;
            self.last_known_uuids.clear();
            return;
        };
        let (first_uuid, active_uuid, all_uuids) = workspace.read_with(cx, |ws, _| {
            let first = ws.workspace_groups().first().map(|g| g.uuid);
            let active = ws.active_group_uuid();
            let all: HashSet<Uuid> = ws.group_uuids().collect();
            (first, active, all)
        });

        let Some(first_uuid) = first_uuid else {
            log::warn!("notepad_panel: 첫 번째 워크스페이스 그룹 없음, 모드만 갱신");
            self.multi_memo = true;
            self.last_active_uuid = None;
            self.last_known_uuids.clear();
            return;
        };

        // 단일 콘텐츠는 방금 flush 로 디스크와 동기화된 에디터 메모리에서 직접 사용.
        // (디스크 read 는 자기가 방금 쓴 내용을 다시 읽는 낭비)
        let single_content = self.editor.read(cx).text(cx);
        Self::write_to_file(&group_file_path(&first_uuid), &single_content);

        self.multi_memo = true;
        self.last_active_uuid = active_uuid;
        self.last_known_uuids = all_uuids;

        // 활성 그룹이 첫 번째 그룹과 다르면 활성 그룹 콘텐츠로 표시 갱신.
        if active_uuid != Some(first_uuid) {
            let active_content = active_uuid.map(Self::load_for_uuid).unwrap_or_default();
            self.set_text_suppressed(active_content, window, cx);
        }
    }

    /// 멀티 → 단일 모드 마이그레이션.
    /// 첫 번째 그룹 메모를 단일 메모로 보존하고 나머지 그룹 메모 파일은 모두 삭제.
    fn migrate_multi_to_single(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 다음 단계에서 첫 번째 그룹 파일을 단일 파일로 옮기기 전에 활성 그룹 파일을 최신화.
        self.flush_pending_save(cx);

        let first_uuid = self
            .workspace
            .upgrade()
            .and_then(|ws| ws.read_with(cx, |ws, _| ws.workspace_groups().first().map(|g| g.uuid)));

        let first_content = first_uuid.map(Self::load_for_uuid).unwrap_or_default();
        Self::write_to_file(&single_save_path(), &first_content);

        // 사용자 명시 동작: notepad/ 디렉터리 통째로 삭제.
        let dir = multi_dir();
        match std::fs::remove_dir_all(&dir) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("notepad_panel: 멀티 디렉터리 삭제 실패 {:?}: {}", dir, e),
        }

        self.multi_memo = false;
        self.last_active_uuid = None;
        self.last_known_uuids.clear();

        self.set_text_suppressed(first_content, window, cx);
    }

    /// 비동기 로드
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, window, cx| {
            cx.new(|cx| NotepadPanel::new(workspace, window, cx))
        })
    }

    /// 에디터의 현재 선택 텍스트를 결합해 반환. 공백뿐이거나 선택이 없으면 None.
    /// 다중 커서 선택은 개행으로 이어 붙인다.
    fn selected_text(editor: &Entity<Editor>, cx: &mut App) -> Option<String> {
        editor.update(cx, |editor, cx| {
            let snapshot = editor.display_snapshot(cx);
            let selections = editor.selections.all::<MultiBufferOffset>(&snapshot);
            let buffer = editor.buffer().read(cx).read(cx);
            let mut parts: Vec<String> = Vec::new();
            for selection in selections.iter() {
                if selection.start == selection.end {
                    continue;
                }
                let text: String = buffer
                    .text_for_range(selection.start..selection.end)
                    .collect();
                if !text.is_empty() {
                    parts.push(text);
                }
            }
            if parts.is_empty() {
                return None;
            }
            let joined = parts.join("\n");
            if joined.trim().is_empty() {
                None
            } else {
                Some(joined)
            }
        })
    }

    /// 워크스페이스에 존재하는 모든 터미널 탭을 수집.
    /// - Dock의 TerminalPanel 내부 pane들
    /// - 중앙 pane에 열린 터미널 (workspace.items_of_type)
    /// 반환 튜플은 (메뉴에 표시할 라벨, TerminalView 엔티티). 동일 라벨이 중복되면 `(N)` 접미사 부여.
    fn collect_terminals(
        workspace: &Workspace,
        cx: &App,
    ) -> Vec<(SharedString, Entity<TerminalView>)> {
        let mut base_names: Vec<(SharedString, Entity<TerminalView>)> = Vec::new();
        let mut seen: std::collections::HashSet<gpui::EntityId> = std::collections::HashSet::new();

        // 1) TerminalPanel (dock) 내부 pane 순회
        if let Some(terminal_panel) = workspace.panel::<TerminalPanel>(cx) {
            let terminal_panel = terminal_panel.read(cx);
            for pane in terminal_panel.panes() {
                for item in pane.read(cx).items() {
                    let Some(terminal_view) = item.downcast::<TerminalView>() else {
                        continue;
                    };
                    if !seen.insert(terminal_view.entity_id()) {
                        continue;
                    }
                    let label = terminal_view.read(cx).tab_content_text(0, cx);
                    base_names.push((label, terminal_view));
                }
            }
        }

        // 2) 중앙 pane에 열린 TerminalView 순회
        for terminal_view in workspace.items_of_type::<TerminalView>(cx) {
            if !seen.insert(terminal_view.entity_id()) {
                continue;
            }
            let label = terminal_view.read(cx).tab_content_text(0, cx);
            base_names.push((label, terminal_view));
        }

        // 같은 라벨이 여러 개면 등장 순서대로 `(2)`, `(3)` 접미사를 붙여 구분한다.
        let mut total_counts: HashMap<SharedString, u32> = HashMap::new();
        for (label, _) in base_names.iter() {
            *total_counts.entry(label.clone()).or_insert(0) += 1;
        }
        let mut running_counts: HashMap<SharedString, u32> = HashMap::new();
        let mut result: Vec<(SharedString, Entity<TerminalView>)> = Vec::with_capacity(base_names.len());
        for (label, view) in base_names.into_iter() {
            let total = total_counts.get(&label).copied().unwrap_or(1);
            if total <= 1 {
                result.push((label, view));
            } else {
                let n = running_counts.entry(label.clone()).or_insert(0);
                *n += 1;
                let numbered = SharedString::from(format!("{} ({})", label, *n));
                result.push((numbered, view));
            }
        }
        result
    }

    /// 메모장 전용 우클릭 컨텍스트 메뉴를 구성해 NotepadPanel의 `context_menu` 필드에 저장한다.
    /// render에서 deferred anchored로 그려진다.
    /// - 선택 텍스트가 공백뿐이거나 비어있으면 메뉴를 띄우지 않는다.
    /// - 현재 워크스페이스의 TerminalPanel에 있는 모든 터미널 탭을 항목으로 노출한다.
    /// - 터미널 탭이 하나도 없으면 메뉴를 띄우지 않는다.
    ///
    /// 메뉴를 실제로 띄웠으면 `true`를 반환한다. 호출 측은 이 값으로 이벤트 전파 차단 여부를 결정한다.
    fn deploy_terminal_send_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let editor = self.editor.clone();
        // 선택 텍스트는 터미널 섹션용. 없어도 편집 섹션은 표시하므로 여기서 return하지 않는다.
        let selected_text = Self::selected_text(&editor, cx);

        let terminals = self
            .workspace
            .upgrade()
            .map(|workspace| {
                workspace.read_with(cx, |workspace, cx| Self::collect_terminals(workspace, cx))
            })
            .unwrap_or_default();

        // 메뉴 라벨 준비 (i18n): 편집 메뉴 3종 + 터미널 입력 접미사
        let copy_label: SharedString = t("notepad_panel.context_menu.copy", cx).into();
        let cut_label: SharedString = t("notepad_panel.context_menu.cut", cx).into();
        let paste_label: SharedString = t("notepad_panel.context_menu.paste", cx).into();
        let send_suffix = t("notepad_panel.context_menu.send_suffix", cx).to_string();

        // action의 키 바인딩이 Editor 키맵 컨텍스트에서 조회되도록 editor의 focus handle을 action_context로 지정.
        let editor_focus = editor.focus_handle(cx);
        let editor_for_menu = editor.clone();
        let menu = ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
            menu = menu.context(editor_focus);
            // 편집 섹션: 복사 / 잘라내기 / 붙여넣기 (action 바인딩으로 단축키 표시)
            menu = menu.entry(copy_label, Some(Box::new(Copy)), {
                let editor = editor_for_menu.clone();
                move |window, cx| {
                    editor.update(cx, |editor, cx| editor.copy(&Copy, window, cx));
                }
            });
            menu = menu.entry(cut_label, Some(Box::new(Cut)), {
                let editor = editor_for_menu.clone();
                move |window, cx| {
                    editor.update(cx, |editor, cx| editor.cut(&Cut, window, cx));
                }
            });
            menu = menu.entry(paste_label, Some(Box::new(Paste)), {
                let editor = editor_for_menu.clone();
                move |window, cx| {
                    editor.update(cx, |editor, cx| editor.paste(&Paste, window, cx));
                }
            });

            // 터미널 섹션: 선택 텍스트가 있고 터미널 탭이 1개 이상일 때만 구분선 + 엔트리 추가.
            if let Some(text) = selected_text.as_ref() {
                if !terminals.is_empty() {
                    menu = menu.separator();
                    for (label, terminal_view) in terminals.iter().cloned() {
                        let text = text.clone();
                        let entry_label =
                            SharedString::from(format!("{} {}", label, send_suffix));
                        menu = menu.entry(entry_label, None, move |_window, cx| {
                            terminal_view.update(cx, |view, cx| {
                                view.terminal().update(cx, |terminal, _cx| {
                                    terminal.paste(&text);
                                });
                            });
                        });
                    }
                }
            }
            menu
        });

        // 메뉴에 포커스 이동 → blur 시 dismiss.
        window.focus(&menu.focus_handle(cx), cx);
        // 메뉴 dismiss 이벤트 구독 → 상태 정리.
        let subscription = cx.subscribe(&menu, |this, _, _: &DismissEvent, cx| {
            this.context_menu.take();
            cx.notify();
        });
        self.context_menu = Some((menu, position, subscription));
        cx.notify();
        true
    }
}

impl Focusable for NotepadPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.focus_handle(cx)
    }
}

impl EventEmitter<PanelEvent> for NotepadPanel {}

impl Render for NotepadPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("notepad-panel")
            .key_context("NotepadPanel")
            .track_focus(&self.editor.focus_handle(cx))
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().colors().panel_background)
            // 헤더
            .child(
                div()
                    .flex()
                    .items_center()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Label::new(t("notepad_panel.title", cx))
                            .size(LabelSize::Small)
                            .color(Color::Default),
                    ),
            )
            // 에디터 영역 — 우클릭은 capture phase에서 가로채어 터미널 전송 메뉴를 띄운다.
            // Editor가 bubble phase에서 자체 컨텍스트 메뉴를 deploy하므로 capture로 선점하고,
            // 메뉴를 실제로 띄운 경우에만 전파를 차단한다.
            // 메뉴는 NotepadPanel이 직접 `context_menu` 필드에 소유하며 deferred anchored로 그린다.
            .child(
                div()
                    .id("notepad-editor-area")
                    .flex_1()
                    .size_full()
                    .occlude()
                    .capture_any_mouse_down(cx.listener(
                        |this, event: &MouseDownEvent, window, cx| {
                            if event.button != MouseButton::Right {
                                return;
                            }
                            if this.deploy_terminal_send_menu(event.position, window, cx) {
                                cx.stop_propagation();
                            }
                        },
                    ))
                    .child(self.editor.clone()),
            )
            .children(self.context_menu.as_ref().map(|(menu, position, _)| {
                deferred(
                    anchored()
                        .position(*position)
                        .anchor(gpui::Corner::TopLeft)
                        .child(menu.clone()),
                )
                .with_priority(1)
            }))
    }
}

impl Panel for NotepadPanel {
    fn persistent_name() -> &'static str {
        "Notepad Panel"
    }

    fn panel_key() -> &'static str {
        "NotepadPanel"
    }

    fn position(&self, _window: &Window, cx: &App) -> DockPosition {
        NotepadPanelSettings::get_global(cx).dock
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Left | DockPosition::Bottom | DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        settings::update_settings_file(self.fs.clone(), cx, move |settings, _| {
            settings
                .notepad_panel
                .get_or_insert_default()
                .dock = Some(position.into());
        });
    }

    fn default_size(&self, _window: &Window, cx: &App) -> Pixels {
        NotepadPanelSettings::get_global(cx).default_width
    }

    fn icon(&self, _window: &Window, cx: &App) -> Option<IconName> {
        Some(IconName::Notepad).filter(|_| NotepadPanelSettings::get_global(cx).button)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("notepad_panel.tooltip")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        9
    }
}
