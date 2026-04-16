mod preview;

use agent_settings::AgentSettings;
use editor::actions::{
    AddSelectionAbove, AddSelectionBelow, CodeActionSource, DuplicateLineDown, GoToDiagnostic,
    GoToHunk, GoToPreviousDiagnostic, GoToPreviousHunk, MoveLineDown, MoveLineUp, SelectAll,
    SelectLargerSyntaxNode, SelectNext, SelectSmallerSyntaxNode, ToggleCodeActions,
    ToggleDiagnostics, ToggleGoToLine, ToggleInlineDiagnostics,
};
use editor::code_context_menus::{CodeContextMenu, ContextMenuOrigin};
use i18n::t;
use editor::{Editor, EditorSettings};
use gpui::{
    Action, AnchoredPositionMode, ClickEvent, Context, Corner, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, ParentElement, Render, Styled, Subscription,
    WeakEntity, Window, anchored, deferred, point,
};
use project::project_settings::DiagnosticSeverity;
use search::{BufferSearchBar, buffer_search};
use settings::{Settings, SettingsStore};
use ui::{
    ButtonStyle, ContextMenu, ContextMenuEntry, DocumentationSide, IconButton, IconName, IconSize,
    PopoverMenu, PopoverMenuHandle, Tooltip, prelude::*,
};
use workspace::item::ItemBufferKind;
use workspace::{
    ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView, Workspace, item::ItemHandle,
};
use zed_actions::{assistant::InlineAssist, outline::ToggleOutline};

const MAX_CODE_ACTION_MENU_LINES: u32 = 16;

pub struct QuickActionBar {
    _inlay_hints_enabled_subscription: Option<Subscription>,
    _ai_settings_subscription: Subscription,
    active_item: Option<Box<dyn ItemHandle>>,
    buffer_search_bar: Entity<BufferSearchBar>,
    show: bool,
    toggle_selections_handle: PopoverMenuHandle<ContextMenu>,
    toggle_settings_handle: PopoverMenuHandle<ContextMenu>,
    workspace: WeakEntity<Workspace>,
}

impl QuickActionBar {
    pub fn new(
        buffer_search_bar: Entity<BufferSearchBar>,
        workspace: &Workspace,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut was_agent_enabled = AgentSettings::get_global(cx).enabled(cx);
        let mut was_agent_button = AgentSettings::get_global(cx).button;

        let ai_settings_subscription = cx.observe_global::<SettingsStore>(move |_, cx| {
            let agent_settings = AgentSettings::get_global(cx);
            let is_agent_enabled = agent_settings.enabled(cx);

            if was_agent_enabled != is_agent_enabled || was_agent_button != agent_settings.button {
                was_agent_enabled = is_agent_enabled;
                was_agent_button = agent_settings.button;
                cx.notify();
            }
        });

        let mut this = Self {
            _inlay_hints_enabled_subscription: None,
            _ai_settings_subscription: ai_settings_subscription,
            active_item: None,
            buffer_search_bar,
            show: true,
            toggle_selections_handle: Default::default(),
            toggle_settings_handle: Default::default(),
            workspace: workspace.weak_handle(),
        };
        this.apply_settings(cx);
        cx.observe_global::<SettingsStore>(|this, cx| this.apply_settings(cx))
            .detach();
        this
    }

    fn active_editor(&self) -> Option<Entity<Editor>> {
        self.active_item
            .as_ref()
            .and_then(|item| item.downcast::<Editor>())
    }

    fn apply_settings(&mut self, cx: &mut Context<Self>) {
        let new_show = EditorSettings::get_global(cx).toolbar.quick_actions;
        if new_show != self.show {
            self.show = new_show;
            cx.emit(ToolbarItemEvent::ChangeLocation(
                self.get_toolbar_item_location(),
            ));
        }
    }

    fn get_toolbar_item_location(&self) -> ToolbarItemLocation {
        if self.show && self.active_editor().is_some() {
            ToolbarItemLocation::PrimaryRight
        } else {
            ToolbarItemLocation::Hidden
        }
    }
}

