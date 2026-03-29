use collab_ui::collab_panel;
use gpui::{App, Menu, MenuItem, OsAction};
use i18n::t;
use release_channel::ReleaseChannel;
use terminal_view::terminal_panel;
use zed_actions::{debug_panel, dev};

pub fn app_menus(cx: &mut App) -> Vec<Menu> {
    use zed_actions::Quit;

    let mut view_items = vec![
        MenuItem::action(
            t("menu.view.zoom_in", cx),
            zed_actions::IncreaseBufferFontSize { persist: false },
        ),
        MenuItem::action(
            t("menu.view.zoom_out", cx),
            zed_actions::DecreaseBufferFontSize { persist: false },
        ),
        MenuItem::action(
            t("menu.view.reset_zoom", cx),
            zed_actions::ResetBufferFontSize { persist: false },
        ),
        MenuItem::action(
            t("menu.view.reset_all_zoom", cx),
            zed_actions::ResetAllZoom { persist: false },
        ),
        MenuItem::separator(),
        MenuItem::action(t("menu.view.toggle_left_dock", cx), workspace::ToggleLeftDock),
        MenuItem::action(t("menu.view.toggle_right_dock", cx), workspace::ToggleRightDock),
        MenuItem::action(t("menu.view.toggle_bottom_dock", cx), workspace::ToggleBottomDock),
        MenuItem::action(t("menu.view.toggle_all_docks", cx), workspace::ToggleAllDocks),
        MenuItem::submenu(Menu {
            name: t("menu.view.editor_layout", cx),
            disabled: false,
            items: vec![
                MenuItem::action(t("menu.view.split_up", cx), workspace::SplitUp::default()),
                MenuItem::action(t("menu.view.split_down", cx), workspace::SplitDown::default()),
                MenuItem::action(t("menu.view.split_left", cx), workspace::SplitLeft::default()),
                MenuItem::action(t("menu.view.split_right", cx), workspace::SplitRight::default()),
            ],
        }),
        MenuItem::separator(),
        MenuItem::action(t("menu.view.project_panel", cx), zed_actions::project_panel::ToggleFocus),
        MenuItem::action(t("menu.view.outline_panel", cx), outline_panel::ToggleFocus),
        MenuItem::action(t("menu.view.collab_panel", cx), collab_panel::ToggleFocus),
        MenuItem::action(t("menu.view.terminal_panel", cx), terminal_panel::ToggleFocus),
        MenuItem::action(t("menu.view.debugger_panel", cx), debug_panel::ToggleFocus),
        MenuItem::separator(),
        MenuItem::action(t("menu.view.diagnostics", cx), diagnostics::Deploy),
        MenuItem::separator(),
    ];

    if ReleaseChannel::try_global(cx) == Some(ReleaseChannel::Dev) {
        view_items.push(MenuItem::action(
            t("menu.view.toggle_gpui_inspector", cx),
            dev::ToggleInspector,
        ));
        view_items.push(MenuItem::separator());
    }

    vec![
        Menu {
            name: t("menu.zed", cx),
            disabled: false,
            items: vec![
                MenuItem::action(t("menu.zed.about", cx), zed_actions::About),
                MenuItem::action(t("menu.zed.check_for_updates", cx), auto_update::Check),
                MenuItem::separator(),
                MenuItem::submenu(Menu::new(t("menu.zed.settings", cx)).items([
                    MenuItem::action(t("menu.zed.settings.open_settings", cx), zed_actions::OpenSettings),
                    MenuItem::action(t("menu.zed.settings.open_settings_file", cx), super::OpenSettingsFile),
                    MenuItem::action(t("menu.zed.settings.open_project_settings", cx), zed_actions::OpenProjectSettings),
                    MenuItem::action(t("menu.zed.settings.open_project_settings_file", cx), super::OpenProjectSettingsFile),
                    MenuItem::action(t("menu.zed.settings.open_default_settings", cx), super::OpenDefaultSettings),
                    MenuItem::separator(),
                    MenuItem::action(t("menu.zed.settings.open_keymap", cx), zed_actions::OpenKeymap),
                    MenuItem::action(t("menu.zed.settings.open_keymap_file", cx), zed_actions::OpenKeymapFile),
                    MenuItem::action(t("menu.zed.settings.open_default_key_bindings", cx), zed_actions::OpenDefaultKeymap),
                    MenuItem::separator(),
                    MenuItem::action(
                        t("menu.zed.settings.select_theme", cx),
                        zed_actions::theme_selector::Toggle::default(),
                    ),
                    MenuItem::action(
                        t("menu.zed.settings.select_icon_theme", cx),
                        zed_actions::icon_theme_selector::Toggle::default(),
                    ),
                ])),
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::os_submenu("Services", gpui::SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action(t("menu.zed.extensions", cx), zed_actions::Extensions::default()),
                #[cfg(not(target_os = "windows"))]
                MenuItem::action(t("menu.zed.install_cli", cx), install_cli::InstallCliBinary),
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::action(t("menu.zed.hide", cx), super::Hide),
                #[cfg(target_os = "macos")]
                MenuItem::action(t("menu.zed.hide_others", cx), super::HideOthers),
                #[cfg(target_os = "macos")]
                MenuItem::action(t("menu.zed.show_all", cx), super::ShowAll),
                MenuItem::separator(),
                MenuItem::action(t("menu.zed.quit", cx), Quit),
            ],
        },
        Menu {
            name: t("menu.file", cx),
            disabled: false,
            items: vec![
                MenuItem::action(t("menu.file.new", cx), workspace::NewFile),
                MenuItem::action(t("menu.file.new_window", cx), workspace::NewWindow),
                MenuItem::separator(),
                #[cfg(not(target_os = "macos"))]
                MenuItem::action(t("menu.file.open_file", cx), workspace::OpenFiles),
                MenuItem::action(
                    if cfg!(not(target_os = "macos")) {
                        t("menu.file.open_folder", cx)
                    } else {
                        t("menu.file.open", cx)
                    },
                    workspace::Open::default(),
                ),
                MenuItem::action(
                    t("menu.file.open_recent", cx),
                    zed_actions::OpenRecent {
                        create_new_window: false,
                    },
                ),
                MenuItem::action(
                    t("menu.file.open_remote", cx),
                    zed_actions::OpenRemote {
                        create_new_window: false,
                        from_existing_connection: false,
                    },
                ),
                MenuItem::separator(),
                MenuItem::action(t("menu.file.add_folder_to_project", cx), workspace::AddFolderToProject),
                MenuItem::separator(),
                MenuItem::action(t("menu.file.save", cx), workspace::Save { save_intent: None }),
                MenuItem::action(t("menu.file.save_as", cx), workspace::SaveAs),
                MenuItem::action(t("menu.file.save_all", cx), workspace::SaveAll { save_intent: None }),
                MenuItem::separator(),
                MenuItem::action(
                    t("menu.file.close_editor", cx),
                    workspace::CloseActiveItem {
                        save_intent: None,
                        close_pinned: true,
                    },
                ),
                MenuItem::action(t("menu.file.close_project", cx), workspace::CloseProject),
                MenuItem::action(t("menu.file.close_window", cx), workspace::CloseWindow),
            ],
        },
        Menu {
            name: t("menu.edit", cx),
            disabled: false,
            items: vec![
                MenuItem::os_action(t("menu.edit.undo", cx), editor::actions::Undo, OsAction::Undo),
                MenuItem::os_action(t("menu.edit.redo", cx), editor::actions::Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action(t("menu.edit.cut", cx), editor::actions::Cut, OsAction::Cut),
                MenuItem::os_action(t("menu.edit.copy", cx), editor::actions::Copy, OsAction::Copy),
                MenuItem::action(t("menu.edit.copy_and_trim", cx), editor::actions::CopyAndTrim),
                MenuItem::os_action(t("menu.edit.paste", cx), editor::actions::Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::action(t("menu.edit.find", cx), search::buffer_search::Deploy::find()),
                MenuItem::action(t("menu.edit.find_in_project", cx), workspace::DeploySearch::find()),
                MenuItem::separator(),
                MenuItem::action(
                    t("menu.edit.toggle_line_comment", cx),
                    editor::actions::ToggleComments::default(),
                ),
            ],
        },
        Menu {
            name: t("menu.selection", cx),
            disabled: false,
            items: vec![
                MenuItem::os_action(
                    t("menu.selection.select_all", cx),
                    editor::actions::SelectAll,
                    OsAction::SelectAll,
                ),
                MenuItem::action(t("menu.selection.expand_selection", cx), editor::actions::SelectLargerSyntaxNode),
                MenuItem::action(t("menu.selection.shrink_selection", cx), editor::actions::SelectSmallerSyntaxNode),
                MenuItem::action(t("menu.selection.select_next_sibling", cx), editor::actions::SelectNextSyntaxNode),
                MenuItem::action(
                    t("menu.selection.select_previous_sibling", cx),
                    editor::actions::SelectPreviousSyntaxNode,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    t("menu.selection.add_cursor_above", cx),
                    editor::actions::AddSelectionAbove {
                        skip_soft_wrap: true,
                    },
                ),
                MenuItem::action(
                    t("menu.selection.add_cursor_below", cx),
                    editor::actions::AddSelectionBelow {
                        skip_soft_wrap: true,
                    },
                ),
                MenuItem::action(
                    t("menu.selection.select_next_occurrence", cx),
                    editor::actions::SelectNext {
                        replace_newest: false,
                    },
                ),
                MenuItem::action(
                    t("menu.selection.select_previous_occurrence", cx),
                    editor::actions::SelectPrevious {
                        replace_newest: false,
                    },
                ),
                MenuItem::action(t("menu.selection.select_all_occurrences", cx), editor::actions::SelectAllMatches),
                MenuItem::separator(),
                MenuItem::action(t("menu.selection.move_line_up", cx), editor::actions::MoveLineUp),
                MenuItem::action(t("menu.selection.move_line_down", cx), editor::actions::MoveLineDown),
                MenuItem::action(t("menu.selection.duplicate_selection", cx), editor::actions::DuplicateLineDown),
            ],
        },
        Menu {
            name: t("menu.view", cx),
            disabled: false,
            items: view_items,
        },
        Menu {
            name: t("menu.go", cx),
            disabled: false,
            items: vec![
                MenuItem::action(t("menu.go.back", cx), workspace::GoBack),
                MenuItem::action(t("menu.go.forward", cx), workspace::GoForward),
                MenuItem::separator(),
                MenuItem::action(t("menu.go.command_palette", cx), zed_actions::command_palette::Toggle),
                MenuItem::separator(),
                MenuItem::action(t("menu.go.go_to_file", cx), workspace::ToggleFileFinder::default()),
                // MenuItem::action("Go to Symbol in Project", project_symbols::Toggle),
                MenuItem::action(
                    t("menu.go.go_to_symbol_in_editor", cx),
                    zed_actions::outline::ToggleOutline,
                ),
                MenuItem::action(t("menu.go.go_to_line_column", cx), editor::actions::ToggleGoToLine),
                MenuItem::separator(),
                MenuItem::action(t("menu.go.go_to_definition", cx), editor::actions::GoToDefinition),
                MenuItem::action(t("menu.go.go_to_declaration", cx), editor::actions::GoToDeclaration),
                MenuItem::action(t("menu.go.go_to_type_definition", cx), editor::actions::GoToTypeDefinition),
                MenuItem::action(
                    t("menu.go.find_all_references", cx),
                    editor::actions::FindAllReferences::default(),
                ),
                MenuItem::separator(),
                MenuItem::action(t("menu.go.next_problem", cx), editor::actions::GoToDiagnostic::default()),
                MenuItem::action(
                    t("menu.go.previous_problem", cx),
                    editor::actions::GoToPreviousDiagnostic::default(),
                ),
            ],
        },
        Menu {
            name: t("menu.run", cx),
            disabled: false,
            items: vec![
                MenuItem::action(
                    t("menu.run.spawn_task", cx),
                    zed_actions::Spawn::ViaModal {
                        reveal_target: None,
                    },
                ),
                MenuItem::action(t("menu.run.start_debugger", cx), debugger_ui::Start),
                MenuItem::separator(),
                MenuItem::action(t("menu.run.edit_tasks_json", cx), crate::zed::OpenProjectTasks),
                MenuItem::action(t("menu.run.edit_debug_json", cx), zed_actions::OpenProjectDebugTasks),
                MenuItem::separator(),
                MenuItem::action(t("menu.run.continue", cx), debugger_ui::Continue),
                MenuItem::action(t("menu.run.step_over", cx), debugger_ui::StepOver),
                MenuItem::action(t("menu.run.step_into", cx), debugger_ui::StepInto),
                MenuItem::action(t("menu.run.step_out", cx), debugger_ui::StepOut),
                MenuItem::separator(),
                MenuItem::action(t("menu.run.toggle_breakpoint", cx), editor::actions::ToggleBreakpoint),
                MenuItem::action(t("menu.run.edit_breakpoint", cx), editor::actions::EditLogBreakpoint),
                MenuItem::action(t("menu.run.clear_all_breakpoints", cx), debugger_ui::ClearAllBreakpoints),
            ],
        },
        Menu {
            name: t("menu.window", cx),
            disabled: false,
            items: vec![
                MenuItem::action(t("menu.window.minimize", cx), super::Minimize),
                MenuItem::action(t("menu.window.zoom", cx), super::Zoom),
                MenuItem::separator(),
            ],
        },
        Menu {
            name: t("menu.help", cx),
            disabled: false,
            items: vec![
                MenuItem::action(
                    t("menu.help.view_release_notes", cx),
                    auto_update_ui::ViewReleaseNotesLocally,
                ),
                MenuItem::action(t("menu.help.view_telemetry", cx), zed_actions::OpenTelemetryLog),
                MenuItem::action(t("menu.help.view_dependency_licenses", cx), zed_actions::OpenLicenses),
                MenuItem::action(t("menu.help.show_welcome", cx), onboarding::ShowWelcome),
                MenuItem::separator(),
                MenuItem::action(t("menu.help.file_bug_report", cx), zed_actions::feedback::FileBugReport),
                MenuItem::action(t("menu.help.request_feature", cx), zed_actions::feedback::RequestFeature),
                MenuItem::action(t("menu.help.email_us", cx), zed_actions::feedback::EmailZed),
                MenuItem::separator(),
                MenuItem::action(
                    t("menu.help.documentation", cx),
                    super::OpenBrowser {
                        url: "https://zed.dev/docs".into(),
                    },
                ),
                MenuItem::action(t("menu.help.zed_repository", cx), feedback::OpenZedRepo),
                MenuItem::action(
                    t("menu.help.zed_twitter", cx),
                    super::OpenBrowser {
                        url: "https://twitter.com/zeddotdev".into(),
                    },
                ),
                MenuItem::action(
                    t("menu.help.join_the_team", cx),
                    super::OpenBrowser {
                        url: "https://zed.dev/jobs".into(),
                    },
                ),
            ],
        },
    ]
}
