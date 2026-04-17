// 언어 선택기
// 테마 선택기와 동일한 팝업 UI로 앱 UI 언어(한국어/영어)를 선택한다.

use fs::Fs;
use gpui::{App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, WeakEntity, Window};
use picker::{Picker, PickerDelegate};
use settings::update_settings_file;
use settings_content::Locale;
use std::sync::Arc;
use ui::{ListItem, ListItemSpacing, prelude::*, v_flex};
use util::ResultExt;
use workspace::{ModalView, Workspace, ui::HighlightedLabel, with_active_or_new_workspace};

/// 로케일 선택기 초기화 — locale_selector::Toggle 액션 핸들러를 등록한다.
pub fn init(cx: &mut App) {
    cx.on_action(|_action: &zed_actions::locale_selector::Toggle, cx| {
        with_active_or_new_workspace(cx, move |workspace, window, cx| {
            toggle_language_selector(workspace, window, cx);
        });
    });
}

fn toggle_language_selector(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let fs = workspace.app_state().fs.clone();
    workspace.toggle_modal(window, cx, |window, cx| {
        let delegate = LanguageSelectorDelegate::new(cx.entity().downgrade(), fs, cx);
        LanguageSelector::new(delegate, window, cx)
    });
}

impl ModalView for LanguageSelector {}

struct LanguageSelector {
    picker: Entity<Picker<LanguageSelectorDelegate>>,
}

impl EventEmitter<DismissEvent> for LanguageSelector {}

impl Focusable for LanguageSelector {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for LanguageSelector {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("LanguageSelector")
            .w(rems(34.))
            .child(self.picker.clone())
    }
}

impl LanguageSelector {
    pub fn new(
        delegate: LanguageSelectorDelegate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let picker = cx.new(|cx| Picker::uniform_list(delegate, window, cx));
        Self { picker }
    }
}

/// 언어 항목 — 로케일과 표시 이름을 가진다.
struct LanguageItem {
    locale: Locale,
    display_name: String,
}

struct LanguageSelectorDelegate {
    fs: Arc<dyn Fs>,
    /// 사용 가능한 언어 목록
    languages: Vec<LanguageItem>,
    /// 현재 선택된 인덱스
    selected_index: usize,
    /// 선택기를 열기 전의 원래 로케일 (취소 시 복원에 사용)
    original_locale: Locale,
    /// 선택 완료 여부 (dismiss 시 중복 복원 방지)
    selection_completed: bool,
    selector: WeakEntity<LanguageSelector>,
}

impl LanguageSelectorDelegate {
    fn new(
        selector: WeakEntity<LanguageSelector>,
        fs: Arc<dyn Fs>,
        cx: &App,
    ) -> Self {
        let original_locale = i18n::current_locale(cx);

        let languages = vec![
            LanguageItem {
                locale: Locale::System,
                display_name: i18n::t("language_selector.system", cx).to_string(),
            },
            LanguageItem {
                locale: Locale::En,
                display_name: "English".to_string(),
            },
            LanguageItem {
                locale: Locale::Ko,
                display_name: "한국어".to_string(),
            },
        ];

        let selected_index = languages
            .iter()
            .position(|l| l.locale == original_locale)
            .unwrap_or(0);

        Self {
            fs,
            languages,
            selected_index,
            original_locale,
            selection_completed: false,
            selector,
        }
    }
}

impl PickerDelegate for LanguageSelectorDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, cx: &mut App) -> Arc<str> {
        i18n::t("language_selector.placeholder", cx).into()
    }

    fn match_count(&self) -> usize {
        self.languages.len()
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        _window: &mut Window,
        cx: &mut Context<Picker<LanguageSelectorDelegate>>,
    ) {
        self.selection_completed = true;

        if let Some(item) = self.languages.get(self.selected_index) {
            let locale = item.locale;

            // i18n 글로벌에 즉시 반영
            i18n::set_locale(locale, cx);

            // settings.json에 locale 저장
            update_settings_file(self.fs.clone(), cx, move |settings, _| {
                settings.locale = Some(locale);
            });
        }

        self.selector
            .update(cx, |_, cx| cx.emit(DismissEvent))
            .ok();
    }

    fn dismissed(&mut self, _: &mut Window, cx: &mut Context<Picker<LanguageSelectorDelegate>>) {
        // 취소 시 원래 locale로 복원
        if !self.selection_completed {
            i18n::set_locale(self.original_locale, cx);
            self.selection_completed = true;
        }

        self.selector
            .update(cx, |_, cx| cx.emit(DismissEvent))
            .log_err();
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Picker<LanguageSelectorDelegate>>,
    ) {
        self.selected_index = ix;
        // 커서 이동 시 미리보기로 locale 변경
        if let Some(item) = self.languages.get(ix) {
            i18n::set_locale(item.locale, cx);
        }
    }

    fn update_matches(
        &mut self,
        _query: String,
        _window: &mut Window,
        cx: &mut Context<Picker<LanguageSelectorDelegate>>,
    ) -> gpui::Task<()> {
        // 언어 목록은 고정 — 검색 없이 전체 표시
        cx.background_executor().spawn(async {})
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let item = self.languages.get(ix)?;

        Some(
            ListItem::new(ix)
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .child(HighlightedLabel::new(item.display_name.clone(), Vec::new())),
        )
    }
}