impl Render for QuickActionBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.active_editor() else {
            return div().id("empty quick action bar");
        };

        let supports_inlay_hints = editor.update(cx, |editor, cx| editor.supports_inlay_hints(cx));
        let supports_semantic_tokens =
            editor.update(cx, |editor, cx| editor.supports_semantic_tokens(cx));
        let editor_value = editor.read(cx);
        let selection_menu_enabled = editor_value.selection_menu_enabled(cx);
        let inlay_hints_enabled = editor_value.inlay_hints_enabled();
        let semantic_highlights_enabled = editor_value.semantic_highlights_enabled();
        let is_full = editor_value.mode().is_full();
        let diagnostics_enabled = editor_value.diagnostics_max_severity != DiagnosticSeverity::Off;
        let supports_inline_diagnostics = editor_value.inline_diagnostics_enabled();
        let inline_diagnostics_enabled = editor_value.show_inline_diagnostics();
        let git_blame_inline_enabled = editor_value.git_blame_inline_enabled();
        let show_git_blame_gutter = editor_value.show_git_blame_gutter();
        let auto_signature_help_enabled = editor_value.auto_signature_help_enabled(cx);
        let show_line_numbers = editor_value.line_numbers_enabled(cx);
        let has_edit_prediction_provider = editor_value.edit_prediction_provider().is_some();
        let show_edit_predictions = editor_value.edit_predictions_enabled();
        let edit_predictions_enabled_at_cursor =
            editor_value.edit_predictions_enabled_at_cursor(cx);
        let supports_minimap = editor_value.supports_minimap(cx);
        let minimap_enabled = supports_minimap && editor_value.minimap().is_some();
        let has_available_code_actions = editor_value.has_available_code_actions();
        let code_action_enabled = editor_value.code_actions_enabled_for_toolbar(cx);
        let focus_handle = editor_value.focus_handle(cx);

        let search_button = (editor.buffer_kind(cx) == ItemBufferKind::Singleton).then(|| {
            QuickActionBarButton::new(
                "toggle buffer search",
                search::SEARCH_ICON,
                !self.buffer_search_bar.read(cx).is_dismissed(),
                Box::new(buffer_search::Deploy::find()),
                focus_handle.clone(),
                "Buffer Search",
                {
                    let buffer_search_bar = self.buffer_search_bar.clone();
                    move |_, window, cx| {
                        buffer_search_bar.update(cx, |search_bar, cx| {
                            search_bar.toggle(&buffer_search::Deploy::find(), window, cx)
                        });
                    }
                },
            )
        });

        let assistant_button = QuickActionBarButton::new(
            "toggle inline assistant",
            IconName::DokkaebiAssistant,
            false,
            Box::new(InlineAssist::default()),
            focus_handle,
            "Inline Assist",
            move |_, window, cx| {
                window.dispatch_action(Box::new(InlineAssist::default()), cx);
            },
        );

        let code_actions_dropdown = code_action_enabled.then(|| {
            let focus = editor.focus_handle(cx);
            let is_deployed = {
                let menu_ref = editor.read(cx).context_menu().borrow();
                let code_action_menu = menu_ref
                    .as_ref()
                    .filter(|menu| matches!(menu, CodeContextMenu::CodeActions(..)));
                code_action_menu
                    .as_ref()
                    .is_some_and(|menu| matches!(menu.origin(), ContextMenuOrigin::QuickActionBar))
            };
            let code_action_element = is_deployed
                .then(|| {
                    editor.update(cx, |editor, cx| {
                        editor.render_context_menu(MAX_CODE_ACTION_MENU_LINES, window, cx)
                    })
                })
                .flatten();
            v_flex()
                .child(
                    IconButton::new("toggle_code_actions_icon", IconName::BoltOutlined)
                        .icon_size(IconSize::Small)
                        .style(ButtonStyle::Subtle)
                        .disabled(!has_available_code_actions)
                        .toggle_state(is_deployed)
                        .when(!is_deployed, |this| {
                            this.when(has_available_code_actions, |this| {
                                this.tooltip(Tooltip::for_action_title(
                                    "Code Actions",
                                    &ToggleCodeActions::default(),
                                ))
                            })
                            .when(
                                !has_available_code_actions,
                                |this| {
                                    this.tooltip(Tooltip::for_action_title(
                                        "No Code Actions Available",
                                        &ToggleCodeActions::default(),
                                    ))
                                },
                            )
                        })
                        .on_click({
                            let focus = focus;
                            move |_, window, cx| {
                                focus.dispatch_action(
                                    &ToggleCodeActions {
                                        deployed_from: Some(CodeActionSource::QuickActionBar),
                                        quick_launch: false,
                                    },
                                    window,
                                    cx,
                                );
                            }
                        }),
                )
                .children(code_action_element.map(|menu| {
                    deferred(
                        anchored()
                            .position_mode(AnchoredPositionMode::Local)
                            .position(point(px(20.), px(20.)))
                            .anchor(Corner::TopRight)
                            .child(menu),
                    )
                }))
        });

        let editor_selections_dropdown = selection_menu_enabled.then(|| {
            let has_diff_hunks = editor
                .read(cx)
                .buffer()
                .read(cx)
                .snapshot(cx)
                .has_diff_hunks();
            let focus = editor.focus_handle(cx);

            PopoverMenu::new("editor-selections-dropdown")
                .trigger_with_tooltip(
                    IconButton::new("toggle_editor_selections_icon", IconName::CursorIBeam)
                        .icon_size(IconSize::Small)
                        .style(ButtonStyle::Subtle)
                        .toggle_state(self.toggle_selections_handle.is_deployed()),
                    Tooltip::text(t("editor.toolbar.selection_controls", cx)),
                )
                .with_handle(self.toggle_selections_handle.clone())
                .anchor(Corner::TopRight)
                .menu(move |window, cx| {
                    let focus = focus.clone();
                    // i18n 레이블 미리 생성 (내부 클로저에서 cx 접근 불가)
                    let l_select_all = t("menu.selection.select_all", cx);
                    let l_select_next = t("menu.selection.select_next_occurrence", cx);
                    let l_expand = t("menu.selection.expand_selection", cx);
                    let l_shrink = t("menu.selection.shrink_selection", cx);
                    let l_cursor_above = t("menu.selection.add_cursor_above", cx);
                    let l_cursor_below = t("menu.selection.add_cursor_below", cx);
                    let l_go_to_symbol = t("editor.toolbar.go_to_symbol", cx);
                    let l_go_to_line = t("editor.toolbar.go_to_line_column", cx);
                    let l_next_problem = t("menu.go.next_problem", cx);
                    let l_prev_problem = t("menu.go.previous_problem", cx);
                    let l_next_hunk = t("editor.toolbar.next_hunk", cx);
                    let l_prev_hunk = t("editor.toolbar.previous_hunk", cx);
                    let l_move_up = t("menu.selection.move_line_up", cx);
                    let l_move_down = t("menu.selection.move_line_down", cx);
                    let l_duplicate = t("menu.selection.duplicate_selection", cx);
                    let menu = ContextMenu::build(window, cx, move |menu, _, _| {
                        menu.context(focus.clone())
                            .action(l_select_all, Box::new(SelectAll))
                            .action(
                                l_select_next,
                                Box::new(SelectNext {
                                    replace_newest: false,
                                }),
                            )
                            .action(l_expand, Box::new(SelectLargerSyntaxNode))
                            .action(l_shrink, Box::new(SelectSmallerSyntaxNode))
                            .action(
                                l_cursor_above,
                                Box::new(AddSelectionAbove {
                                    skip_soft_wrap: true,
                                }),
                            )
                            .action(
                                l_cursor_below,
                                Box::new(AddSelectionBelow {
                                    skip_soft_wrap: true,
                                }),
                            )
                            .separator()
                            .action(l_go_to_symbol, Box::new(ToggleOutline))
                            .action(l_go_to_line, Box::new(ToggleGoToLine))
                            .separator()
                            .action(l_next_problem, Box::new(GoToDiagnostic::default()))
                            .action(
                                l_prev_problem,
                                Box::new(GoToPreviousDiagnostic::default()),
                            )
                            .separator()
                            .action_disabled_when(!has_diff_hunks, l_next_hunk, Box::new(GoToHunk))
                            .action_disabled_when(
                                !has_diff_hunks,
                                l_prev_hunk,
                                Box::new(GoToPreviousHunk),
                            )
                            .separator()
                            .action(l_move_up, Box::new(MoveLineUp))
                            .action(l_move_down, Box::new(MoveLineDown))
                            .action(l_duplicate, Box::new(DuplicateLineDown))
                    });
                    Some(menu)
                })
        });

        let editor_focus_handle = editor.focus_handle(cx);
        let editor = editor.downgrade();
        let editor_settings_dropdown = {
            PopoverMenu::new("editor-settings")
                .trigger_with_tooltip(
                    IconButton::new("toggle_editor_settings_icon", IconName::Sliders)
                        .icon_size(IconSize::Small)
                        .style(ButtonStyle::Subtle)
                        .toggle_state(self.toggle_settings_handle.is_deployed()),
                    Tooltip::text(t("editor.toolbar.editor_controls", cx)),
                )
                .anchor(Corner::TopRight)
                .with_handle(self.toggle_settings_handle.clone())
                .menu(move |window, cx| {
                    // i18n 레이블 미리 생성 (내부 클로저에서 cx 접근 불가)
                    let l_inlay_hints = t("editor.toolbar.inlay_hints", cx);
                    let l_semantic_highlights = t("editor.toolbar.semantic_highlights", cx);
                    let l_minimap = t("editor.toolbar.minimap", cx);
                    let l_edit_predictions = t("editor.toolbar.edit_predictions", cx);
                    let l_edit_predictions_disabled = t("editor.toolbar.edit_predictions_disabled", cx);
                    let l_diagnostics = t("editor.toolbar.diagnostics", cx);
                    let l_inline_diagnostics = t("editor.toolbar.inline_diagnostics", cx);
                    let l_inline_diagnostics_disabled = t("editor.toolbar.inline_diagnostics_disabled", cx);
                    let l_line_numbers = t("editor.toolbar.line_numbers", cx);
                    let l_selection_menu = t("editor.toolbar.selection_menu", cx);
                    let l_auto_signature_help = t("editor.toolbar.auto_signature_help", cx);
                    let l_inline_git_blame = t("editor.toolbar.inline_git_blame", cx);
                    let l_column_git_blame = t("editor.toolbar.column_git_blame", cx);
                    let menu = ContextMenu::build(window, cx, {
                        let focus_handle = editor_focus_handle.clone();
                        |mut menu, _, _| {
                            menu = menu.context(focus_handle);

                            if supports_inlay_hints {
                                menu = menu.toggleable_entry(
                                    l_inlay_hints,
                                    inlay_hints_enabled,
                                    IconPosition::Start,
                                    Some(editor::actions::ToggleInlayHints.boxed_clone()),
                                    {
                                        let editor = editor.clone();
                                        move |window, cx| {
                                            editor
                                                .update(cx, |editor, cx| {
                                                    editor.toggle_inlay_hints(
                                                        &editor::actions::ToggleInlayHints,
                                                        window,
                                                        cx,
                                                    );
                                                })
                                                .ok();
                                        }
                                    },
                                );

                            }

                            if supports_semantic_tokens {
                                menu = menu.toggleable_entry(
                                    l_semantic_highlights,
                                    semantic_highlights_enabled,
                                    IconPosition::Start,
                                    Some(editor::actions::ToggleSemanticHighlights.boxed_clone()),
                                    {
                                        let editor = editor.clone();
                                        move |window, cx| {
                                            editor
                                                .update(cx, |editor, cx| {
                                                    editor.toggle_semantic_highlights(
                                                        &editor::actions::ToggleSemanticHighlights,
                                                        window,
                                                        cx,
                                                    );
                                                })
                                                .ok();
                                        }
                                    },
                                );
                            }

                            if supports_minimap {
                                menu = menu.toggleable_entry(l_minimap, minimap_enabled, IconPosition::Start, Some(editor::actions::ToggleMinimap.boxed_clone()), {
                                    let editor = editor.clone();
                                    move |window, cx| {
                                        editor
                                            .update(cx, |editor, cx| {
                                                editor.toggle_minimap(
                                                    &editor::actions::ToggleMinimap,
                                                    window,
                                                    cx,
                                                );
                                            })
                                            .ok();
                                    }
                                },)
                            }

                            if has_edit_prediction_provider {
                                let mut edit_prediction_entry = ContextMenuEntry::new(l_edit_predictions)
                                    .toggleable(IconPosition::Start, edit_predictions_enabled_at_cursor && show_edit_predictions)
                                    .disabled(!edit_predictions_enabled_at_cursor)
                                    .action(
                                        editor::actions::ToggleEditPrediction.boxed_clone(),
                                    ).handler({
                                        let editor = editor.clone();
                                        move |window, cx| {
                                            editor
                                                .update(cx, |editor, cx| {
                                                    editor.toggle_edit_predictions(
                                                        &editor::actions::ToggleEditPrediction,
                                                        window,
                                                        cx,
                                                    );
                                                })
                                                .ok();
                                        }
                                    });
                                if !edit_predictions_enabled_at_cursor {
                                    edit_prediction_entry = edit_prediction_entry.documentation_aside(DocumentationSide::Left, {
                                        let msg = l_edit_predictions_disabled.clone();
                                        move |_| Label::new(msg.clone()).into_any_element()
                                    });
                                }

                                menu = menu.item(edit_prediction_entry);
                            }

                            menu = menu.separator();

                            if is_full {
                                menu = menu.toggleable_entry(
                                    l_diagnostics,
                                    diagnostics_enabled,
                                    IconPosition::Start,
                                    Some(ToggleDiagnostics.boxed_clone()),
                                    {
                                        let editor = editor.clone();
                                        move |window, cx| {
                                            editor
                                                .update(cx, |editor, cx| {
                                                    editor.toggle_diagnostics(
                                                        &ToggleDiagnostics,
                                                        window,
                                                        cx,
                                                    );
                                                })
                                                .ok();
                                        }
                                    },
                                );

                                if supports_inline_diagnostics {
                                    let mut inline_diagnostics_item = ContextMenuEntry::new(l_inline_diagnostics)
                                        .toggleable(IconPosition::Start, diagnostics_enabled && inline_diagnostics_enabled)
                                        .action(ToggleInlineDiagnostics.boxed_clone())
                                        .handler({
                                            let editor = editor.clone();
                                            move |window, cx| {
                                                editor
                                                    .update(cx, |editor, cx| {
                                                        editor.toggle_inline_diagnostics(
                                                            &ToggleInlineDiagnostics,
                                                            window,
                                                            cx,
                                                        );
                                                    })
                                                    .ok();
                                            }
                                        });
                                    if !diagnostics_enabled {
                                        inline_diagnostics_item = inline_diagnostics_item.disabled(true).documentation_aside(DocumentationSide::Left, {
                                            let msg = l_inline_diagnostics_disabled.clone();
                                            move |_| Label::new(msg.clone()).into_any_element()
                                        });
                                    }
                                    menu = menu.item(inline_diagnostics_item)
                                }

                                menu = menu.separator();
                            }

                            menu = menu.toggleable_entry(
                                l_line_numbers,
                                show_line_numbers,
                                IconPosition::Start,
                                Some(editor::actions::ToggleLineNumbers.boxed_clone()),
                                {
                                    let editor = editor.clone();
                                    move |window, cx| {
                                        editor
                                            .update(cx, |editor, cx| {
                                                editor.toggle_line_numbers(
                                                    &editor::actions::ToggleLineNumbers,
                                                    window,
                                                    cx,
                                                );
                                            })
                                            .ok();
                                    }
                                },
                            );

                            menu = menu.toggleable_entry(
                                l_selection_menu,
                                selection_menu_enabled,
                                IconPosition::Start,
                                Some(editor::actions::ToggleSelectionMenu.boxed_clone()),
                                {
                                    let editor = editor.clone();
                                    move |window, cx| {
                                        editor
                                            .update(cx, |editor, cx| {
                                                editor.toggle_selection_menu(
                                                    &editor::actions::ToggleSelectionMenu,
                                                    window,
                                                    cx,
                                                )
                                            })
                                            .ok();
                                    }
                                },
                            );

                            menu = menu.toggleable_entry(
                                l_auto_signature_help,
                                auto_signature_help_enabled,
                                IconPosition::Start,
                                Some(editor::actions::ToggleAutoSignatureHelp.boxed_clone()),
                                {
                                    let editor = editor.clone();
                                    move |window, cx| {
                                        editor
                                            .update(cx, |editor, cx| {
                                                editor.toggle_auto_signature_help_menu(
                                                    &editor::actions::ToggleAutoSignatureHelp,
                                                    window,
                                                    cx,
                                                );
                                            })
                                            .ok();
                                    }
                                },
                            );

                            menu = menu.separator();

                            menu = menu.toggleable_entry(
                                l_inline_git_blame,
                                git_blame_inline_enabled,
                                IconPosition::Start,
                                Some(editor::actions::ToggleGitBlameInline.boxed_clone()),
                                {
                                    let editor = editor.clone();
                                    move |window, cx| {
                                        editor
                                            .update(cx, |editor, cx| {
                                                editor.toggle_git_blame_inline(
                                                    &editor::actions::ToggleGitBlameInline,
                                                    window,
                                                    cx,
                                                )
                                            })
                                            .ok();
                                    }
                                },
                            );

                            menu = menu.toggleable_entry(
                                l_column_git_blame,
                                show_git_blame_gutter,
                                IconPosition::Start,
                                Some(git::Blame.boxed_clone()),
                                {
                                    let editor = editor.clone();
                                    move |window, cx| {
                                        editor
                                            .update(cx, |editor, cx| {
                                                editor.toggle_git_blame(
                                                    &git::Blame,
                                                    window,
                                                    cx,
                                                )
                                            })
                                            .ok();
                                    }
                                },
                            );

                            menu
                        }
                    });
                    Some(menu)
                })
        };

        h_flex()
            .id("quick action bar")
            .gap(DynamicSpacing::Base01.rems(cx))
            .children(self.render_preview_button(self.workspace.clone(), cx))
            .children(search_button)
            .when(
                AgentSettings::get_global(cx).enabled(cx) && AgentSettings::get_global(cx).button,
                |bar| bar.child(assistant_button),
            )
            .children(code_actions_dropdown)
            .children(editor_selections_dropdown)
            .child(editor_settings_dropdown)
    }
}

impl EventEmitter<ToolbarItemEvent> for QuickActionBar {}

#[derive(IntoElement)]
struct QuickActionBarButton {
    id: ElementId,
    icon: IconName,
    toggled: bool,
    action: Box<dyn Action>,
    focus_handle: FocusHandle,
    tooltip: SharedString,
    on_click: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
}

impl QuickActionBarButton {
    fn new(
        id: impl Into<ElementId>,
        icon: IconName,
        toggled: bool,
        action: Box<dyn Action>,
        focus_handle: FocusHandle,
        tooltip: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            toggled,
            action,
            focus_handle,
            tooltip: tooltip.into(),
            on_click: Box::new(on_click),
        }
    }
}

impl RenderOnce for QuickActionBarButton {
    fn render(self, _window: &mut Window, _: &mut App) -> impl IntoElement {
        let tooltip = self.tooltip.clone();
        let action = self.action.boxed_clone();

        IconButton::new(self.id.clone(), self.icon)
            .icon_size(IconSize::Small)
            .style(ButtonStyle::Subtle)
            .toggle_state(self.toggled)
            .tooltip(move |_window, cx| {
                Tooltip::for_action_in(tooltip.clone(), &*action, &self.focus_handle, cx)
            })
            .on_click(move |event, window, cx| (self.on_click)(event, window, cx))
    }
}

impl ToolbarItemView for QuickActionBar {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarItemLocation {
        self.active_item = active_pane_item.map(ItemHandle::boxed_clone);
        if let Some(active_item) = active_pane_item {
            self._inlay_hints_enabled_subscription.take();

            if let Some(editor) = active_item.downcast::<Editor>() {
                let (
                    mut inlay_hints_enabled,
                    mut supports_inlay_hints,
                    mut supports_semantic_tokens,
                ) = editor.update(cx, |editor, cx| {
                    (
                        editor.inlay_hints_enabled(),
                        editor.supports_inlay_hints(cx),
                        editor.supports_semantic_tokens(cx),
                    )
                });
                self._inlay_hints_enabled_subscription =
                    Some(cx.observe(&editor, move |_, editor, cx| {
                        let (
                            new_inlay_hints_enabled,
                            new_supports_inlay_hints,
                            new_supports_semantic_tokens,
                        ) = editor.update(cx, |editor, cx| {
                            (
                                editor.inlay_hints_enabled(),
                                editor.supports_inlay_hints(cx),
                                editor.supports_semantic_tokens(cx),
                            )
                        });
                        let should_notify = inlay_hints_enabled != new_inlay_hints_enabled
                            || supports_inlay_hints != new_supports_inlay_hints
                            || supports_semantic_tokens != new_supports_semantic_tokens;
                        inlay_hints_enabled = new_inlay_hints_enabled;
                        supports_inlay_hints = new_supports_inlay_hints;
                        supports_semantic_tokens = new_supports_semantic_tokens;
                        if should_notify {
                            cx.notify()
                        }
                    }));
            }
        }
        self.get_toolbar_item_location()
    }
}
