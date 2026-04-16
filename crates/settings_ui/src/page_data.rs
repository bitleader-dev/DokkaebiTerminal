use feature_flags::{AgentV2FeatureFlag, FeatureFlagAppExt as _};
use gpui::{Action as _, App, PathPromptOptions, ReadGlobal as _};
use settings::{
    LanguageSettingsContent, SettingsContent, SettingsStore,
    WallpaperFitContent, WallpaperOpacity,
};
use std::sync::Arc;
use strum::IntoDiscriminant as _;
use ui::IntoElement;

use crate::{
    ActionLink, DynamicItem, PROJECT, SettingField, SettingItem, SettingsFieldMetadata,
    SettingsPage, SettingsPageItem, SubPageLink, USER, active_language, all_language_names,
    pages::{render_edit_prediction_setup_page, render_tool_permissions_setup_page},
};

const DEFAULT_STRING: String = String::new();
/// A default empty string reference. Useful in `pick` functions for cases either in dynamic item fields, or when dealing with `settings::Maybe`
/// to avoid the "NO DEFAULT" case.
const DEFAULT_EMPTY_STRING: Option<&String> = Some(&DEFAULT_STRING);



macro_rules! concat_sections {
    (@vec, $($arr:expr),+ $(,)?) => {{
        let total_len = 0_usize $(+ $arr.len())+;
        let mut out = Vec::with_capacity(total_len);

        $(
            out.extend($arr);
        )+

        out
    }};

    ($($arr:expr),+ $(,)?) => {{
        let total_len = 0_usize $(+ $arr.len())+;

        let mut out: Box<[std::mem::MaybeUninit<_>]> = Box::new_uninit_slice(total_len);

        let mut index = 0usize;
        $(
            let array = $arr;
            for item in array {
                out[index].write(item);
                index += 1;
            }
        )+

        debug_assert_eq!(index, total_len);

        // SAFETY: we wrote exactly `total_len` elements.
        unsafe { out.assume_init() }
    }};
}

pub(crate) fn settings_data(cx: &App) -> Vec<SettingsPage> {
    vec![
        general_page(),
        appearance_page(),
        keymap_page(),
        editor_page(),
        languages_and_tools_page(cx),
        search_and_files_page(),
        window_and_layout_page(),
        panels_page(),

        terminal_page(),
        version_control_page(),

        ai_page(cx),
        notification_page(),
        wallpaper_page(),
    ]
}

fn general_page() -> SettingsPage {
    fn general_settings_section() -> [SettingsPageItem; 7] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.general_settings"),
            SettingsPageItem::SettingItem(SettingItem {
                files: PROJECT,
                title: "settings_page.item.project_name",
                description: "settings_page.desc.general_settings.project_name",
                field: Box::new(SettingField {
                    json_path: Some("project_name"),
                    pick: |settings_content| {
                        settings_content
                            .project
                            .worktree
                            .project_name
                            .as_ref()
                            .or(DEFAULT_EMPTY_STRING)
                    },
                    write: |settings_content, value| {
                        settings_content.project.worktree.project_name =
                            value.filter(|name| !name.is_empty());
                    },
                }),
                metadata: Some(Box::new(SettingsFieldMetadata {
                    placeholder: Some("Project Name"),
                    ..Default::default()
                })),
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.when_closing_with_no_tabs",
                description: "settings_page.desc.general_settings.when_closing_with_no_tabs",
                field: Box::new(SettingField {
                    json_path: Some("when_closing_with_no_tabs"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .when_closing_with_no_tabs
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.when_closing_with_no_tabs = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.on_last_window_closed",
                description: "settings_page.desc.general_settings.on_last_window_closed",
                field: Box::new(SettingField {
                    json_path: Some("on_last_window_closed"),
                    pick: |settings_content| {
                        settings_content.workspace.on_last_window_closed.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.on_last_window_closed = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_system_path_prompts",
                description: "settings_page.desc.general_settings.use_system_path_prompts",
                field: Box::new(SettingField {
                    json_path: Some("use_system_path_prompts"),
                    pick: |settings_content| {
                        settings_content.workspace.use_system_path_prompts.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.use_system_path_prompts = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_system_prompts",
                description: "settings_page.desc.general_settings.use_system_prompts",
                field: Box::new(SettingField {
                    json_path: Some("use_system_prompts"),
                    pick: |settings_content| settings_content.workspace.use_system_prompts.as_ref(),
                    write: |settings_content, value| {
                        settings_content.workspace.use_system_prompts = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.system_monitoring",
                description: "settings_page.desc.general_settings.system_monitoring",
                field: Box::new(SettingField {
                    json_path: Some("system_monitoring"),
                    pick: |settings_content| {
                        settings_content.workspace.system_monitoring.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.system_monitoring = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn workspace_restoration_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.workspace_restoration"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.restore_unsaved_buffers",
                description: "settings_page.desc.workspace_restoration.restore_unsaved_buffers",
                field: Box::new(SettingField {
                    json_path: Some("session.restore_unsaved_buffers"),
                    pick: |settings_content| {
                        settings_content
                            .session
                            .as_ref()
                            .and_then(|session| session.restore_unsaved_buffers.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .session
                            .get_or_insert_default()
                            .restore_unsaved_buffers = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.restore_on_startup",
                description: "settings_page.desc.workspace_restoration.restore_on_startup",
                field: Box::new(SettingField {
                    json_path: Some("restore_on_startup"),
                    pick: |settings_content| settings_content.workspace.restore_on_startup.as_ref(),
                    write: |settings_content, value| {
                        settings_content.workspace.restore_on_startup = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn auto_update_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.auto_update"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_update",
                description: "settings_page.desc.auto_update.auto_update",
                field: Box::new(SettingField {
                    json_path: Some("auto_update"),
                    pick: |settings_content| settings_content.auto_update.as_ref(),
                    write: |settings_content, value| {
                        settings_content.auto_update = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.general",
        items: concat_sections!(
            general_settings_section(),
            workspace_restoration_section(),
            auto_update_section(),
        ),
    }
}

fn appearance_page() -> SettingsPage {
    fn theme_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.theme"),
            SettingsPageItem::DynamicItem(DynamicItem {
                discriminant: SettingItem {
                    files: USER,
                    title: "settings_page.item.theme_mode",
                    description: "settings_page.desc.theme.theme_mode",
                    field: Box::new(SettingField {
                        json_path: Some("theme$"),
                        pick: |settings_content| {
                            Some(&dynamic_variants::<settings::ThemeSelection>()[
                                settings_content
                                    .theme
                                    .theme
                                    .as_ref()?
                                    .discriminant() as usize])
                        },
                        write: |settings_content, value| {
                            let Some(value) = value else {
                                settings_content.theme.theme = None;
                                return;
                            };
                            let settings_value = settings_content.theme.theme.get_or_insert_default();
                            *settings_value = match value {
                                settings::ThemeSelectionDiscriminants::Static => {
                                    let name = match settings_value {
                                        settings::ThemeSelection::Static(_) => return,
                                        settings::ThemeSelection::Dynamic { mode, light, dark } => {
                                            match mode {
                                                theme_settings::ThemeAppearanceMode::Light => light.clone(),
                                                theme_settings::ThemeAppearanceMode::Dark => dark.clone(),
                                                theme_settings::ThemeAppearanceMode::System => dark.clone(), // no cx, can't determine correct choice
                                            }
                                        },
                                    };
                                    settings::ThemeSelection::Static(name)
                                },
                                settings::ThemeSelectionDiscriminants::Dynamic => {
                                    let static_name = match settings_value {
                                        settings::ThemeSelection::Static(theme_name) => theme_name.clone(),
                                        settings::ThemeSelection::Dynamic {..} => return,
                                    };

                                    settings::ThemeSelection::Dynamic {
                                        mode: settings::ThemeAppearanceMode::System,
                                        light: static_name.clone(),
                                        dark: static_name,
                                    }
                                },
                            };
                        },
                    }),
                    metadata: None,
                },
                pick_discriminant: |settings_content| {
                    Some(settings_content.theme.theme.as_ref()?.discriminant() as usize)
                },
                fields: dynamic_variants::<settings::ThemeSelection>().into_iter().map(|variant| {
                    match variant {
                        settings::ThemeSelectionDiscriminants::Static => vec![
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.theme_name",
                                description: "settings_page.desc.theme.theme_name",
                                field: Box::new(SettingField {
                                    json_path: Some("theme"),
                                    pick: |settings_content| {
                                        match settings_content.theme.theme.as_ref() {
                                            Some(settings::ThemeSelection::Static(name)) => Some(name),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .theme.get_or_insert_default() {
                                                settings::ThemeSelection::Static(theme_name) => *theme_name = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            }
                        ],
                        settings::ThemeSelectionDiscriminants::Dynamic => vec![
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.mode",
                                description: "settings_page.desc.theme.mode",
                                field: Box::new(SettingField {
                                    json_path: Some("theme.mode"),
                                    pick: |settings_content| {
                                        match settings_content.theme.theme.as_ref() {
                                            Some(settings::ThemeSelection::Dynamic { mode, ..}) => Some(mode),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .theme.get_or_insert_default() {
                                                settings::ThemeSelection::Dynamic{ mode, ..} => *mode = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            },
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.light_theme",
                                description: "settings_page.desc.theme.light_theme",
                                field: Box::new(SettingField {
                                    json_path: Some("theme.light"),
                                    pick: |settings_content| {
                                        match settings_content.theme.theme.as_ref() {
                                            Some(settings::ThemeSelection::Dynamic { light, ..}) => Some(light),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .theme.get_or_insert_default() {
                                                settings::ThemeSelection::Dynamic{ light, ..} => *light = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            },
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.dark_theme",
                                description: "settings_page.desc.theme.dark_theme",
                                field: Box::new(SettingField {
                                    json_path: Some("theme.dark"),
                                    pick: |settings_content| {
                                        match settings_content.theme.theme.as_ref() {
                                            Some(settings::ThemeSelection::Dynamic { dark, ..}) => Some(dark),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .theme.get_or_insert_default() {
                                                settings::ThemeSelection::Dynamic{ dark, ..} => *dark = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            }
                        ],
                    }
                }).collect(),
            }),
            SettingsPageItem::DynamicItem(DynamicItem {
                discriminant: SettingItem {
                    files: USER,
                    title: "settings_page.item.icon_theme",
                    description: "settings_page.desc.theme.icon_theme",
                    field: Box::new(SettingField {
                        json_path: Some("icon_theme$"),
                        pick: |settings_content| {
                            Some(&dynamic_variants::<settings::IconThemeSelection>()[
                                settings_content
                                    .theme
                                    .icon_theme
                                    .as_ref()?
                                    .discriminant() as usize])
                        },
                        write: |settings_content, value| {
                            let Some(value) = value else {
                                settings_content.theme.icon_theme = None;
                                return;
                            };
                            let settings_value = settings_content.theme.icon_theme.get_or_insert_with(|| {
                                settings::IconThemeSelection::Static(settings::IconThemeName(theme::default_icon_theme().name.clone().into()))
                            });
                            *settings_value = match value {
                                settings::IconThemeSelectionDiscriminants::Static => {
                                    let name = match settings_value {
                                        settings::IconThemeSelection::Static(_) => return,
                                        settings::IconThemeSelection::Dynamic { mode, light, dark } => {
                                            match mode {
                                                theme_settings::ThemeAppearanceMode::Light => light.clone(),
                                                theme_settings::ThemeAppearanceMode::Dark => dark.clone(),
                                                theme_settings::ThemeAppearanceMode::System => dark.clone(), // no cx, can't determine correct choice
                                            }
                                        },
                                    };
                                    settings::IconThemeSelection::Static(name)
                                },
                                settings::IconThemeSelectionDiscriminants::Dynamic => {
                                    let static_name = match settings_value {
                                        settings::IconThemeSelection::Static(theme_name) => theme_name.clone(),
                                        settings::IconThemeSelection::Dynamic {..} => return,
                                    };

                                    settings::IconThemeSelection::Dynamic {
                                        mode: settings::ThemeAppearanceMode::System,
                                        light: static_name.clone(),
                                        dark: static_name,
                                    }
                                },
                            };
                        },
                    }),
                    metadata: None,
                },
                pick_discriminant: |settings_content| {
                    Some(settings_content.theme.icon_theme.as_ref()?.discriminant() as usize)
                },
                fields: dynamic_variants::<settings::IconThemeSelection>().into_iter().map(|variant| {
                    match variant {
                        settings::IconThemeSelectionDiscriminants::Static => vec![
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.icon_theme_name",
                                description: "settings_page.desc.theme.icon_theme_name",
                                field: Box::new(SettingField {
                                    json_path: Some("icon_theme$string"),
                                    pick: |settings_content| {
                                        match settings_content.theme.icon_theme.as_ref() {
                                            Some(settings::IconThemeSelection::Static(name)) => Some(name),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .icon_theme.as_mut() {
                                                Some(settings::IconThemeSelection::Static(theme_name)) => *theme_name = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            }
                        ],
                        settings::IconThemeSelectionDiscriminants::Dynamic => vec![
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.mode",
                                description: "settings_page.desc.icon_theme.mode",
                                field: Box::new(SettingField {
                                    json_path: Some("icon_theme"),
                                    pick: |settings_content| {
                                        match settings_content.theme.icon_theme.as_ref() {
                                            Some(settings::IconThemeSelection::Dynamic { mode, ..}) => Some(mode),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .icon_theme.as_mut() {
                                                Some(settings::IconThemeSelection::Dynamic{ mode, ..}) => *mode = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            },
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.light_icon_theme",
                                description: "settings_page.desc.theme.light_icon_theme",
                                field: Box::new(SettingField {
                                    json_path: Some("icon_theme.light"),
                                    pick: |settings_content| {
                                        match settings_content.theme.icon_theme.as_ref() {
                                            Some(settings::IconThemeSelection::Dynamic { light, ..}) => Some(light),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .icon_theme.as_mut() {
                                                Some(settings::IconThemeSelection::Dynamic{ light, ..}) => *light = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            },
                            SettingItem {
                                files: USER,
                                title: "settings_page.item.dark_icon_theme",
                                description: "settings_page.desc.theme.dark_icon_theme",
                                field: Box::new(SettingField {
                                    json_path: Some("icon_theme.dark"),
                                    pick: |settings_content| {
                                        match settings_content.theme.icon_theme.as_ref() {
                                            Some(settings::IconThemeSelection::Dynamic { dark, ..}) => Some(dark),
                                            _ => None
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .theme
                                            .icon_theme.as_mut() {
                                                Some(settings::IconThemeSelection::Dynamic{ dark, ..}) => *dark = value,
                                                _ => return
                                            }
                                    },
                                }),
                                metadata: None,
                            }
                        ],
                    }
                }).collect(),
            }),
        ]
    }

    fn buffer_font_section() -> [SettingsPageItem; 7] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.buffer_font"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_family",
                description: "settings_page.desc.buffer_font.font_family",
                field: Box::new(SettingField {
                    json_path: Some("buffer_font_family"),
                    pick: |settings_content| settings_content.theme.buffer_font_family.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.buffer_font_family = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_size",
                description: "settings_page.desc.buffer_font.font_size",
                field: Box::new(SettingField {
                    json_path: Some("buffer_font_size"),
                    pick: |settings_content| settings_content.theme.buffer_font_size.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.buffer_font_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_weight",
                description: "settings_page.desc.buffer_font.font_weight",
                field: Box::new(SettingField {
                    json_path: Some("buffer_font_weight"),
                    pick: |settings_content| settings_content.theme.buffer_font_weight.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.buffer_font_weight = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::DynamicItem(DynamicItem {
                discriminant: SettingItem {
                    files: USER,
                    title: "settings_page.item.line_height",
                    description: "settings_page.desc.buffer_font.line_height",
                    field: Box::new(SettingField {
                        json_path: Some("buffer_line_height$"),
                        pick: |settings_content| {
                            Some(
                                &dynamic_variants::<settings::BufferLineHeight>()[settings_content
                                    .theme
                                    .buffer_line_height
                                    .as_ref()?
                                    .discriminant()
                                    as usize],
                            )
                        },
                        write: |settings_content, value| {
                            let Some(value) = value else {
                                settings_content.theme.buffer_line_height = None;
                                return;
                            };
                            let settings_value = settings_content
                                .theme
                                .buffer_line_height
                                .get_or_insert_with(|| settings::BufferLineHeight::default());
                            *settings_value = match value {
                                settings::BufferLineHeightDiscriminants::Comfortable => {
                                    settings::BufferLineHeight::Comfortable
                                }
                                settings::BufferLineHeightDiscriminants::Standard => {
                                    settings::BufferLineHeight::Standard
                                }
                                settings::BufferLineHeightDiscriminants::Custom => {
                                    let custom_value =
                                        theme_settings::BufferLineHeight::from(*settings_value)
                                            .value();
                                    settings::BufferLineHeight::Custom(custom_value)
                                }
                            };
                        },
                    }),
                    metadata: None,
                },
                pick_discriminant: |settings_content| {
                    Some(
                        settings_content
                            .theme
                            .buffer_line_height
                            .as_ref()?
                            .discriminant() as usize,
                    )
                },
                fields: dynamic_variants::<settings::BufferLineHeight>()
                    .into_iter()
                    .map(|variant| match variant {
                        settings::BufferLineHeightDiscriminants::Comfortable => vec![],
                        settings::BufferLineHeightDiscriminants::Standard => vec![],
                        settings::BufferLineHeightDiscriminants::Custom => vec![SettingItem {
                            files: USER,
                            title: "settings_page.item.custom_line_height",
                            description: "settings_page.desc.buffer_font.custom_line_height",
                            field: Box::new(SettingField {
                                json_path: Some("buffer_line_height"),
                                pick: |settings_content| match settings_content
                                    .theme
                                    .buffer_line_height
                                    .as_ref()
                                {
                                    Some(settings::BufferLineHeight::Custom(value)) => Some(value),
                                    _ => None,
                                },
                                write: |settings_content, value| {
                                    let Some(value) = value else {
                                        return;
                                    };
                                    match settings_content.theme.buffer_line_height.as_mut() {
                                        Some(settings::BufferLineHeight::Custom(line_height)) => {
                                            *line_height = f32::max(value, 1.0)
                                        }
                                        _ => return,
                                    }
                                },
                            }),
                            metadata: None,
                        }],
                    })
                    .collect(),
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.font_features",
                description: "settings_page.desc.buffer_font.font_features",
                field: Box::new(
                    SettingField {
                        json_path: Some("buffer_font_features"),
                        pick: |settings_content| {
                            settings_content.theme.buffer_font_features.as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.theme.buffer_font_features = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.font_fallbacks",
                description: "settings_page.desc.buffer_font.font_fallbacks",
                field: Box::new(
                    SettingField {
                        json_path: Some("buffer_font_fallbacks"),
                        pick: |settings_content| {
                            settings_content.theme.buffer_font_fallbacks.as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.theme.buffer_font_fallbacks = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
            }),
        ]
    }

    fn ui_font_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.ui_font"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_family",
                description: "settings_page.desc.ui_font.font_family",
                field: Box::new(SettingField {
                    json_path: Some("ui_font_family"),
                    pick: |settings_content| settings_content.theme.ui_font_family.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.ui_font_family = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_size",
                description: "settings_page.desc.ui_font.font_size",
                field: Box::new(SettingField {
                    json_path: Some("ui_font_size"),
                    pick: |settings_content| settings_content.theme.ui_font_size.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.ui_font_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_weight",
                description: "settings_page.desc.ui_font.font_weight",
                field: Box::new(SettingField {
                    json_path: Some("ui_font_weight"),
                    pick: |settings_content| settings_content.theme.ui_font_weight.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.ui_font_weight = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.font_features",
                description: "settings_page.desc.ui_font.font_features",
                field: Box::new(
                    SettingField {
                        json_path: Some("ui_font_features"),
                        pick: |settings_content| settings_content.theme.ui_font_features.as_ref(),
                        write: |settings_content, value| {
                            settings_content.theme.ui_font_features = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.font_fallbacks",
                description: "settings_page.desc.ui_font.font_fallbacks",
                field: Box::new(
                    SettingField {
                        json_path: Some("ui_font_fallbacks"),
                        pick: |settings_content| settings_content.theme.ui_font_fallbacks.as_ref(),
                        write: |settings_content, value| {
                            settings_content.theme.ui_font_fallbacks = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
            }),
        ]
    }

    fn agent_panel_font_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.agent_panel_font"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.ui_font_size",
                description: "settings_page.desc.agent_panel_font.ui_font_size",
                field: Box::new(SettingField {
                    json_path: Some("agent_ui_font_size"),
                    pick: |settings_content| {
                        settings_content
                            .theme
                            .agent_ui_font_size
                            .as_ref()
                            .or(settings_content.theme.ui_font_size.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.theme.agent_ui_font_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.buffer_font_size",
                description: "settings_page.desc.agent_panel_font.buffer_font_size",
                field: Box::new(SettingField {
                    json_path: Some("agent_buffer_font_size"),
                    pick: |settings_content| {
                        settings_content
                            .theme
                            .agent_buffer_font_size
                            .as_ref()
                            .or(settings_content.theme.buffer_font_size.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.theme.agent_buffer_font_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn text_rendering_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.text_rendering"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.text_rendering_mode",
                description: "settings_page.desc.text_rendering.text_rendering_mode",
                field: Box::new(SettingField {
                    json_path: Some("text_rendering_mode"),
                    pick: |settings_content| {
                        settings_content.workspace.text_rendering_mode.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.text_rendering_mode = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn cursor_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.cursor"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.multi_cursor_modifier",
                description: "settings_page.desc.cursor.multi_cursor_modifier",
                field: Box::new(SettingField {
                    json_path: Some("multi_cursor_modifier"),
                    pick: |settings_content| settings_content.editor.multi_cursor_modifier.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.multi_cursor_modifier = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_blink",
                description: "settings_page.desc.cursor.cursor_blink",
                field: Box::new(SettingField {
                    json_path: Some("cursor_blink"),
                    pick: |settings_content| settings_content.editor.cursor_blink.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.cursor_blink = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_shape",
                description: "settings_page.desc.cursor.cursor_shape",
                field: Box::new(SettingField {
                    json_path: Some("cursor_shape"),
                    pick: |settings_content| settings_content.editor.cursor_shape.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.cursor_shape = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hide_mouse",
                description: "settings_page.desc.cursor.hide_mouse",
                field: Box::new(SettingField {
                    json_path: Some("hide_mouse"),
                    pick: |settings_content| settings_content.editor.hide_mouse.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.hide_mouse = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn highlighting_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.highlighting"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.unnecessary_code_fade",
                description: "settings_page.desc.highlighting.unnecessary_code_fade",
                field: Box::new(SettingField {
                    json_path: Some("unnecessary_code_fade"),
                    pick: |settings_content| settings_content.theme.unnecessary_code_fade.as_ref(),
                    write: |settings_content, value| {
                        settings_content.theme.unnecessary_code_fade = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.current_line_highlight",
                description: "settings_page.desc.highlighting.current_line_highlight",
                field: Box::new(SettingField {
                    json_path: Some("current_line_highlight"),
                    pick: |settings_content| {
                        settings_content.editor.current_line_highlight.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.current_line_highlight = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.selection_highlight",
                description: "settings_page.desc.highlighting.selection_highlight",
                field: Box::new(SettingField {
                    json_path: Some("selection_highlight"),
                    pick: |settings_content| settings_content.editor.selection_highlight.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.selection_highlight = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.rounded_selection",
                description: "settings_page.desc.highlighting.rounded_selection",
                field: Box::new(SettingField {
                    json_path: Some("rounded_selection"),
                    pick: |settings_content| settings_content.editor.rounded_selection.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.rounded_selection = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.minimum_contrast_for_highlights",
                description: "settings_page.desc.highlighting.minimum_contrast_for_highlights",
                field: Box::new(SettingField {
                    json_path: Some("minimum_contrast_for_highlights"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .minimum_contrast_for_highlights
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.minimum_contrast_for_highlights = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn guides_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.guides"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_wrap_guides",
                description: "settings_page.desc.guides.show_wrap_guides",
                field: Box::new(SettingField {
                    json_path: Some("show_wrap_guides"),
                    pick: |settings_content| {
                        settings_content
                            .project
                            .all_languages
                            .defaults
                            .show_wrap_guides
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project
                            .all_languages
                            .defaults
                            .show_wrap_guides = value;
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            // todo(settings_ui): This needs a custom component
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.wrap_guides",
                description: "settings_page.desc.guides.wrap_guides",
                field: Box::new(
                    SettingField {
                        json_path: Some("wrap_guides"),
                        pick: |settings_content| {
                            settings_content
                                .project
                                .all_languages
                                .defaults
                                .wrap_guides
                                .as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.project.all_languages.defaults.wrap_guides = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    let items: Box<[SettingsPageItem]> = concat_sections!(
        theme_section(),
        buffer_font_section(),
        ui_font_section(),
        agent_panel_font_section(),
        text_rendering_section(),
        cursor_section(),
        highlighting_section(),
        guides_section(),
    );

    SettingsPage {
        title: "settings_page.item.appearance",
        items,
    }
}

fn keymap_page() -> SettingsPage {
    fn keybindings_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.keybindings"),
            SettingsPageItem::ActionLink(ActionLink {
                title: "settings_page.item.edit_keybindings".into(),
                description: Some("settings_page.desc.keybindings.edit_keybindings".into()),
                button_text: "settings_page.button.open_keymap".into(),
                on_click: Arc::new(|settings_window, window, cx| {
                    let Some(original_window) = settings_window.original_window else {
                        return;
                    };
                    original_window
                        .update(cx, |_workspace, original_window, cx| {
                            original_window
                                .dispatch_action(zed_actions::OpenKeymap.boxed_clone(), cx);
                            original_window.activate_window();
                        })
                        .ok();
                    window.remove_window();
                }),
                files: USER,
            }),
        ]
    }

    fn base_keymap_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.base_keymap"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.base_keymap",
                description: "settings_page.desc.base_keymap.base_keymap",
                field: Box::new(SettingField {
                    json_path: Some("base_keymap"),
                    pick: |settings_content| settings_content.base_keymap.as_ref(),
                    write: |settings_content, value| {
                        settings_content.base_keymap = value;
                    },
                }),
                metadata: Some(Box::new(SettingsFieldMetadata {
                    should_do_titlecase: Some(false),
                    ..Default::default()
                })),
                files: USER,
            }),
        ]
    }

    fn modal_editing_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.modal_editing"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.vim_mode",
                description: "settings_page.desc.modal_editing.vim_mode",
                field: Box::new(SettingField {
                    json_path: Some("vim_mode"),
                    pick: |settings_content| settings_content.vim_mode.as_ref(),
                    write: write_vim_mode,
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.helix_mode",
                description: "settings_page.desc.modal_editing.helix_mode",
                field: Box::new(SettingField {
                    json_path: Some("helix_mode"),
                    pick: |settings_content| settings_content.helix_mode.as_ref(),
                    write: write_helix_mode,
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    let items: Box<[SettingsPageItem]> = concat_sections!(
        keybindings_section(),
        base_keymap_section(),
        modal_editing_section(),
    );

    SettingsPage {
        title: "settings_page.item.keymap",
        items,
    }
}

fn editor_page() -> SettingsPage {
    fn auto_save_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.auto_save"),
            SettingsPageItem::DynamicItem(DynamicItem {
                discriminant: SettingItem {
                    files: USER,
                    title: "settings_page.item.auto_save_mode",
                    description: "settings_page.desc.auto_save.auto_save_mode",
                    field: Box::new(SettingField {
                        json_path: Some("autosave$"),
                        pick: |settings_content| {
                            Some(
                                &dynamic_variants::<settings::AutosaveSetting>()[settings_content
                                    .workspace
                                    .autosave
                                    .as_ref()?
                                    .discriminant()
                                    as usize],
                            )
                        },
                        write: |settings_content, value| {
                            let Some(value) = value else {
                                settings_content.workspace.autosave = None;
                                return;
                            };
                            let settings_value = settings_content
                                .workspace
                                .autosave
                                .get_or_insert_with(|| settings::AutosaveSetting::Off);
                            *settings_value = match value {
                                settings::AutosaveSettingDiscriminants::Off => {
                                    settings::AutosaveSetting::Off
                                }
                                settings::AutosaveSettingDiscriminants::AfterDelay => {
                                    let milliseconds = match settings_value {
                                        settings::AutosaveSetting::AfterDelay { milliseconds } => {
                                            *milliseconds
                                        }
                                        _ => settings::DelayMs(1000),
                                    };
                                    settings::AutosaveSetting::AfterDelay { milliseconds }
                                }
                                settings::AutosaveSettingDiscriminants::OnFocusChange => {
                                    settings::AutosaveSetting::OnFocusChange
                                }
                                settings::AutosaveSettingDiscriminants::OnWindowChange => {
                                    settings::AutosaveSetting::OnWindowChange
                                }
                            };
                        },
                    }),
                    metadata: None,
                },
                pick_discriminant: |settings_content| {
                    Some(settings_content.workspace.autosave.as_ref()?.discriminant() as usize)
                },
                fields: dynamic_variants::<settings::AutosaveSetting>()
                    .into_iter()
                    .map(|variant| match variant {
                        settings::AutosaveSettingDiscriminants::Off => vec![],
                        settings::AutosaveSettingDiscriminants::AfterDelay => vec![SettingItem {
                            files: USER,
                            title: "settings_page.item.delay_milliseconds",
                            description: "settings_page.desc.auto_save.delay_milliseconds",
                            field: Box::new(SettingField {
                                json_path: Some("autosave.after_delay.milliseconds"),
                                pick: |settings_content| match settings_content
                                    .workspace
                                    .autosave
                                    .as_ref()
                                {
                                    Some(settings::AutosaveSetting::AfterDelay {
                                        milliseconds,
                                    }) => Some(milliseconds),
                                    _ => None,
                                },
                                write: |settings_content, value| {
                                    let Some(value) = value else {
                                        settings_content.workspace.autosave = None;
                                        return;
                                    };
                                    match settings_content.workspace.autosave.as_mut() {
                                        Some(settings::AutosaveSetting::AfterDelay {
                                            milliseconds,
                                        }) => *milliseconds = value,
                                        _ => return,
                                    }
                                },
                            }),
                            metadata: None,
                        }],
                        settings::AutosaveSettingDiscriminants::OnFocusChange => vec![],
                        settings::AutosaveSettingDiscriminants::OnWindowChange => vec![],
                    })
                    .collect(),
            }),
        ]
    }

    fn which_key_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.which_key_menu"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_which_key_menu",
                description: "settings_page.desc.which_key.show_which_key_menu",
                field: Box::new(SettingField {
                    json_path: Some("which_key.enabled"),
                    pick: |settings_content| {
                        settings_content
                            .which_key
                            .as_ref()
                            .and_then(|settings| settings.enabled.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.which_key.get_or_insert_default().enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.menu_delay",
                description: "settings_page.desc.which_key.menu_delay",
                field: Box::new(SettingField {
                    json_path: Some("which_key.delay_ms"),
                    pick: |settings_content| {
                        settings_content
                            .which_key
                            .as_ref()
                            .and_then(|settings| settings.delay_ms.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.which_key.get_or_insert_default().delay_ms = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn multibuffer_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.multibuffer"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.double_click_in_multibuffer",
                description: "settings_page.desc.multibuffer.double_click_in_multibuffer",
                field: Box::new(SettingField {
                    json_path: Some("double_click_in_multibuffer"),
                    pick: |settings_content| {
                        settings_content.editor.double_click_in_multibuffer.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.double_click_in_multibuffer = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.expand_excerpt_lines",
                description: "settings_page.desc.multibuffer.expand_excerpt_lines",
                field: Box::new(SettingField {
                    json_path: Some("expand_excerpt_lines"),
                    pick: |settings_content| settings_content.editor.expand_excerpt_lines.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.expand_excerpt_lines = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.excerpt_context_lines",
                description: "settings_page.desc.multibuffer.excerpt_context_lines",
                field: Box::new(SettingField {
                    json_path: Some("excerpt_context_lines"),
                    pick: |settings_content| settings_content.editor.excerpt_context_lines.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.excerpt_context_lines = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.expand_outlines_with_depth",
                description: "settings_page.desc.multibuffer.expand_outlines_with_depth",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.expand_outlines_with_depth"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()
                            .and_then(|outline_panel| {
                                outline_panel.expand_outlines_with_depth.as_ref()
                            })
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .expand_outlines_with_depth = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.diff_view_style",
                description: "settings_page.desc.multibuffer.diff_view_style",
                field: Box::new(SettingField {
                    json_path: Some("diff_view_style"),
                    pick: |settings_content| settings_content.editor.diff_view_style.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.diff_view_style = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn scrolling_section() -> [SettingsPageItem; 8] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.scrolling"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.scroll_beyond_last_line",
                description: "settings_page.desc.scrolling.scroll_beyond_last_line",
                field: Box::new(SettingField {
                    json_path: Some("scroll_beyond_last_line"),
                    pick: |settings_content| {
                        settings_content.editor.scroll_beyond_last_line.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.scroll_beyond_last_line = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.vertical_scroll_margin",
                description: "settings_page.desc.scrolling.vertical_scroll_margin",
                field: Box::new(SettingField {
                    json_path: Some("vertical_scroll_margin"),
                    pick: |settings_content| {
                        settings_content.editor.vertical_scroll_margin.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.vertical_scroll_margin = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.horizontal_scroll_margin",
                description: "settings_page.desc.scrolling.horizontal_scroll_margin",
                field: Box::new(SettingField {
                    json_path: Some("horizontal_scroll_margin"),
                    pick: |settings_content| {
                        settings_content.editor.horizontal_scroll_margin.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.horizontal_scroll_margin = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.scroll_sensitivity",
                description: "settings_page.desc.scrolling.scroll_sensitivity",
                field: Box::new(SettingField {
                    json_path: Some("scroll_sensitivity"),
                    pick: |settings_content| settings_content.editor.scroll_sensitivity.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.scroll_sensitivity = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.fast_scroll_sensitivity",
                description: "settings_page.desc.scrolling.fast_scroll_sensitivity",
                field: Box::new(SettingField {
                    json_path: Some("fast_scroll_sensitivity"),
                    pick: |settings_content| {
                        settings_content.editor.fast_scroll_sensitivity.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.fast_scroll_sensitivity = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.autoscroll_on_clicks",
                description: "settings_page.desc.scrolling.autoscroll_on_clicks",
                field: Box::new(SettingField {
                    json_path: Some("autoscroll_on_clicks"),
                    pick: |settings_content| settings_content.editor.autoscroll_on_clicks.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.autoscroll_on_clicks = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.sticky_scroll",
                description: "settings_page.desc.scrolling.sticky_scroll",
                field: Box::new(SettingField {
                    json_path: Some("sticky_scroll.enabled"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .sticky_scroll
                            .as_ref()
                            .and_then(|sticky_scroll| sticky_scroll.enabled.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .sticky_scroll
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn signature_help_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.signature_help"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_signature_help",
                description: "settings_page.desc.signature_help.auto_signature_help",
                field: Box::new(SettingField {
                    json_path: Some("auto_signature_help"),
                    pick: |settings_content| settings_content.editor.auto_signature_help.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.auto_signature_help = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_signature_help_after_edits",
                description: "settings_page.desc.signature_help.show_signature_help_after_edits",
                field: Box::new(SettingField {
                    json_path: Some("show_signature_help_after_edits"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .show_signature_help_after_edits
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.show_signature_help_after_edits = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.snippet_sort_order",
                description: "settings_page.desc.signature_help.snippet_sort_order",
                field: Box::new(SettingField {
                    json_path: Some("snippet_sort_order"),
                    pick: |settings_content| settings_content.editor.snippet_sort_order.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.snippet_sort_order = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn hover_popover_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.hover_popover"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.hover_popover.enabled",
                field: Box::new(SettingField {
                    json_path: Some("hover_popover_enabled"),
                    pick: |settings_content| settings_content.editor.hover_popover_enabled.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.hover_popover_enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            // todo(settings ui): add units to this number input
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.delay",
                description: "settings_page.desc.hover_popover.delay",
                field: Box::new(SettingField {
                    // delay 설정의 json_path 오타 수정
                    json_path: Some("hover_popover_delay"),
                    pick: |settings_content| settings_content.editor.hover_popover_delay.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.hover_popover_delay = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn drag_and_drop_selection_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.drag_and_drop_selection"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.drag_and_drop_selection.enabled",
                field: Box::new(SettingField {
                    json_path: Some("drag_and_drop_selection.enabled"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .drag_and_drop_selection
                            .as_ref()
                            .and_then(|drag_and_drop| drag_and_drop.enabled.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .drag_and_drop_selection
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.delay",
                description: "settings_page.desc.drag_and_drop_selection.delay",
                field: Box::new(SettingField {
                    json_path: Some("drag_and_drop_selection.delay"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .drag_and_drop_selection
                            .as_ref()
                            .and_then(|drag_and_drop| drag_and_drop.delay.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .drag_and_drop_selection
                            .get_or_insert_default()
                            .delay = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn gutter_section() -> [SettingsPageItem; 7] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.gutter"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_line_numbers",
                description: "settings_page.desc.gutter.show_line_numbers",
                field: Box::new(SettingField {
                    json_path: Some("gutter.line_numbers"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .gutter
                            .as_ref()
                            .and_then(|gutter| gutter.line_numbers.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .gutter
                            .get_or_insert_default()
                            .line_numbers = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.relative_line_numbers",
                description: "settings_page.desc.gutter.relative_line_numbers",
                field: Box::new(SettingField {
                    json_path: Some("relative_line_numbers"),
                    pick: |settings_content| settings_content.editor.relative_line_numbers.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.relative_line_numbers = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_runnables",
                description: "settings_page.desc.gutter.show_runnables",
                field: Box::new(SettingField {
                    json_path: Some("gutter.runnables"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .gutter
                            .as_ref()
                            .and_then(|gutter| gutter.runnables.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .gutter
                            .get_or_insert_default()
                            .runnables = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_folds",
                description: "settings_page.desc.gutter.show_folds",
                field: Box::new(SettingField {
                    json_path: Some("gutter.folds"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .gutter
                            .as_ref()
                            .and_then(|gutter| gutter.folds.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.editor.gutter.get_or_insert_default().folds = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.min_line_number_digits",
                description: "settings_page.desc.gutter.min_line_number_digits",
                field: Box::new(SettingField {
                    json_path: Some("gutter.min_line_number_digits"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .gutter
                            .as_ref()
                            .and_then(|gutter| gutter.min_line_number_digits.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .gutter
                            .get_or_insert_default()
                            .min_line_number_digits = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.inline_code_actions",
                description: "settings_page.desc.gutter.inline_code_actions",
                field: Box::new(SettingField {
                    json_path: Some("inline_code_actions"),
                    pick: |settings_content| settings_content.editor.inline_code_actions.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.inline_code_actions = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn scrollbar_section() -> [SettingsPageItem; 10] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.scrollbar"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show",
                description: "settings_page.desc.scrollbar.show",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar"),
                    pick: |settings_content| {
                        settings_content.editor.scrollbar.as_ref()?.show.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .show = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursors",
                description: "settings_page.desc.scrollbar.cursors",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.cursors"),
                    pick: |settings_content| {
                        settings_content.editor.scrollbar.as_ref()?.cursors.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .cursors = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_diff",
                description: "settings_page.desc.scrollbar.git_diff",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.git_diff"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .git_diff
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .git_diff = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.search_results",
                description: "settings_page.desc.scrollbar.search_results",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.search_results"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .search_results
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .search_results = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.selected_text",
                description: "settings_page.desc.scrollbar.selected_text",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.selected_text"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .selected_text
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .selected_text = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.selected_symbol",
                description: "settings_page.desc.scrollbar.selected_symbol",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.selected_symbol"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .selected_symbol
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .selected_symbol = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.diagnostics",
                description: "settings_page.desc.scrollbar.diagnostics",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.diagnostics"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .diagnostics
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .diagnostics = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.horizontal_scrollbar",
                description: "settings_page.desc.scrollbar.horizontal_scrollbar",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.axes.horizontal"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .axes
                            .as_ref()?
                            .horizontal
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .axes
                            .get_or_insert_default()
                            .horizontal = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.vertical_scrollbar",
                description: "settings_page.desc.scrollbar.vertical_scrollbar",
                field: Box::new(SettingField {
                    json_path: Some("scrollbar.axes.vertical"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .scrollbar
                            .as_ref()?
                            .axes
                            .as_ref()?
                            .vertical
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .scrollbar
                            .get_or_insert_default()
                            .axes
                            .get_or_insert_default()
                            .vertical = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn minimap_section() -> [SettingsPageItem; 7] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.minimap"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show",
                description: "settings_page.desc.minimap.show",
                field: Box::new(SettingField {
                    json_path: Some("minimap.show"),
                    pick: |settings_content| {
                        settings_content.editor.minimap.as_ref()?.show.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.minimap.get_or_insert_default().show = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.display_in",
                description: "settings_page.desc.minimap.display_in",
                field: Box::new(SettingField {
                    json_path: Some("minimap.display_in"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .minimap
                            .as_ref()?
                            .display_in
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .minimap
                            .get_or_insert_default()
                            .display_in = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.thumb",
                description: "settings_page.desc.minimap.thumb",
                field: Box::new(SettingField {
                    json_path: Some("minimap.thumb"),
                    pick: |settings_content| {
                        settings_content.editor.minimap.as_ref()?.thumb.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .minimap
                            .get_or_insert_default()
                            .thumb = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.thumb_border",
                description: "settings_page.desc.minimap.thumb_border",
                field: Box::new(SettingField {
                    json_path: Some("minimap.thumb_border"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .minimap
                            .as_ref()?
                            .thumb_border
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .minimap
                            .get_or_insert_default()
                            .thumb_border = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.current_line_highlight",
                description: "settings_page.desc.minimap.current_line_highlight",
                field: Box::new(SettingField {
                    json_path: Some("minimap.current_line_highlight"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .minimap
                            .as_ref()
                            .and_then(|minimap| minimap.current_line_highlight.as_ref())
                            .or(settings_content.editor.current_line_highlight.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .minimap
                            .get_or_insert_default()
                            .current_line_highlight = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.max_width_columns",
                description: "settings_page.desc.minimap.max_width_columns",
                field: Box::new(SettingField {
                    json_path: Some("minimap.max_width_columns"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .minimap
                            .as_ref()?
                            .max_width_columns
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .minimap
                            .get_or_insert_default()
                            .max_width_columns = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn toolbar_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.toolbar"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.breadcrumbs",
                description: "settings_page.desc.toolbar.breadcrumbs",
                field: Box::new(SettingField {
                    json_path: Some("toolbar.breadcrumbs"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .toolbar
                            .as_ref()?
                            .breadcrumbs
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .toolbar
                            .get_or_insert_default()
                            .breadcrumbs = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.quick_actions",
                description: "settings_page.desc.toolbar.quick_actions",
                field: Box::new(SettingField {
                    json_path: Some("toolbar.quick_actions"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .toolbar
                            .as_ref()?
                            .quick_actions
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .toolbar
                            .get_or_insert_default()
                            .quick_actions = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.selections_menu",
                description: "settings_page.desc.toolbar.selections_menu",
                field: Box::new(SettingField {
                    json_path: Some("toolbar.selections_menu"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .toolbar
                            .as_ref()?
                            .selections_menu
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .toolbar
                            .get_or_insert_default()
                            .selections_menu = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.agent_review",
                description: "settings_page.desc.toolbar.agent_review",
                field: Box::new(SettingField {
                    json_path: Some("toolbar.agent_review"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .toolbar
                            .as_ref()?
                            .agent_review
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .toolbar
                            .get_or_insert_default()
                            .agent_review = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.code_actions",
                description: "settings_page.desc.toolbar.code_actions",
                field: Box::new(SettingField {
                    json_path: Some("toolbar.code_actions"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .toolbar
                            .as_ref()?
                            .code_actions
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .toolbar
                            .get_or_insert_default()
                            .code_actions = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn vim_settings_section() -> [SettingsPageItem; 12] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.vim"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.default_mode",
                description: "settings_page.desc.vim_settings.default_mode",
                field: Box::new(SettingField {
                    json_path: Some("vim.default_mode"),
                    pick: |settings_content| settings_content.vim.as_ref()?.default_mode.as_ref(),
                    write: |settings_content, value| {
                        settings_content.vim.get_or_insert_default().default_mode = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.toggle_relative_line_numbers",
                description: "settings_page.desc.vim_settings.toggle_relative_line_numbers",
                field: Box::new(SettingField {
                    json_path: Some("vim.toggle_relative_line_numbers"),
                    pick: |settings_content| {
                        settings_content
                            .vim
                            .as_ref()?
                            .toggle_relative_line_numbers
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .toggle_relative_line_numbers = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_system_clipboard",
                description: "settings_page.desc.vim_settings.use_system_clipboard",
                field: Box::new(SettingField {
                    json_path: Some("vim.use_system_clipboard"),
                    pick: |settings_content| {
                        settings_content.vim.as_ref()?.use_system_clipboard.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .use_system_clipboard = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_smartcase_find",
                description: "settings_page.desc.vim_settings.use_smartcase_find",
                field: Box::new(SettingField {
                    json_path: Some("vim.use_smartcase_find"),
                    pick: |settings_content| {
                        settings_content.vim.as_ref()?.use_smartcase_find.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .use_smartcase_find = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.global_substitution_default",
                description: "settings_page.desc.vim_settings.global_substitution_default",
                field: Box::new(SettingField {
                    json_path: Some("vim.gdefault"),
                    pick: |settings_content| settings_content.vim.as_ref()?.gdefault.as_ref(),
                    write: |settings_content, value| {
                        settings_content.vim.get_or_insert_default().gdefault = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.highlight_on_yank_duration",
                description: "settings_page.desc.vim_settings.highlight_on_yank_duration",
                field: Box::new(SettingField {
                    json_path: Some("vim.highlight_on_yank_duration"),
                    pick: |settings_content| {
                        settings_content
                            .vim
                            .as_ref()?
                            .highlight_on_yank_duration
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .highlight_on_yank_duration = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_shape_normal_mode",
                description: "settings_page.desc.vim_settings.cursor_shape_normal_mode",
                field: Box::new(SettingField {
                    json_path: Some("vim.cursor_shape.normal"),
                    pick: |settings_content| {
                        settings_content
                            .vim
                            .as_ref()?
                            .cursor_shape
                            .as_ref()?
                            .normal
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .cursor_shape
                            .get_or_insert_default()
                            .normal = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_shape_insert_mode",
                description: "settings_page.desc.vim_settings.cursor_shape_insert_mode",
                field: Box::new(SettingField {
                    json_path: Some("vim.cursor_shape.insert"),
                    pick: |settings_content| {
                        settings_content
                            .vim
                            .as_ref()?
                            .cursor_shape
                            .as_ref()?
                            .insert
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .cursor_shape
                            .get_or_insert_default()
                            .insert = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_shape_replace_mode",
                description: "settings_page.desc.vim_settings.cursor_shape_replace_mode",
                field: Box::new(SettingField {
                    json_path: Some("vim.cursor_shape.replace"),
                    pick: |settings_content| {
                        settings_content
                            .vim
                            .as_ref()?
                            .cursor_shape
                            .as_ref()?
                            .replace
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .cursor_shape
                            .get_or_insert_default()
                            .replace = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_shape_visual_mode",
                description: "settings_page.desc.vim_settings.cursor_shape_visual_mode",
                field: Box::new(SettingField {
                    json_path: Some("vim.cursor_shape.visual"),
                    pick: |settings_content| {
                        settings_content
                            .vim
                            .as_ref()?
                            .cursor_shape
                            .as_ref()?
                            .visual
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .vim
                            .get_or_insert_default()
                            .cursor_shape
                            .get_or_insert_default()
                            .visual = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.custom_digraphs",
                description: "settings_page.desc.vim_settings.custom_digraphs",
                field: Box::new(
                    SettingField {
                        json_path: Some("vim.custom_digraphs"),
                        pick: |settings_content| {
                            settings_content.vim.as_ref()?.custom_digraphs.as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.vim.get_or_insert_default().custom_digraphs = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
        ]
    }

    let items = concat_sections!(
        auto_save_section(),
        which_key_section(),
        multibuffer_section(),
        scrolling_section(),
        signature_help_section(),
        hover_popover_section(),
        drag_and_drop_selection_section(),
        gutter_section(),
        scrollbar_section(),
        minimap_section(),
        toolbar_section(),
        vim_settings_section(),
        language_settings_data(),
    );

    SettingsPage {
        title: "settings_page.item.editor",
        items: items,
    }
}

fn languages_and_tools_page(cx: &App) -> SettingsPage {
    fn file_types_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.file_types"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_type_associations",
                description: "settings_page.desc.file_types.file_type_associations",
                field: Box::new(
                    SettingField {
                        json_path: Some("file_type_associations"),
                        pick: |settings_content| {
                            settings_content.project.all_languages.file_types.as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.project.all_languages.file_types = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn diagnostics_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.diagnostics"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.max_severity",
                description: "settings_page.desc.diagnostics.max_severity",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics_max_severity"),
                    pick: |settings_content| {
                        settings_content.editor.diagnostics_max_severity.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.diagnostics_max_severity = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.include_warnings",
                description: "settings_page.desc.diagnostics.include_warnings",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.include_warnings"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .include_warnings
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .include_warnings = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn inline_diagnostics_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.inline_diagnostics"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.inline_diagnostics.enabled",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.inline.enabled"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .inline
                            .as_ref()?
                            .enabled
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .inline
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.update_debounce",
                description: "settings_page.desc.inline_diagnostics.update_debounce",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.inline.update_debounce_ms"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .inline
                            .as_ref()?
                            .update_debounce_ms
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .inline
                            .get_or_insert_default()
                            .update_debounce_ms = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.padding",
                description: "settings_page.desc.inline_diagnostics.padding",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.inline.padding"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .inline
                            .as_ref()?
                            .padding
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .inline
                            .get_or_insert_default()
                            .padding = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.minimum_column",
                description: "settings_page.desc.inline_diagnostics.minimum_column",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.inline.min_column"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .inline
                            .as_ref()?
                            .min_column
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .inline
                            .get_or_insert_default()
                            .min_column = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn lsp_pull_diagnostics_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.lsp_pull_diagnostics"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.lsp_pull_diagnostics.enabled",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.lsp_pull_diagnostics.enabled"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .lsp_pull_diagnostics
                            .as_ref()?
                            .enabled
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .lsp_pull_diagnostics
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            // todo(settings_ui): Needs unit
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.debounce",
                description: "settings_page.desc.lsp_pull_diagnostics.debounce",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.lsp_pull_diagnostics.debounce_ms"),
                    pick: |settings_content| {
                        settings_content
                            .diagnostics
                            .as_ref()?
                            .lsp_pull_diagnostics
                            .as_ref()?
                            .debounce_ms
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .diagnostics
                            .get_or_insert_default()
                            .lsp_pull_diagnostics
                            .get_or_insert_default()
                            .debounce_ms = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn lsp_highlights_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.lsp_highlights"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.debounce",
                description: "settings_page.desc.lsp_highlights.debounce",
                field: Box::new(SettingField {
                    json_path: Some("lsp_highlight_debounce"),
                    pick: |settings_content| {
                        settings_content.editor.lsp_highlight_debounce.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.lsp_highlight_debounce = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn languages_list_section(cx: &App) -> Box<[SettingsPageItem]> {
        // todo(settings_ui): Refresh on extension (un)/installed
        // Note that `crates/json_schema_store` solves the same problem, there is probably a way to unify the two
        std::iter::once(SettingsPageItem::SectionHeader("settings_page.section.languages"))
            .chain(all_language_names(cx).into_iter().map(|language_name| {
                let link = format!("languages.{language_name}");
                SettingsPageItem::SubPageLink(SubPageLink {
                    title: language_name,
                    r#type: crate::SubPageType::Language,
                    description: None,
                    json_path: Some(link.leak()),
                    in_json: true,
                    files: USER | PROJECT,
                    render: |this, scroll_handle, window, cx| {
                        let items: Box<[SettingsPageItem]> = concat_sections!(
                            language_settings_data(),
                            non_editor_language_settings_data(),
                            edit_prediction_language_settings_section()
                        );
                        this.render_sub_page_items(
                            items.iter().enumerate(),
                            scroll_handle,
                            window,
                            cx,
                        )
                        .into_any_element()
                    },
                })
            }))
            .collect()
    }

    SettingsPage {
        title: "settings_page.item.languages_tools",
        items: {
            concat_sections!(
                non_editor_language_settings_data(),
                file_types_section(),
                diagnostics_section(),
                inline_diagnostics_section(),
                lsp_pull_diagnostics_section(),
                lsp_highlights_section(),
                languages_list_section(cx),
            )
        },
    }
}

fn search_and_files_page() -> SettingsPage {
    fn search_section() -> [SettingsPageItem; 9] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.search"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.whole_word",
                description: "settings_page.desc.search.whole_word",
                field: Box::new(SettingField {
                    json_path: Some("search.whole_word"),
                    pick: |settings_content| {
                        settings_content.editor.search.as_ref()?.whole_word.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .search
                            .get_or_insert_default()
                            .whole_word = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.case_sensitive",
                description: "settings_page.desc.search.case_sensitive",
                field: Box::new(SettingField {
                    json_path: Some("search.case_sensitive"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .search
                            .as_ref()?
                            .case_sensitive
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .search
                            .get_or_insert_default()
                            .case_sensitive = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_smartcase_search",
                description: "settings_page.desc.search.use_smartcase_search",
                field: Box::new(SettingField {
                    json_path: Some("use_smartcase_search"),
                    pick: |settings_content| settings_content.editor.use_smartcase_search.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.use_smartcase_search = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.include_ignored",
                description: "settings_page.desc.search.include_ignored",
                field: Box::new(SettingField {
                    json_path: Some("search.include_ignored"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .search
                            .as_ref()?
                            .include_ignored
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .search
                            .get_or_insert_default()
                            .include_ignored = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.regex",
                description: "settings_page.desc.search.regex",
                field: Box::new(SettingField {
                    json_path: Some("search.regex"),
                    pick: |settings_content| {
                        settings_content.editor.search.as_ref()?.regex.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.search.get_or_insert_default().regex = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.search_wrap",
                description: "settings_page.desc.search.search_wrap",
                field: Box::new(SettingField {
                    json_path: Some("search_wrap"),
                    pick: |settings_content| settings_content.editor.search_wrap.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.search_wrap = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.center_on_match",
                description: "settings_page.desc.search.center_on_match",
                field: Box::new(SettingField {
                    json_path: Some("editor.search.center_on_match"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .search
                            .as_ref()
                            .and_then(|search| search.center_on_match.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .search
                            .get_or_insert_default()
                            .center_on_match = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.seed_search_query_from_cursor",
                description: "settings_page.desc.search.seed_search_query_from_cursor",
                field: Box::new(SettingField {
                    json_path: Some("seed_search_query_from_cursor"),
                    pick: |settings_content| {
                        settings_content
                            .editor
                            .seed_search_query_from_cursor
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.seed_search_query_from_cursor = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn file_finder_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.file_finder"),
            // todo: null by default
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.include_ignored_in_search",
                description: "settings_page.desc.file_finder.include_ignored_in_search",
                field: Box::new(SettingField {
                    json_path: Some("file_finder.include_ignored"),
                    pick: |settings_content| {
                        settings_content
                            .file_finder
                            .as_ref()?
                            .include_ignored
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .file_finder
                            .get_or_insert_default()
                            .include_ignored = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_icons",
                description: "settings_page.desc.file_finder.file_icons",
                field: Box::new(SettingField {
                    json_path: Some("file_finder.file_icons"),
                    pick: |settings_content| {
                        settings_content.file_finder.as_ref()?.file_icons.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .file_finder
                            .get_or_insert_default()
                            .file_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.modal_max_width",
                description: "settings_page.desc.file_finder.modal_max_width",
                field: Box::new(SettingField {
                    json_path: Some("file_finder.modal_max_width"),
                    pick: |settings_content| {
                        settings_content
                            .file_finder
                            .as_ref()?
                            .modal_max_width
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .file_finder
                            .get_or_insert_default()
                            .modal_max_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.skip_focus_for_active_in_search",
                description: "settings_page.desc.file_finder.skip_focus_for_active_in_search",
                field: Box::new(SettingField {
                    json_path: Some("file_finder.skip_focus_for_active_in_search"),
                    pick: |settings_content| {
                        settings_content
                            .file_finder
                            .as_ref()?
                            .skip_focus_for_active_in_search
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .file_finder
                            .get_or_insert_default()
                            .skip_focus_for_active_in_search = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn file_scan_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.file_scan"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_scan_exclusions",
                description: "settings_page.desc.file_scan.file_scan_exclusions",
                field: Box::new(
                    SettingField {
                        json_path: Some("file_scan_exclusions"),
                        pick: |settings_content| {
                            settings_content
                                .project
                                .worktree
                                .file_scan_exclusions
                                .as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.project.worktree.file_scan_exclusions = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_scan_inclusions",
                description: "settings_page.desc.file_scan.file_scan_inclusions",
                field: Box::new(
                    SettingField {
                        json_path: Some("file_scan_inclusions"),
                        pick: |settings_content| {
                            settings_content
                                .project
                                .worktree
                                .file_scan_inclusions
                                .as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.project.worktree.file_scan_inclusions = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.restore_file_state",
                description: "settings_page.desc.file_scan.restore_file_state",
                field: Box::new(SettingField {
                    json_path: Some("restore_on_file_reopen"),
                    pick: |settings_content| {
                        settings_content.workspace.restore_on_file_reopen.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.restore_on_file_reopen = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.close_on_file_delete",
                description: "settings_page.desc.file_scan.close_on_file_delete",
                field: Box::new(SettingField {
                    json_path: Some("close_on_file_delete"),
                    pick: |settings_content| {
                        settings_content.workspace.close_on_file_delete.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.close_on_file_delete = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.search_files",
        items: concat_sections![search_section(), file_finder_section(), file_scan_section()],
    }
}

fn window_and_layout_page() -> SettingsPage {
    fn status_bar_section() -> [SettingsPageItem; 9] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.status_bar"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.project_panel_button",
                description: "settings_page.desc.status_bar.project_panel_button",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.button"),
                    pick: |settings_content| {
                        settings_content.project_panel.as_ref()?.button.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.active_language_button",
                description: "settings_page.desc.status_bar.active_language_button",
                field: Box::new(SettingField {
                    json_path: Some("status_bar.active_language_button"),
                    pick: |settings_content| {
                        settings_content
                            .status_bar
                            .as_ref()?
                            .active_language_button
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .status_bar
                            .get_or_insert_default()
                            .active_language_button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.active_encoding_button",
                description: "settings_page.desc.status_bar.active_encoding_button",
                field: Box::new(SettingField {
                    json_path: Some("status_bar.active_encoding_button"),
                    pick: |settings_content| {
                        settings_content
                            .status_bar
                            .as_ref()?
                            .active_encoding_button
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .status_bar
                            .get_or_insert_default()
                            .active_encoding_button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_position_button",
                description: "settings_page.desc.status_bar.cursor_position_button",
                field: Box::new(SettingField {
                    json_path: Some("status_bar.cursor_position_button"),
                    pick: |settings_content| {
                        settings_content
                            .status_bar
                            .as_ref()?
                            .cursor_position_button
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .status_bar
                            .get_or_insert_default()
                            .cursor_position_button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.terminal_button",
                description: "settings_page.desc.status_bar.terminal_button",
                field: Box::new(SettingField {
                    json_path: Some("terminal.button"),
                    pick: |settings_content| settings_content.terminal.as_ref()?.button.as_ref(),
                    write: |settings_content, value| {
                        settings_content.terminal.get_or_insert_default().button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.diagnostics_button",
                description: "settings_page.desc.status_bar.diagnostics_button",
                field: Box::new(SettingField {
                    json_path: Some("diagnostics.button"),
                    pick: |settings_content| settings_content.diagnostics.as_ref()?.button.as_ref(),
                    write: |settings_content, value| {
                        settings_content.diagnostics.get_or_insert_default().button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.project_search_button",
                description: "settings_page.desc.status_bar.project_search_button",
                field: Box::new(SettingField {
                    json_path: Some("search.button"),
                    pick: |settings_content| {
                        settings_content.editor.search.as_ref()?.button.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .editor
                            .search
                            .get_or_insert_default()
                            .button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.active_file_name",
                description: "settings_page.desc.status_bar.active_file_name",
                field: Box::new(SettingField {
                    json_path: Some("status_bar.show_active_file"),
                    pick: |settings_content| {
                        settings_content
                            .status_bar
                            .as_ref()?
                            .show_active_file
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .status_bar
                            .get_or_insert_default()
                            .show_active_file = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn tab_bar_section() -> [SettingsPageItem; 9] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.tab_bar"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_tab_bar",
                description: "settings_page.desc.tab_bar.show_tab_bar",
                field: Box::new(SettingField {
                    json_path: Some("tab_bar.show"),
                    pick: |settings_content| settings_content.tab_bar.as_ref()?.show.as_ref(),
                    write: |settings_content, value| {
                        settings_content.tab_bar.get_or_insert_default().show = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_git_status_in_tabs",
                description: "settings_page.desc.tab_bar.show_git_status_in_tabs",
                field: Box::new(SettingField {
                    json_path: Some("tabs.git_status"),
                    pick: |settings_content| settings_content.tabs.as_ref()?.git_status.as_ref(),
                    write: |settings_content, value| {
                        settings_content.tabs.get_or_insert_default().git_status = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_file_icons_in_tabs",
                description: "settings_page.desc.tab_bar.show_file_icons_in_tabs",
                field: Box::new(SettingField {
                    json_path: Some("tabs.file_icons"),
                    pick: |settings_content| settings_content.tabs.as_ref()?.file_icons.as_ref(),
                    write: |settings_content, value| {
                        settings_content.tabs.get_or_insert_default().file_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.tab_close_position",
                description: "settings_page.desc.tab_bar.tab_close_position",
                field: Box::new(SettingField {
                    json_path: Some("tabs.close_position"),
                    pick: |settings_content| {
                        settings_content.tabs.as_ref()?.close_position.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.tabs.get_or_insert_default().close_position = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.maximum_tabs",
                description: "settings_page.desc.tab_bar.maximum_tabs",
                // todo(settings_ui): The default for this value is null and it's use in code
                // is complex, so I'm going to come back to this later
                field: Box::new(
                    SettingField {
                        json_path: Some("max_tabs"),
                        pick: |settings_content| settings_content.workspace.max_tabs.as_ref(),
                        write: |settings_content, value| {
                            settings_content.workspace.max_tabs = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_navigation_history_buttons",
                description: "settings_page.desc.tab_bar.show_navigation_history_buttons",
                field: Box::new(SettingField {
                    json_path: Some("tab_bar.show_nav_history_buttons"),
                    pick: |settings_content| {
                        settings_content
                            .tab_bar
                            .as_ref()?
                            .show_nav_history_buttons
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .tab_bar
                            .get_or_insert_default()
                            .show_nav_history_buttons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_tab_bar_buttons",
                description: "settings_page.desc.tab_bar.show_tab_bar_buttons",
                field: Box::new(SettingField {
                    json_path: Some("tab_bar.show_tab_bar_buttons"),
                    pick: |settings_content| {
                        settings_content
                            .tab_bar
                            .as_ref()?
                            .show_tab_bar_buttons
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .tab_bar
                            .get_or_insert_default()
                            .show_tab_bar_buttons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.pinned_tabs_layout",
                description: "settings_page.desc.tab_bar.pinned_tabs_layout",
                field: Box::new(SettingField {
                    json_path: Some("tab_bar.show_pinned_tabs_in_separate_row"),
                    pick: |settings_content| {
                        settings_content
                            .tab_bar
                            .as_ref()?
                            .show_pinned_tabs_in_separate_row
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .tab_bar
                            .get_or_insert_default()
                            .show_pinned_tabs_in_separate_row = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn tab_settings_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.tab_settings"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.activate_on_close",
                description: "settings_page.desc.tab_settings.activate_on_close",
                field: Box::new(SettingField {
                    json_path: Some("tabs.activate_on_close"),
                    pick: |settings_content| {
                        settings_content.tabs.as_ref()?.activate_on_close.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .tabs
                            .get_or_insert_default()
                            .activate_on_close = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.tab_show_diagnostics",
                description: "settings_page.desc.tab_settings.tab_show_diagnostics",
                field: Box::new(SettingField {
                    json_path: Some("tabs.show_diagnostics"),
                    pick: |settings_content| {
                        settings_content.tabs.as_ref()?.show_diagnostics.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .tabs
                            .get_or_insert_default()
                            .show_diagnostics = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_close_button",
                description: "settings_page.desc.tab_settings.show_close_button",
                field: Box::new(SettingField {
                    json_path: Some("tabs.show_close_button"),
                    pick: |settings_content| {
                        settings_content.tabs.as_ref()?.show_close_button.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .tabs
                            .get_or_insert_default()
                            .show_close_button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn preview_tabs_section() -> [SettingsPageItem; 8] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.preview_tabs"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.preview_tabs_enabled",
                description: "settings_page.desc.preview_tabs.preview_tabs_enabled",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enabled"),
                    pick: |settings_content| {
                        settings_content.preview_tabs.as_ref()?.enabled.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_preview_from_project_panel",
                description: "settings_page.desc.preview_tabs.enable_preview_from_project_panel",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enable_preview_from_project_panel"),
                    pick: |settings_content| {
                        settings_content
                            .preview_tabs
                            .as_ref()?
                            .enable_preview_from_project_panel
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enable_preview_from_project_panel = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_preview_from_file_finder",
                description: "settings_page.desc.preview_tabs.enable_preview_from_file_finder",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enable_preview_from_file_finder"),
                    pick: |settings_content| {
                        settings_content
                            .preview_tabs
                            .as_ref()?
                            .enable_preview_from_file_finder
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enable_preview_from_file_finder = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_preview_from_multibuffer",
                description: "settings_page.desc.preview_tabs.enable_preview_from_multibuffer",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enable_preview_from_multibuffer"),
                    pick: |settings_content| {
                        settings_content
                            .preview_tabs
                            .as_ref()?
                            .enable_preview_from_multibuffer
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enable_preview_from_multibuffer = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_preview_multibuffer_from_code_navigation",
                description: "settings_page.desc.preview_tabs.enable_preview_multibuffer_from_code_navigation",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enable_preview_multibuffer_from_code_navigation"),
                    pick: |settings_content| {
                        settings_content
                            .preview_tabs
                            .as_ref()?
                            .enable_preview_multibuffer_from_code_navigation
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enable_preview_multibuffer_from_code_navigation = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_preview_file_from_code_navigation",
                description: "settings_page.desc.preview_tabs.enable_preview_file_from_code_navigation",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enable_preview_file_from_code_navigation"),
                    pick: |settings_content| {
                        settings_content
                            .preview_tabs
                            .as_ref()?
                            .enable_preview_file_from_code_navigation
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enable_preview_file_from_code_navigation = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_keep_preview_on_code_navigation",
                description: "settings_page.desc.preview_tabs.enable_keep_preview_on_code_navigation",
                field: Box::new(SettingField {
                    json_path: Some("preview_tabs.enable_keep_preview_on_code_navigation"),
                    pick: |settings_content| {
                        settings_content
                            .preview_tabs
                            .as_ref()?
                            .enable_keep_preview_on_code_navigation
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .preview_tabs
                            .get_or_insert_default()
                            .enable_keep_preview_on_code_navigation = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn layout_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.layout"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.bottom_dock_layout",
                description: "settings_page.desc.layout.bottom_dock_layout",
                field: Box::new(SettingField {
                    json_path: Some("bottom_dock_layout"),
                    pick: |settings_content| settings_content.workspace.bottom_dock_layout.as_ref(),
                    write: |settings_content, value| {
                        settings_content.workspace.bottom_dock_layout = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.centered_layout_left_padding",
                description: "settings_page.desc.layout.centered_layout_left_padding",
                field: Box::new(SettingField {
                    json_path: Some("centered_layout.left_padding"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .centered_layout
                            .as_ref()?
                            .left_padding
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .workspace
                            .centered_layout
                            .get_or_insert_default()
                            .left_padding = value;
                    },
                }),
                metadata: None,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.centered_layout_right_padding",
                description: "settings_page.desc.layout.centered_layout_right_padding",
                field: Box::new(SettingField {
                    json_path: Some("centered_layout.right_padding"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .centered_layout
                            .as_ref()?
                            .right_padding
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .workspace
                            .centered_layout
                            .get_or_insert_default()
                            .right_padding = value;
                    },
                }),
                metadata: None,
            }),
        ]
    }

    fn pane_modifiers_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.pane_modifiers"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.inactive_opacity",
                description: "settings_page.desc.pane_modifiers.inactive_opacity",
                field: Box::new(SettingField {
                    json_path: Some("active_pane_modifiers.inactive_opacity"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .active_pane_modifiers
                            .as_ref()?
                            .inactive_opacity
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .workspace
                            .active_pane_modifiers
                            .get_or_insert_default()
                            .inactive_opacity = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.border_size",
                description: "settings_page.desc.pane_modifiers.border_size",
                field: Box::new(SettingField {
                    json_path: Some("active_pane_modifiers.border_size"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .active_pane_modifiers
                            .as_ref()?
                            .border_size
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .workspace
                            .active_pane_modifiers
                            .get_or_insert_default()
                            .border_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.zoomed_padding",
                description: "settings_page.desc.pane_modifiers.zoomed_padding",
                field: Box::new(SettingField {
                    json_path: Some("zoomed_padding"),
                    pick: |settings_content| settings_content.workspace.zoomed_padding.as_ref(),
                    write: |settings_content, value| {
                        settings_content.workspace.zoomed_padding = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn pane_split_direction_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.pane_split_direction"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.vertical_split_direction",
                description: "settings_page.desc.pane_split_direction.vertical_split_direction",
                field: Box::new(SettingField {
                    json_path: Some("pane_split_direction_vertical"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .pane_split_direction_vertical
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.pane_split_direction_vertical = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.horizontal_split_direction",
                description: "settings_page.desc.pane_split_direction.horizontal_split_direction",
                field: Box::new(SettingField {
                    json_path: Some("pane_split_direction_horizontal"),
                    pick: |settings_content| {
                        settings_content
                            .workspace
                            .pane_split_direction_horizontal
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.workspace.pane_split_direction_horizontal = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.window_layout",
        items: concat_sections![
            status_bar_section(),
            tab_bar_section(),
            tab_settings_section(),
            preview_tabs_section(),
            layout_section(),
            pane_modifiers_section(),
            pane_split_direction_section(),
        ],
    }
}

fn panels_page() -> SettingsPage {
    fn project_panel_section() -> [SettingsPageItem; 26] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.project_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.project_panel_dock",
                description: "settings_page.desc.project_panel.project_panel_dock",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.dock"),
                    pick: |settings_content| settings_content.project_panel.as_ref()?.dock.as_ref(),
                    write: |settings_content, value| {
                        settings_content.project_panel.get_or_insert_default().dock = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.project_panel_default_width",
                description: "settings_page.desc.project_panel.project_panel_default_width",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.default_width"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .default_width
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hide_gitignore",
                description: "settings_page.desc.project_panel.hide_gitignore",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.hide_gitignore"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .hide_gitignore
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .hide_gitignore = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.entry_spacing",
                description: "settings_page.desc.project_panel.entry_spacing",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.entry_spacing"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .entry_spacing
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .entry_spacing = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_icons",
                description: "settings_page.desc.project_panel.file_icons",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.file_icons"),
                    pick: |settings_content| {
                        settings_content.project_panel.as_ref()?.file_icons.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .file_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.folder_icons",
                description: "settings_page.desc.project_panel.folder_icons",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.folder_icons"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .folder_icons
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .folder_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_status",
                description: "settings_page.desc.project_panel.git_status",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.git_status"),
                    pick: |settings_content| {
                        settings_content.project_panel.as_ref()?.git_status.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .git_status = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.indent_size",
                description: "settings_page.desc.project_panel.indent_size",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.indent_size"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .indent_size
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .indent_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_reveal_entries",
                description: "settings_page.desc.project_panel.auto_reveal_entries",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.auto_reveal_entries"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .auto_reveal_entries
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .auto_reveal_entries = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.starts_open",
                description: "settings_page.desc.project_panel.starts_open",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.starts_open"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .starts_open
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .starts_open = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_fold_directories",
                description: "settings_page.desc.project_panel.auto_fold_directories",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.auto_fold_dirs"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .auto_fold_dirs
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .auto_fold_dirs = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.bold_folder_labels",
                description: "settings_page.desc.project_panel.bold_folder_labels",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.bold_folder_labels"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .bold_folder_labels
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .bold_folder_labels = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_scrollbar",
                description: "settings_page.desc.project_panel.show_scrollbar",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.scrollbar.show"),
                    pick: |settings_content| {
                        show_scrollbar_or_editor(settings_content, |settings_content| {
                            settings_content
                                .project_panel
                                .as_ref()?
                                .scrollbar
                                .as_ref()?
                                .show
                                .as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .scrollbar
                            .get_or_insert_default()
                            .show = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.horizontal_scroll",
                description: "settings_page.desc.project_panel.horizontal_scroll",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.scrollbar.horizontal_scroll"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .scrollbar
                            .as_ref()?
                            .horizontal_scroll
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .scrollbar
                            .get_or_insert_default()
                            .horizontal_scroll = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_diagnostics",
                description: "settings_page.desc.project_panel.show_diagnostics",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.show_diagnostics"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .show_diagnostics
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .show_diagnostics = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.diagnostic_badges",
                description: "settings_page.desc.project_panel.diagnostic_badges",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.diagnostic_badges"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .diagnostic_badges
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .diagnostic_badges = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_status_indicator",
                description: "settings_page.desc.project_panel.git_status_indicator",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.git_status_indicator"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .git_status_indicator
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .git_status_indicator = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.sticky_scroll",
                description: "settings_page.desc.project_panel.sticky_scroll",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.sticky_scroll"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .sticky_scroll
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .sticky_scroll = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.show_indent_guides",
                description: "settings_page.desc.project_panel.show_indent_guides",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.indent_guides.show"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .indent_guides
                            .as_ref()?
                            .show
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .indent_guides
                            .get_or_insert_default()
                            .show = value;
                    },
                }),
                metadata: None,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.drag_and_drop",
                description: "settings_page.desc.project_panel.drag_and_drop",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.drag_and_drop"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .drag_and_drop
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .drag_and_drop = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hide_root",
                description: "settings_page.desc.project_panel.hide_root",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.hide_root"),
                    pick: |settings_content| {
                        settings_content.project_panel.as_ref()?.hide_root.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .hide_root = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hide_hidden",
                description: "settings_page.desc.project_panel.hide_hidden",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.hide_hidden"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .hide_hidden
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .hide_hidden = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hidden_files",
                description: "settings_page.desc.project_panel.hidden_files",
                field: Box::new(
                    SettingField {
                        json_path: Some("worktree.hidden_files"),
                        pick: |settings_content| {
                            settings_content.project.worktree.hidden_files.as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content.project.worktree.hidden_files = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
            // 정렬 모드·순서를 Project Panel 섹션 끝에 노출
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.sort_mode",
                description: "settings_page.desc.auto_open_files.sort_mode",
                field: Box::new(SettingField {
                    pick: |settings_content| {
                        settings_content.project_panel.as_ref()?.sort_mode.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .sort_mode = value;
                    },
                    json_path: Some("project_panel.sort_mode"),
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.sort_order",
                description: "settings_page.desc.project_panel.sort_order",
                field: Box::new(SettingField {
                    pick: |settings_content| {
                        settings_content.project_panel.as_ref()?.sort_order.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .sort_order = value;
                    },
                    json_path: Some("project_panel.sort_order"),
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn auto_open_files_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.auto_open_files"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.on_create",
                description: "settings_page.desc.auto_open_files.on_create",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.auto_open.on_create"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .auto_open
                            .as_ref()?
                            .on_create
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .auto_open
                            .get_or_insert_default()
                            .on_create = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.on_paste",
                description: "settings_page.desc.auto_open_files.on_paste",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.auto_open.on_paste"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .auto_open
                            .as_ref()?
                            .on_paste
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .auto_open
                            .get_or_insert_default()
                            .on_paste = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.on_drop",
                description: "settings_page.desc.auto_open_files.on_drop",
                field: Box::new(SettingField {
                    json_path: Some("project_panel.auto_open.on_drop"),
                    pick: |settings_content| {
                        settings_content
                            .project_panel
                            .as_ref()?
                            .auto_open
                            .as_ref()?
                            .on_drop
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project_panel
                            .get_or_insert_default()
                            .auto_open
                            .get_or_insert_default()
                            .on_drop = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn terminal_panel_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.terminal_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.terminal_dock",
                description: "settings_page.desc.terminal_panel.terminal_dock",
                field: Box::new(SettingField {
                    json_path: Some("terminal.dock"),
                    pick: |settings_content| settings_content.terminal.as_ref()?.dock.as_ref(),
                    write: |settings_content, value| {
                        settings_content.terminal.get_or_insert_default().dock = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_count_badge",
                description: "settings_page.desc.terminal_panel.show_count_badge",
                field: Box::new(SettingField {
                    json_path: Some("terminal.show_count_badge"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .show_count_badge
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .show_count_badge = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn outline_panel_section() -> [SettingsPageItem; 11] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.outline_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.outline_panel_button",
                description: "settings_page.desc.outline_panel.outline_panel_button",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.button"),
                    pick: |settings_content| {
                        settings_content.outline_panel.as_ref()?.button.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.outline_panel_dock",
                description: "settings_page.desc.outline_panel.outline_panel_dock",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.dock"),
                    pick: |settings_content| settings_content.outline_panel.as_ref()?.dock.as_ref(),
                    write: |settings_content, value| {
                        settings_content.outline_panel.get_or_insert_default().dock = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.outline_panel_default_width",
                description: "settings_page.desc.outline_panel.outline_panel_default_width",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.default_width"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()?
                            .default_width
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_icons",
                description: "settings_page.desc.outline_panel.file_icons",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.file_icons"),
                    pick: |settings_content| {
                        settings_content.outline_panel.as_ref()?.file_icons.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .file_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.folder_icons",
                description: "settings_page.desc.outline_panel.folder_icons",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.folder_icons"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()?
                            .folder_icons
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .folder_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_status",
                description: "settings_page.desc.outline_panel.git_status",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.git_status"),
                    pick: |settings_content| {
                        settings_content.outline_panel.as_ref()?.git_status.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .git_status = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.indent_size",
                description: "settings_page.desc.outline_panel.indent_size",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.indent_size"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()?
                            .indent_size
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .indent_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_reveal_entries",
                description: "settings_page.desc.outline_panel.auto_reveal_entries",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.auto_reveal_entries"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()?
                            .auto_reveal_entries
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .auto_reveal_entries = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_fold_directories",
                description: "settings_page.desc.outline_panel.auto_fold_directories",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.auto_fold_dirs"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()?
                            .auto_fold_dirs
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .auto_fold_dirs = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                files: USER,
                title: "settings_page.item.show_indent_guides",
                description: "settings_page.desc.outline_panel.show_indent_guides",
                field: Box::new(SettingField {
                    json_path: Some("outline_panel.indent_guides.show"),
                    pick: |settings_content| {
                        settings_content
                            .outline_panel
                            .as_ref()?
                            .indent_guides
                            .as_ref()?
                            .show
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .outline_panel
                            .get_or_insert_default()
                            .indent_guides
                            .get_or_insert_default()
                            .show = value;
                    },
                }),
                metadata: None,
            }),
        ]
    }

    fn git_panel_section() -> [SettingsPageItem; 15] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.git_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_panel_button",
                description: "settings_page.desc.git_panel.git_panel_button",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.button"),
                    pick: |settings_content| settings_content.git_panel.as_ref()?.button.as_ref(),
                    write: |settings_content, value| {
                        settings_content.git_panel.get_or_insert_default().button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_panel_dock",
                description: "settings_page.desc.git_panel.git_panel_dock",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.dock"),
                    pick: |settings_content| settings_content.git_panel.as_ref()?.dock.as_ref(),
                    write: |settings_content, value| {
                        settings_content.git_panel.get_or_insert_default().dock = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_panel_default_width",
                description: "settings_page.desc.git_panel.git_panel_default_width",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.default_width"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.default_width.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.git_panel_status_style",
                description: "settings_page.desc.git_panel.git_panel_status_style",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.status_style"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.status_style.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .status_style = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.fallback_branch_name",
                description: "settings_page.desc.git_panel.fallback_branch_name",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.fallback_branch_name"),
                    pick: |settings_content| {
                        settings_content
                            .git_panel
                            .as_ref()?
                            .fallback_branch_name
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .fallback_branch_name = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.sort_by_path",
                description: "settings_page.desc.git_panel.sort_by_path",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.sort_by_path"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.sort_by_path.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .sort_by_path = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.collapse_untracked_diff",
                description: "settings_page.desc.git_panel.collapse_untracked_diff",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.collapse_untracked_diff"),
                    pick: |settings_content| {
                        settings_content
                            .git_panel
                            .as_ref()?
                            .collapse_untracked_diff
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .collapse_untracked_diff = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.tree_view",
                description: "settings_page.desc.git_panel.tree_view",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.tree_view"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.tree_view.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.git_panel.get_or_insert_default().tree_view = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.file_icons",
                description: "settings_page.desc.git_panel.file_icons",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.file_icons"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.file_icons.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .file_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.folder_icons",
                description: "settings_page.desc.git_panel.folder_icons",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.folder_icons"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.folder_icons.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .folder_icons = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.diff_stats",
                description: "settings_page.desc.git_panel.diff_stats",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.diff_stats"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.diff_stats.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .diff_stats = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_count_badge",
                description: "settings_page.desc.git_panel.show_count_badge",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.show_count_badge"),
                    pick: |settings_content| {
                        settings_content
                            .git_panel
                            .as_ref()?
                            .show_count_badge
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .show_count_badge = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.scroll_bar",
                description: "settings_page.desc.git_panel.scroll_bar",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.scrollbar.show"),
                    pick: |settings_content| {
                        show_scrollbar_or_editor(settings_content, |settings_content| {
                            settings_content
                                .git_panel
                                .as_ref()?
                                .scrollbar
                                .as_ref()?
                                .show
                                .as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .scrollbar
                            .get_or_insert_default()
                            .show = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.starts_open",
                description: "settings_page.desc.git_panel.starts_open",
                field: Box::new(SettingField {
                    json_path: Some("git_panel.starts_open"),
                    pick: |settings_content| {
                        settings_content.git_panel.as_ref()?.starts_open.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git_panel
                            .get_or_insert_default()
                            .starts_open = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }


    fn agent_panel_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.agent_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.agent_panel_button",
                description: "settings_page.desc.agent_panel.agent_panel_button",
                field: Box::new(SettingField {
                    json_path: Some("agent.button"),
                    pick: |settings_content| settings_content.agent.as_ref()?.button.as_ref(),
                    write: |settings_content, value| {
                        settings_content.agent.get_or_insert_default().button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.agent_panel_dock",
                description: "settings_page.desc.agent_panel.agent_panel_dock",
                field: Box::new(SettingField {
                    json_path: Some("agent.dock"),
                    pick: |settings_content| settings_content.agent.as_ref()?.dock.as_ref(),
                    write: |settings_content, value| {
                        settings_content.agent.get_or_insert_default().dock = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.agent_panel_default_width",
                description: "settings_page.desc.agent_panel.agent_panel_default_width",
                field: Box::new(SettingField {
                    json_path: Some("agent.default_width"),
                    pick: |settings_content| {
                        settings_content.agent.as_ref()?.default_width.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.agent.get_or_insert_default().default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.agent_panel_default_height",
                description: "settings_page.desc.agent_panel.agent_panel_default_height",
                field: Box::new(SettingField {
                    json_path: Some("agent.default_height"),
                    pick: |settings_content| {
                        settings_content.agent.as_ref()?.default_height.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .default_height = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn notepad_panel_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.notepad_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings.notepad_panel.button.title",
                description: "settings.notepad_panel.button.description",
                field: Box::new(SettingField {
                    json_path: Some("notepad_panel.button"),
                    pick: |settings_content| {
                        settings_content
                            .notepad_panel
                            .as_ref()?
                            .button
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .notepad_panel
                            .get_or_insert_default()
                            .button = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings.notepad_panel.dock.title",
                description: "settings.notepad_panel.dock.description",
                field: Box::new(SettingField {
                    json_path: Some("notepad_panel.dock"),
                    pick: |settings_content| {
                        settings_content
                            .notepad_panel
                            .as_ref()?
                            .dock
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .notepad_panel
                            .get_or_insert_default()
                            .dock = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "notepad_panel.default_width",
                description: "notepad_panel.default_width.description",
                field: Box::new(SettingField {
                    json_path: Some("notepad_panel.default_width"),
                    pick: |settings_content| {
                        settings_content
                            .notepad_panel
                            .as_ref()?
                            .default_width
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .notepad_panel
                            .get_or_insert_default()
                            .default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings.notepad_panel.restore.title",
                description: "settings.notepad_panel.restore.description",
                field: Box::new(SettingField {
                    json_path: Some("notepad_panel.restore"),
                    pick: |settings_content| {
                        settings_content
                            .notepad_panel
                            .as_ref()?
                            .restore
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .notepad_panel
                            .get_or_insert_default()
                            .restore = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings.notepad_panel.horizontal_scroll.title",
                description: "settings.notepad_panel.horizontal_scroll.description",
                field: Box::new(SettingField {
                    json_path: Some("notepad_panel.horizontal_scroll"),
                    pick: |settings_content| {
                        settings_content
                            .notepad_panel
                            .as_ref()?
                            .horizontal_scroll
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .notepad_panel
                            .get_or_insert_default()
                            .horizontal_scroll = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn workspace_group_panel_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.workspace_panel"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.default_width",
                description: "settings_page.desc.workspace_panel.default_width",
                field: Box::new(SettingField {
                    json_path: Some("workspace_group_panel.default_width"),
                    pick: |settings_content| {
                        settings_content
                            .workspace_group_panel
                            .as_ref()?
                            .default_width
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .workspace_group_panel
                            .get_or_insert_default()
                            .default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.starts_open",
                description: "settings_page.desc.workspace_panel.starts_open",
                field: Box::new(SettingField {
                    json_path: Some("workspace_group_panel.starts_open"),
                    pick: |settings_content| {
                        settings_content
                            .workspace_group_panel
                            .as_ref()?
                            .starts_open
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .workspace_group_panel
                            .get_or_insert_default()
                            .starts_open = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.panels",
        items: concat_sections![
            project_panel_section(),
            auto_open_files_section(),
            terminal_panel_section(),
            outline_panel_section(),
            git_panel_section(),
            agent_panel_section(),
            notepad_panel_section(),
            workspace_group_panel_section(),
        ],
    }
}


fn terminal_page() -> SettingsPage {
    fn environment_section() -> [SettingsPageItem; 5] {
        [
                SettingsPageItem::SectionHeader("settings_page.section.environment"),
                SettingsPageItem::DynamicItem(DynamicItem {
                    discriminant: SettingItem {
                        files: USER | PROJECT,
                        title: "settings_page.item.shell",
                        description: "settings_page.desc.environment.shell",
                        field: Box::new(SettingField {
                            json_path: Some("terminal.shell$"),
                            pick: |settings_content| {
                                Some(&dynamic_variants::<settings::Shell>()[
                                    settings_content
                                        .terminal
                                        .as_ref()?
                                        .project
                                        .shell
                                        .as_ref()?
                                        .discriminant() as usize
                                ])
                            },
                            write: |settings_content, value| {
                                let Some(value) = value else {
                                    if let Some(terminal) = settings_content.terminal.as_mut() {
                                        terminal.project.shell = None;
                                    }
                                    return;
                                };
                                let settings_value = settings_content
                                    .terminal
                                    .get_or_insert_default()
                                    .project
                                    .shell
                                    .get_or_insert_with(|| settings::Shell::default());
                                let default_shell = if cfg!(target_os = "windows") {
                                    "powershell.exe"
                                } else {
                                    "sh"
                                };
                                *settings_value = match value {
                                    settings::ShellDiscriminants::System => settings::Shell::System,
                                    settings::ShellDiscriminants::Program => {
                                        let program = match settings_value {
                                            settings::Shell::Program(program) => program.clone(),
                                            settings::Shell::WithArguments { program, .. } => program.clone(),
                                            _ => String::from(default_shell),
                                        };
                                        settings::Shell::Program(program)
                                    }
                                    settings::ShellDiscriminants::WithArguments => {
                                        let (program, args, title_override) = match settings_value {
                                            settings::Shell::Program(program) => (program.clone(), vec![], None),
                                            settings::Shell::WithArguments {
                                                program,
                                                args,
                                                title_override,
                                            } => (program.clone(), args.clone(), title_override.clone()),
                                            _ => (String::from(default_shell), vec![], None),
                                        };
                                        settings::Shell::WithArguments {
                                            program,
                                            args,
                                            title_override,
                                        }
                                    }
                                    settings::ShellDiscriminants::Command => settings::Shell::Command,
                                };
                            },
                        }),
                        metadata: None,
                    },
                    pick_discriminant: |settings_content| {
                        Some(
                            settings_content
                                .terminal
                                .as_ref()?
                                .project
                                .shell
                                .as_ref()?
                                .discriminant() as usize,
                        )
                    },
                    fields: dynamic_variants::<settings::Shell>()
                        .into_iter()
                        .map(|variant| match variant {
                            settings::ShellDiscriminants::System => vec![],
                            settings::ShellDiscriminants::Program => vec![SettingItem {
                                files: USER | PROJECT,
                                title: "settings_page.item.program",
                                description: "settings_page.desc.environment.program",
                                field: Box::new(SettingField {
                                    json_path: Some("terminal.shell"),
                                    pick: |settings_content| match settings_content.terminal.as_ref()?.project.shell.as_ref()
                                    {
                                        Some(settings::Shell::Program(program)) => Some(program),
                                        _ => None,
                                    },
                                    write: |settings_content, value| {
                                        let Some(value) = value else {
                                            return;
                                        };
                                        match settings_content
                                            .terminal
                                            .get_or_insert_default()
                                            .project
                                            .shell
                                            .as_mut()
                                        {
                                            Some(settings::Shell::Program(program)) => *program = value,
                                            _ => return,
                                        }
                                    },
                                }),
                                metadata: None,
                            }],
                            settings::ShellDiscriminants::Command => vec![],
                            settings::ShellDiscriminants::WithArguments => vec![
                                SettingItem {
                                    files: USER | PROJECT,
                                    title: "settings_page.item.program",
                                    description: "settings_page.desc.environment.program",
                                    field: Box::new(SettingField {
                                        json_path: Some("terminal.shell.program"),
                                        pick: |settings_content| {
                                            match settings_content.terminal.as_ref()?.project.shell.as_ref() {
                                                Some(settings::Shell::WithArguments { program, .. }) => Some(program),
                                                _ => None,
                                            }
                                        },
                                        write: |settings_content, value| {
                                            let Some(value) = value else {
                                                return;
                                            };
                                            match settings_content
                                                .terminal
                                                .get_or_insert_default()
                                                .project
                                                .shell
                                                .as_mut()
                                            {
                                                Some(settings::Shell::WithArguments { program, .. }) => {
                                                    *program = value
                                                }
                                                _ => return,
                                            }
                                        },
                                    }),
                                    metadata: None,
                                },
                                SettingItem {
                                    files: USER | PROJECT,
                                    title: "settings_page.item.arguments",
                                    description: "settings_page.desc.environment.arguments",
                                    field: Box::new(
                                        SettingField {
                                            json_path: Some("terminal.shell.args"),
                                            pick: |settings_content| {
                                                match settings_content.terminal.as_ref()?.project.shell.as_ref() {
                                                    Some(settings::Shell::WithArguments { args, .. }) => Some(args),
                                                    _ => None,
                                                }
                                            },
                                            write: |settings_content, value| {
                                                let Some(value) = value else {
                                                    return;
                                                };
                                                match settings_content
                                                    .terminal
                                                    .get_or_insert_default()
                                                    .project
                                                    .shell
                                                    .as_mut()
                                                {
                                                    Some(settings::Shell::WithArguments { args, .. }) => *args = value,
                                                    _ => return,
                                                }
                                            },
                                        }
                                        .unimplemented(),
                                    ),
                                    metadata: None,
                                },
                                SettingItem {
                                    files: USER | PROJECT,
                                    title: "settings_page.item.title_override",
                                    description: "settings_page.desc.environment.title_override",
                                    field: Box::new(SettingField {
                                        json_path: Some("terminal.shell.title_override"),
                                        pick: |settings_content| {
                                            match settings_content.terminal.as_ref()?.project.shell.as_ref() {
                                                Some(settings::Shell::WithArguments { title_override, .. }) => {
                                                    title_override.as_ref().or(DEFAULT_EMPTY_STRING)
                                                }
                                                _ => None,
                                            }
                                        },
                                        write: |settings_content, value| {
                                            match settings_content
                                                .terminal
                                                .get_or_insert_default()
                                                .project
                                                .shell
                                                .as_mut()
                                            {
                                                Some(settings::Shell::WithArguments { title_override, .. }) => {
                                                    *title_override = value.filter(|s| !s.is_empty())
                                                }
                                                _ => return,
                                            }
                                        },
                                    }),
                                    metadata: None,
                                },
                            ],
                        })
                        .collect(),
                }),
                SettingsPageItem::DynamicItem(DynamicItem {
                    discriminant: SettingItem {
                        files: USER | PROJECT,
                        title: "settings_page.item.working_directory",
                        description: "settings_page.desc.environment.working_directory",
                        field: Box::new(SettingField {
                            json_path: Some("terminal.working_directory$"),
                            pick: |settings_content| {
                                Some(&dynamic_variants::<settings::WorkingDirectory>()[
                                    settings_content
                                        .terminal
                                        .as_ref()?
                                        .project
                                        .working_directory
                                        .as_ref()?
                                        .discriminant() as usize
                                ])
                            },
                            write: |settings_content, value| {
                                let Some(value) = value else {
                                    if let Some(terminal) = settings_content.terminal.as_mut() {
                                        terminal.project.working_directory = None;
                                    }
                                    return;
                                };
                                let settings_value = settings_content
                                    .terminal
                                    .get_or_insert_default()
                                    .project
                                    .working_directory
                                    .get_or_insert_with(|| settings::WorkingDirectory::CurrentProjectDirectory);
                                *settings_value = match value {
                                    settings::WorkingDirectoryDiscriminants::CurrentFileDirectory => {
                                        settings::WorkingDirectory::CurrentFileDirectory
                                    },
                                    settings::WorkingDirectoryDiscriminants::CurrentProjectDirectory => {
                                        settings::WorkingDirectory::CurrentProjectDirectory
                                    }
                                    settings::WorkingDirectoryDiscriminants::FirstProjectDirectory => {
                                        settings::WorkingDirectory::FirstProjectDirectory
                                    }
                                    settings::WorkingDirectoryDiscriminants::AlwaysHome => {
                                        settings::WorkingDirectory::AlwaysHome
                                    }
                                    settings::WorkingDirectoryDiscriminants::Always => {
                                        let directory = match settings_value {
                                            settings::WorkingDirectory::Always { .. } => return,
                                            _ => String::new(),
                                        };
                                        settings::WorkingDirectory::Always { directory }
                                    }
                                };
                            },
                        }),
                        metadata: None,
                    },
                    pick_discriminant: |settings_content| {
                        Some(
                            settings_content
                                .terminal
                                .as_ref()?
                                .project
                                .working_directory
                                .as_ref()?
                                .discriminant() as usize,
                        )
                    },
                    fields: dynamic_variants::<settings::WorkingDirectory>()
                        .into_iter()
                        .map(|variant| match variant {
                            settings::WorkingDirectoryDiscriminants::CurrentFileDirectory => vec![],
                            settings::WorkingDirectoryDiscriminants::CurrentProjectDirectory => vec![],
                            settings::WorkingDirectoryDiscriminants::FirstProjectDirectory => vec![],
                            settings::WorkingDirectoryDiscriminants::AlwaysHome => vec![],
                            settings::WorkingDirectoryDiscriminants::Always => vec![SettingItem {
                                files: USER | PROJECT,
                                title: "settings_page.item.directory",
                                description: "settings_page.desc.environment.directory",
                                field: Box::new(SettingField {
                                    json_path: Some("terminal.working_directory.always"),
                                    pick: |settings_content| {
                                        match settings_content.terminal.as_ref()?.project.working_directory.as_ref() {
                                            Some(settings::WorkingDirectory::Always { directory }) => Some(directory),
                                            _ => None,
                                        }
                                    },
                                    write: |settings_content, value| {
                                        let value = value.unwrap_or_default();
                                        match settings_content
                                            .terminal
                                            .get_or_insert_default()
                                            .project
                                            .working_directory
                                            .as_mut()
                                        {
                                            Some(settings::WorkingDirectory::Always { directory }) => *directory = value,
                                            _ => return,
                                        }
                                    },
                                }),
                                metadata: None,
                            }],
                        })
                        .collect(),
                }),
                SettingsPageItem::SettingItem(SettingItem {
                    title: "settings_page.item.environment_variables",
                    description: "settings_page.desc.environment.environment_variables",
                    field: Box::new(
                        SettingField {
                            json_path: Some("terminal.env"),
                            pick: |settings_content| settings_content.terminal.as_ref()?.project.env.as_ref(),
                            write: |settings_content, value| {
                                settings_content.terminal.get_or_insert_default().project.env = value;
                            },
                        }
                        .unimplemented(),
                    ),
                    metadata: None,
                    files: USER | PROJECT,
                }),
                SettingsPageItem::SettingItem(SettingItem {
                    title: "settings_page.item.detect_virtual_environment",
                    description: "settings_page.desc.environment.detect_virtual_environment",
                    field: Box::new(
                        SettingField {
                            json_path: Some("terminal.detect_venv"),
                            pick: |settings_content| settings_content.terminal.as_ref()?.project.detect_venv.as_ref(),
                            write: |settings_content, value| {
                                settings_content
                                    .terminal
                                    .get_or_insert_default()
                                    .project
                                    .detect_venv = value;
                            },
                        }
                        .unimplemented(),
                    ),
                    metadata: None,
                    files: USER | PROJECT,
                }),
            ]
    }

    fn font_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.font"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_size",
                description: "settings_page.desc.font.font_size",
                field: Box::new(SettingField {
                    json_path: Some("terminal.font_size"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()
                            .and_then(|terminal| terminal.font_size.as_ref())
                            .or(settings_content.theme.buffer_font_size.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.terminal.get_or_insert_default().font_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_family",
                description: "settings_page.desc.font.font_family",
                field: Box::new(SettingField {
                    json_path: Some("terminal.font_family"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()
                            .and_then(|terminal| terminal.font_family.as_ref())
                            .or(settings_content.theme.buffer_font_family.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .font_family = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_fallbacks",
                description: "settings_page.desc.font.font_fallbacks",
                field: Box::new(
                    SettingField {
                        json_path: Some("terminal.font_fallbacks"),
                        pick: |settings_content| {
                            settings_content
                                .terminal
                                .as_ref()
                                .and_then(|terminal| terminal.font_fallbacks.as_ref())
                                .or(settings_content.theme.buffer_font_fallbacks.as_ref())
                        },
                        write: |settings_content, value| {
                            settings_content
                                .terminal
                                .get_or_insert_default()
                                .font_fallbacks = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_weight",
                description: "settings_page.desc.font.font_weight",
                field: Box::new(SettingField {
                    json_path: Some("terminal.font_weight"),
                    pick: |settings_content| {
                        settings_content.terminal.as_ref()?.font_weight.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .font_weight = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.font_features",
                description: "settings_page.desc.font.font_features",
                field: Box::new(
                    SettingField {
                        json_path: Some("terminal.font_features"),
                        pick: |settings_content| {
                            settings_content
                                .terminal
                                .as_ref()
                                .and_then(|terminal| terminal.font_features.as_ref())
                                .or(settings_content.theme.buffer_font_features.as_ref())
                        },
                        write: |settings_content, value| {
                            settings_content
                                .terminal
                                .get_or_insert_default()
                                .font_features = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn display_settings_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.display_settings"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.line_height",
                description: "settings_page.desc.display_settings.line_height",
                field: Box::new(
                    SettingField {
                        json_path: Some("terminal.line_height"),
                        pick: |settings_content| {
                            settings_content.terminal.as_ref()?.line_height.as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content
                                .terminal
                                .get_or_insert_default()
                                .line_height = value;
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_shape",
                description: "settings_page.desc.display_settings.cursor_shape",
                field: Box::new(SettingField {
                    json_path: Some("terminal.cursor_shape"),
                    pick: |settings_content| {
                        settings_content.terminal.as_ref()?.cursor_shape.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .cursor_shape = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cursor_blinking",
                description: "settings_page.desc.display_settings.cursor_blinking",
                field: Box::new(SettingField {
                    json_path: Some("terminal.blinking"),
                    pick: |settings_content| settings_content.terminal.as_ref()?.blinking.as_ref(),
                    write: |settings_content, value| {
                        settings_content.terminal.get_or_insert_default().blinking = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.alternate_scroll",
                description: "settings_page.desc.display_settings.alternate_scroll",
                field: Box::new(SettingField {
                    json_path: Some("terminal.alternate_scroll"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .alternate_scroll
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .alternate_scroll = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.minimum_contrast",
                description: "settings_page.desc.display_settings.minimum_contrast",
                field: Box::new(SettingField {
                    json_path: Some("terminal.minimum_contrast"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .minimum_contrast
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .minimum_contrast = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn behavior_settings_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.behavior_settings"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.copy_on_select",
                description: "settings_page.desc.behavior_settings.copy_on_select",
                field: Box::new(SettingField {
                    json_path: Some("terminal.copy_on_select"),
                    pick: |settings_content| {
                        settings_content.terminal.as_ref()?.copy_on_select.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .copy_on_select = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.keep_selection_on_copy",
                description: "settings_page.desc.behavior_settings.keep_selection_on_copy",
                field: Box::new(SettingField {
                    json_path: Some("terminal.keep_selection_on_copy"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .keep_selection_on_copy
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .keep_selection_on_copy = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn layout_settings_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.layout_settings"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.default_width",
                description: "settings_page.desc.layout_settings.default_width",
                field: Box::new(SettingField {
                    json_path: Some("terminal.default_width"),
                    pick: |settings_content| {
                        settings_content.terminal.as_ref()?.default_width.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .default_width = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.default_height",
                description: "settings_page.desc.layout_settings.default_height",
                field: Box::new(SettingField {
                    json_path: Some("terminal.default_height"),
                    pick: |settings_content| {
                        settings_content.terminal.as_ref()?.default_height.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .default_height = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn advanced_settings_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.advanced_settings"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.max_scroll_history_lines",
                description: "settings_page.desc.advanced_settings.max_scroll_history_lines",
                field: Box::new(SettingField {
                    json_path: Some("terminal.max_scroll_history_lines"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .max_scroll_history_lines
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .max_scroll_history_lines = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.scroll_multiplier",
                description: "settings_page.desc.advanced_settings.scroll_multiplier",
                field: Box::new(SettingField {
                    json_path: Some("terminal.scroll_multiplier"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .scroll_multiplier
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .scroll_multiplier = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn toolbar_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.toolbar"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.breadcrumbs",
                description: "settings_page.desc.terminal_toolbar.breadcrumbs",
                field: Box::new(SettingField {
                    json_path: Some("terminal.toolbar.breadcrumbs"),
                    pick: |settings_content| {
                        settings_content
                            .terminal
                            .as_ref()?
                            .toolbar
                            .as_ref()?
                            .breadcrumbs
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .toolbar
                            .get_or_insert_default()
                            .breadcrumbs = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn scrollbar_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.scrollbar"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_scrollbar",
                description: "settings_page.desc.terminal_scrollbar.show_scrollbar",
                field: Box::new(SettingField {
                    json_path: Some("terminal.scrollbar.show"),
                    pick: |settings_content| {
                        show_scrollbar_or_editor(settings_content, |settings_content| {
                            settings_content
                                .terminal
                                .as_ref()?
                                .scrollbar
                                .as_ref()?
                                .show
                                .as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        settings_content
                            .terminal
                            .get_or_insert_default()
                            .scrollbar
                            .get_or_insert_default()
                            .show = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.terminal",
        items: concat_sections![
            environment_section(),
            font_section(),
            display_settings_section(),
            behavior_settings_section(),
            layout_settings_section(),
            advanced_settings_section(),
            toolbar_section(),
            scrollbar_section(),
        ],
    }
}

fn version_control_page() -> SettingsPage {
    fn git_integration_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.git_integration"),
            SettingsPageItem::DynamicItem(DynamicItem {
                discriminant: SettingItem {
                    files: USER,
                    title: "settings_page.item.disable_git_integration",
                    description: "settings_page.desc.git_integration.disable_git_integration",
                    field: Box::new(SettingField::<bool> {
                        json_path: Some("git.disable_git"),
                        pick: |settings_content| {
                            settings_content
                                .git
                                .as_ref()?
                                .enabled
                                .as_ref()?
                                .disable_git
                                .as_ref()
                        },
                        write: |settings_content, value| {
                            settings_content
                                .git
                                .get_or_insert_default()
                                .enabled
                                .get_or_insert_default()
                                .disable_git = value;
                        },
                    }),
                    metadata: None,
                },
                pick_discriminant: |settings_content| {
                    let disabled = settings_content
                        .git
                        .as_ref()?
                        .enabled
                        .as_ref()?
                        .disable_git
                        .unwrap_or(false);
                    Some(if disabled { 0 } else { 1 })
                },
                fields: vec![
                    vec![],
                    vec![
                        SettingItem {
                            files: USER,
                            title: "settings_page.item.enable_git_status",
                            description: "settings_page.desc.git_integration.enable_git_status",
                            field: Box::new(SettingField::<bool> {
                                json_path: Some("git.enable_status"),
                                pick: |settings_content| {
                                    settings_content
                                        .git
                                        .as_ref()?
                                        .enabled
                                        .as_ref()?
                                        .enable_status
                                        .as_ref()
                                },
                                write: |settings_content, value| {
                                    settings_content
                                        .git
                                        .get_or_insert_default()
                                        .enabled
                                        .get_or_insert_default()
                                        .enable_status = value;
                                },
                            }),
                            metadata: None,
                        },
                        SettingItem {
                            files: USER,
                            title: "settings_page.item.enable_git_diff",
                            description: "settings_page.desc.git_integration.enable_git_diff",
                            field: Box::new(SettingField::<bool> {
                                json_path: Some("git.enable_diff"),
                                pick: |settings_content| {
                                    settings_content
                                        .git
                                        .as_ref()?
                                        .enabled
                                        .as_ref()?
                                        .enable_diff
                                        .as_ref()
                                },
                                write: |settings_content, value| {
                                    settings_content
                                        .git
                                        .get_or_insert_default()
                                        .enabled
                                        .get_or_insert_default()
                                        .enable_diff = value;
                                },
                            }),
                            metadata: None,
                        },
                    ],
                ],
            }),
        ]
    }

    fn git_gutter_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.git_gutter"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.visibility",
                description: "settings_page.desc.git_gutter.visibility",
                field: Box::new(SettingField {
                    json_path: Some("git.git_gutter"),
                    pick: |settings_content| settings_content.git.as_ref()?.git_gutter.as_ref(),
                    write: |settings_content, value| {
                        settings_content.git.get_or_insert_default().git_gutter = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            // todo(settings_ui): Figure out the right default for this value in default.json
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.debounce",
                description: "settings_page.desc.git_gutter.debounce",
                field: Box::new(SettingField {
                    json_path: Some("git.gutter_debounce"),
                    pick: |settings_content| {
                        settings_content.git.as_ref()?.gutter_debounce.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.git.get_or_insert_default().gutter_debounce = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn inline_git_blame_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.inline_git_blame"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.inline_git_blame.enabled",
                field: Box::new(SettingField {
                    json_path: Some("git.inline_blame.enabled"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .inline_blame
                            .as_ref()?
                            .enabled
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .inline_blame
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.delay",
                description: "settings_page.desc.inline_git_blame.delay",
                field: Box::new(SettingField {
                    json_path: Some("git.inline_blame.delay_ms"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .inline_blame
                            .as_ref()?
                            .delay_ms
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .inline_blame
                            .get_or_insert_default()
                            .delay_ms = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.padding",
                description: "settings_page.desc.inline_git_blame.padding",
                field: Box::new(SettingField {
                    json_path: Some("git.inline_blame.padding"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .inline_blame
                            .as_ref()?
                            .padding
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .inline_blame
                            .get_or_insert_default()
                            .padding = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.minimum_column",
                description: "settings_page.desc.inline_git_blame.minimum_column",
                field: Box::new(SettingField {
                    json_path: Some("git.inline_blame.min_column"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .inline_blame
                            .as_ref()?
                            .min_column
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .inline_blame
                            .get_or_insert_default()
                            .min_column = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_commit_summary",
                description: "settings_page.desc.inline_git_blame.show_commit_summary",
                field: Box::new(SettingField {
                    json_path: Some("git.inline_blame.show_commit_summary"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .inline_blame
                            .as_ref()?
                            .show_commit_summary
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .inline_blame
                            .get_or_insert_default()
                            .show_commit_summary = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn git_blame_view_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.git_blame_view"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_avatar",
                description: "settings_page.desc.git_blame_view.show_avatar",
                field: Box::new(SettingField {
                    json_path: Some("git.blame.show_avatar"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .blame
                            .as_ref()?
                            .show_avatar
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .blame
                            .get_or_insert_default()
                            .show_avatar = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn branch_picker_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.branch_picker"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_author_name",
                description: "settings_page.desc.branch_picker.show_author_name",
                field: Box::new(SettingField {
                    json_path: Some("git.branch_picker.show_author_name"),
                    pick: |settings_content| {
                        settings_content
                            .git
                            .as_ref()?
                            .branch_picker
                            .as_ref()?
                            .show_author_name
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .git
                            .get_or_insert_default()
                            .branch_picker
                            .get_or_insert_default()
                            .show_author_name = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn git_hunks_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.git_hunks"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hunk_style",
                description: "settings_page.desc.git_hunks.hunk_style",
                field: Box::new(SettingField {
                    json_path: Some("git.hunk_style"),
                    pick: |settings_content| settings_content.git.as_ref()?.hunk_style.as_ref(),
                    write: |settings_content, value| {
                        settings_content.git.get_or_insert_default().hunk_style = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.path_style",
                description: "settings_page.desc.git_hunks.path_style",
                field: Box::new(SettingField {
                    json_path: Some("git.path_style"),
                    pick: |settings_content| settings_content.git.as_ref()?.path_style.as_ref(),
                    write: |settings_content, value| {
                        settings_content.git.get_or_insert_default().path_style = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.version_control",
        items: concat_sections![
            git_integration_section(),
            git_gutter_section(),
            inline_git_blame_section(),
            git_blame_view_section(),
            branch_picker_section(),
            git_hunks_section(),
        ],
    }
}


fn ai_page(cx: &App) -> SettingsPage {
    fn general_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.general"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.disable_ai",
                description: "settings_page.desc.general.disable_ai",
                field: Box::new(SettingField {
                    json_path: Some("disable_ai"),
                    pick: |settings_content| settings_content.project.disable_ai.as_ref(),
                    write: |settings_content, value| {
                        settings_content.project.disable_ai = value;
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn agent_configuration_section(cx: &App) -> Box<[SettingsPageItem]> {
        let mut items = vec![
            SettingsPageItem::SectionHeader("settings_page.section.agent_configuration"),
            SettingsPageItem::SubPageLink(SubPageLink {
                title: "settings_page.item.tool_permissions".into(),
                r#type: Default::default(),
                json_path: Some("agent.tool_permissions"),
                description: Some("settings_page.desc.agent_configuration.tool_permissions".into()),
                in_json: true,
                files: USER,
                render: render_tool_permissions_setup_page,
            }),
        ];

        if cx.has_flag::<AgentV2FeatureFlag>() {
            items.push(SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.new_thread_location",
                description: "settings_page.desc.agent_configuration.new_thread_location",
                field: Box::new(SettingField {
                    json_path: Some("agent.new_thread_location"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .new_thread_location
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .new_thread_location = value;
                    },
                }),
                metadata: None,
                files: USER,
            }));
        }

        items.extend([
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.single_file_review",
                description: "settings_page.desc.agent_configuration.single_file_review",
                field: Box::new(SettingField {
                    json_path: Some("agent.single_file_review"),
                    pick: |settings_content| {
                        settings_content.agent.as_ref()?.single_file_review.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .single_file_review = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_feedback",
                description: "settings_page.desc.agent_configuration.enable_feedback",
                field: Box::new(SettingField {
                    json_path: Some("agent.enable_feedback"),
                    pick: |settings_content| {
                        settings_content.agent.as_ref()?.enable_feedback.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .enable_feedback = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.notify_when_agent_waiting",
                description: "settings_page.desc.agent_configuration.notify_when_agent_waiting",
                field: Box::new(SettingField {
                    json_path: Some("agent.notify_when_agent_waiting"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .notify_when_agent_waiting
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .notify_when_agent_waiting = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.play_sound_when_agent_done",
                description: "settings_page.desc.agent_configuration.play_sound_when_agent_done",
                field: Box::new(SettingField {
                    json_path: Some("agent.play_sound_when_agent_done"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .play_sound_when_agent_done
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .play_sound_when_agent_done = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.expand_edit_card",
                description: "settings_page.desc.agent_configuration.expand_edit_card",
                field: Box::new(SettingField {
                    json_path: Some("agent.expand_edit_card"),
                    pick: |settings_content| {
                        settings_content.agent.as_ref()?.expand_edit_card.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .expand_edit_card = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.expand_terminal_card",
                description: "settings_page.desc.agent_configuration.expand_terminal_card",
                field: Box::new(SettingField {
                    json_path: Some("agent.expand_terminal_card"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .expand_terminal_card
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .expand_terminal_card = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.cancel_generation_on_terminal_stop",
                description: "settings_page.desc.agent_configuration.cancel_generation_on_terminal_stop",
                field: Box::new(SettingField {
                    json_path: Some("agent.cancel_generation_on_terminal_stop"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .cancel_generation_on_terminal_stop
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .cancel_generation_on_terminal_stop = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_modifier_to_send",
                description: "settings_page.desc.agent_configuration.use_modifier_to_send",
                field: Box::new(SettingField {
                    json_path: Some("agent.use_modifier_to_send"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .use_modifier_to_send
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .use_modifier_to_send = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.message_editor_min_lines",
                description: "settings_page.desc.agent_configuration.message_editor_min_lines",
                field: Box::new(SettingField {
                    json_path: Some("agent.message_editor_min_lines"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .message_editor_min_lines
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .message_editor_min_lines = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_turn_stats",
                description: "settings_page.desc.agent_configuration.show_turn_stats",
                field: Box::new(SettingField {
                    json_path: Some("agent.show_turn_stats"),
                    pick: |settings_content| {
                        settings_content.agent.as_ref()?.show_turn_stats.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .show_turn_stats = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            // 상태바 병합 충돌 indicator 표시 여부
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_merge_conflict_indicator",
                description: "settings_page.desc.agent.show_merge_conflict_indicator",
                field: Box::new(SettingField {
                    json_path: Some("agent.show_merge_conflict_indicator"),
                    pick: |settings_content| {
                        settings_content
                            .agent
                            .as_ref()?
                            .show_merge_conflict_indicator
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .agent
                            .get_or_insert_default()
                            .show_merge_conflict_indicator = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]);

        items.into_boxed_slice()
    }

    fn context_servers_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.context_servers"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.context_server_timeout",
                description: "settings_page.desc.context_servers.context_server_timeout",
                field: Box::new(SettingField {
                    json_path: Some("context_server_timeout"),
                    pick: |settings_content| {
                        settings_content.project.context_server_timeout.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.project.context_server_timeout = value;
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn edit_prediction_display_sub_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.display_mode",
                description: "settings_page.desc.edit_prediction_display_sub.display_mode",
                field: Box::new(SettingField {
                    json_path: Some("edit_prediction.display_mode"),
                    pick: |settings_content| {
                        settings_content
                            .project
                            .all_languages
                            .edit_predictions
                            .as_ref()?
                            .mode
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project
                            .all_languages
                            .edit_predictions
                            .get_or_insert_default()
                            .mode = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.display_in_text_threads",
                description: "settings_page.desc.edit_prediction_display_sub.display_in_text_threads",
                field: Box::new(SettingField {
                    json_path: Some("edit_prediction.in_text_threads"),
                    pick: |settings_content| {
                        settings_content
                            .project
                            .all_languages
                            .edit_predictions
                            .as_ref()?
                            .enabled_in_text_threads
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project
                            .all_languages
                            .edit_predictions
                            .get_or_insert_default()
                            .enabled_in_text_threads = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.ai",
        items: concat_sections![
            general_section(),
            agent_configuration_section(cx),
            context_servers_section(),
            edit_prediction_language_settings_section(),
            edit_prediction_display_sub_section()
        ],
    }
}

fn notification_page() -> SettingsPage {
    static DEFAULT_BELL: bool = false;

    fn claude_code_section() -> [SettingsPageItem; 2] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.claude_code"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.claude_code_task_completion_bell",
                description:
                    "When Claude Code completes a task, sends a bell signal to the terminal to show a notification indicator on the workspace tab. Modifies ~/.claude/settings.json Stop hook.",
                field: Box::new(SettingField {
                    json_path: Some("notification.claude_code_bell"),
                    pick: |settings_content| {
                        Some(
                            settings_content
                                .notification
                                .as_ref()
                                .and_then(|n| n.claude_code_bell.as_ref())
                                .unwrap_or(&DEFAULT_BELL),
                        )
                    },
                    write: |settings_content, value| {
                        settings_content
                            .notification
                            .get_or_insert_with(Default::default)
                            .claude_code_bell = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    SettingsPage {
        title: "settings_page.item.notifications",
        items: concat_sections!(claude_code_section()),
    }
}

fn wallpaper_page() -> SettingsPage {
    // wallpaper 섹션이 없을 때 사용할 기본값
    static DEFAULT_ENABLED: bool = false;
    static DEFAULT_OBJECT_FIT: WallpaperFitContent = WallpaperFitContent::Cover;
    static DEFAULT_OPACITY: WallpaperOpacity = WallpaperOpacity(0.85);

    SettingsPage {
        title: "settings_page.item.wallpaper",
        items: vec![
            SettingsPageItem::SectionHeader("settings_page.section.wallpaper"),
            // 배경화면 활성화 토글
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_wallpaper",
                description: "settings_page.desc.wallpaper.enable_wallpaper",
                field: Box::new(SettingField {
                    json_path: Some("wallpaper.enabled"),
                    pick: |settings_content| {
                        Some(
                            settings_content
                                .wallpaper
                                .as_ref()
                                .and_then(|w| w.enabled.as_ref())
                                .unwrap_or(&DEFAULT_ENABLED),
                        )
                    },
                    write: |settings_content, value| {
                        settings_content
                            .wallpaper
                            .get_or_insert_default()
                            .enabled = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            // 이미지 파일 선택 다이얼로그
            SettingsPageItem::ActionLink(ActionLink {
                title: "settings_page.item.image_path".into(),
                description: Some("settings_page.desc.wallpaper.image_path".into()),
                button_text: "settings_page.button.browse".into(),
                on_click: Arc::new(|_settings_window, window, cx| {
                    let receiver = cx.prompt_for_paths(PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: None,
                    });
                    window
                        .spawn(cx, async move |cx| {
                            if let Ok(Ok(Some(paths))) = receiver.await {
                                if let Some(path) = paths.first() {
                                    let path_str = path.to_string_lossy().to_string();
                                    cx.update(|_window, cx| {
                                        SettingsStore::global(cx).update_settings_file(
                                            <dyn fs::Fs>::global(cx),
                                            move |settings, _cx| {
                                                settings
                                                    .wallpaper
                                                    .get_or_insert_default()
                                                    .image_path = Some(path_str);
                                            },
                                        );
                                    })
                                    .ok();
                                }
                            }
                        })
                        .detach();
                }),
                files: USER,
            }),
            // 맞춤 방식 드롭다운
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.object_fit",
                description: "settings_page.desc.wallpaper.object_fit",
                field: Box::new(SettingField {
                    json_path: Some("wallpaper.object_fit"),
                    pick: |settings_content| {
                        Some(
                            settings_content
                                .wallpaper
                                .as_ref()
                                .and_then(|w| w.object_fit.as_ref())
                                .unwrap_or(&DEFAULT_OBJECT_FIT),
                        )
                    },
                    write: |settings_content, value| {
                        settings_content
                            .wallpaper
                            .get_or_insert_default()
                            .object_fit = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            // 배경 불투명도 스테퍼
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.background_opacity",
                description: "settings_page.desc.wallpaper.background_opacity",
                field: Box::new(SettingField {
                    json_path: Some("wallpaper.opacity"),
                    pick: |settings_content| {
                        Some(
                            settings_content
                                .wallpaper
                                .as_ref()
                                .and_then(|w| w.opacity.as_ref())
                                .unwrap_or(&DEFAULT_OPACITY),
                        )
                    },
                    write: |settings_content, value| {
                        settings_content
                            .wallpaper
                            .get_or_insert_default()
                            .opacity = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
        .into_boxed_slice(),
    }
}

fn language_settings_field<T>(
    settings_content: &SettingsContent,
    get_language_setting_field: fn(&LanguageSettingsContent) -> Option<&T>,
) -> Option<&T> {
    let all_languages = &settings_content.project.all_languages;

    active_language()
        .and_then(|current_language_name| {
            all_languages
                .languages
                .0
                .get(current_language_name.as_ref())
        })
        .and_then(get_language_setting_field)
        .or_else(|| get_language_setting_field(&all_languages.defaults))
}

fn language_settings_field_mut<T>(
    settings_content: &mut SettingsContent,
    value: Option<T>,
    write: fn(&mut LanguageSettingsContent, Option<T>),
) {
    let all_languages = &mut settings_content.project.all_languages;
    let language_content = if let Some(current_language) = active_language() {
        all_languages
            .languages
            .0
            .entry(current_language.to_string())
            .or_default()
    } else {
        &mut all_languages.defaults
    };
    write(language_content, value);
}

fn language_settings_data() -> Box<[SettingsPageItem]> {
    fn indentation_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.indentation"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.tab_size",
                description: "settings_page.desc.indentation.tab_size",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).tab_size"), // TODO(cameron): not JQ syntax because not URL-safe
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.tab_size.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.tab_size = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.hard_tabs",
                description: "settings_page.desc.indentation.hard_tabs",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).hard_tabs"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.hard_tabs.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.hard_tabs = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_indent",
                description: "settings_page.desc.indentation.auto_indent",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).auto_indent"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.auto_indent.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.auto_indent = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_indent_on_paste",
                description: "settings_page.desc.indentation.auto_indent_on_paste",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).auto_indent_on_paste"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.auto_indent_on_paste.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.auto_indent_on_paste = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn wrapping_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.wrapping"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.soft_wrap",
                description: "settings_page.desc.wrapping.soft_wrap",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).soft_wrap"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.soft_wrap.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.soft_wrap = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_wrap_guides",
                description: "settings_page.desc.wrapping.show_wrap_guides",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).show_wrap_guides"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.show_wrap_guides.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.show_wrap_guides = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.preferred_line_length",
                description: "settings_page.desc.wrapping.preferred_line_length",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).preferred_line_length"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.preferred_line_length.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.preferred_line_length = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.wrap_guides",
                description: "settings_page.desc.wrapping.wrap_guides",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).wrap_guides"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.wrap_guides.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.wrap_guides = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.allow_rewrap",
                description: "settings_page.desc.wrapping.allow_rewrap",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).allow_rewrap"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.allow_rewrap.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.allow_rewrap = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn indent_guides_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.indent_guides"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.indent_guides.enabled",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).indent_guides.enabled"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language
                                .indent_guides
                                .as_ref()
                                .and_then(|indent_guides| indent_guides.enabled.as_ref())
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.indent_guides.get_or_insert_default().enabled = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.line_width",
                description: "settings_page.desc.indent_guides.line_width",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).indent_guides.line_width"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language
                                .indent_guides
                                .as_ref()
                                .and_then(|indent_guides| indent_guides.line_width.as_ref())
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.indent_guides.get_or_insert_default().line_width = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.active_line_width",
                description: "settings_page.desc.indent_guides.active_line_width",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).indent_guides.active_line_width"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language
                                .indent_guides
                                .as_ref()
                                .and_then(|indent_guides| indent_guides.active_line_width.as_ref())
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .indent_guides
                                .get_or_insert_default()
                                .active_line_width = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.coloring",
                description: "settings_page.desc.indent_guides.coloring",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).indent_guides.coloring"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language
                                .indent_guides
                                .as_ref()
                                .and_then(|indent_guides| indent_guides.coloring.as_ref())
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.indent_guides.get_or_insert_default().coloring = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.background_coloring",
                description: "settings_page.desc.indent_guides.background_coloring",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).indent_guides.background_coloring"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.indent_guides.as_ref().and_then(|indent_guides| {
                                indent_guides.background_coloring.as_ref()
                            })
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .indent_guides
                                .get_or_insert_default()
                                .background_coloring = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn formatting_section() -> [SettingsPageItem; 7] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.formatting"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.format_on_save",
                description: "settings_page.desc.formatting.format_on_save",
                field: Box::new(
                    // TODO(settings_ui): this setting should just be a bool
                    SettingField {
                        json_path: Some("languages.$(language).format_on_save"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.format_on_save.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.format_on_save = value;
                                },
                            )
                        },
                    },
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.remove_trailing_whitespace_on_save",
                description: "settings_page.desc.formatting.remove_trailing_whitespace_on_save",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).remove_trailing_whitespace_on_save"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.remove_trailing_whitespace_on_save.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.remove_trailing_whitespace_on_save = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.ensure_final_newline_on_save",
                description: "settings_page.desc.formatting.ensure_final_newline_on_save",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).ensure_final_newline_on_save"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.ensure_final_newline_on_save.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.ensure_final_newline_on_save = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.formatter",
                description: "settings_page.desc.formatting.formatter",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).formatter"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.formatter.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.formatter = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_on_type_format",
                description: "settings_page.desc.formatting.use_on_type_format",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).use_on_type_format"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.use_on_type_format.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.use_on_type_format = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.code_actions_on_format",
                description: "settings_page.desc.formatting.code_actions_on_format",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).code_actions_on_format"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.code_actions_on_format.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.code_actions_on_format = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn autoclose_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.autoclose"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_autoclose",
                description: "settings_page.desc.autoclose.use_autoclose",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).use_autoclose"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.use_autoclose.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.use_autoclose = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.use_auto_surround",
                description: "settings_page.desc.autoclose.use_auto_surround",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).use_auto_surround"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.use_auto_surround.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.use_auto_surround = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.always_treat_brackets_as_autoclosed",
                description: "settings_page.desc.autoclose.always_treat_brackets_as_autoclosed",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).always_treat_brackets_as_autoclosed"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.always_treat_brackets_as_autoclosed.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.always_treat_brackets_as_autoclosed = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.jsx_tag_auto_close",
                description: "settings_page.desc.autoclose.jsx_tag_auto_close",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).jsx_tag_auto_close"),
                    // TODO(settings_ui): this setting should just be a bool
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.jsx_tag_auto_close.as_ref()?.enabled.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.jsx_tag_auto_close.get_or_insert_default().enabled = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn whitespace_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.whitespace"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_whitespaces",
                description: "settings_page.desc.whitespace.show_whitespaces",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).show_whitespaces"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.show_whitespaces.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.show_whitespaces = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.space_whitespace_indicator",
                description: "settings_page.desc.whitespace.space_whitespace_indicator",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).whitespace_map.space"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.whitespace_map.as_ref()?.space.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.whitespace_map.get_or_insert_default().space = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.tab_whitespace_indicator",
                description: "settings_page.desc.whitespace.tab_whitespace_indicator",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).whitespace_map.tab"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.whitespace_map.as_ref()?.tab.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.whitespace_map.get_or_insert_default().tab = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn completions_section() -> [SettingsPageItem; 7] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.completions"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_completions_on_input",
                description: "settings_page.desc.completions.show_completions_on_input",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).show_completions_on_input"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.show_completions_on_input.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.show_completions_on_input = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_completion_documentation",
                description: "settings_page.desc.completions.show_completion_documentation",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).show_completion_documentation"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.show_completion_documentation.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.show_completion_documentation = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.words",
                description: "settings_page.desc.completions.words",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).completions.words"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.completions.as_ref()?.words.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.completions.get_or_insert_default().words = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.words_min_length",
                description: "settings_page.desc.completions.words_min_length",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).completions.words_min_length"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.completions.as_ref()?.words_min_length.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .completions
                                .get_or_insert_default()
                                .words_min_length = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.completion_menu_scrollbar",
                description: "settings_page.desc.completions.completion_menu_scrollbar",
                field: Box::new(SettingField {
                    json_path: Some("editor.completion_menu_scrollbar"),
                    pick: |settings_content| {
                        settings_content.editor.completion_menu_scrollbar.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.completion_menu_scrollbar = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.completion_detail_alignment",
                description: "settings_page.desc.completions.completion_detail_alignment",
                field: Box::new(SettingField {
                    json_path: Some("editor.completion_detail_alignment"),
                    pick: |settings_content| {
                        settings_content.editor.completion_detail_alignment.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.completion_detail_alignment = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    fn inlay_hints_section() -> [SettingsPageItem; 10] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.inlay_hints"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.inlay_hints.enabled",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.enabled"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.enabled.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.inlay_hints.get_or_insert_default().enabled = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_value_hints",
                description: "settings_page.desc.inlay_hints.show_value_hints",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.show_value_hints"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.show_value_hints.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .inlay_hints
                                .get_or_insert_default()
                                .show_value_hints = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_type_hints",
                description: "settings_page.desc.inlay_hints.show_type_hints",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.show_type_hints"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.show_type_hints.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.inlay_hints.get_or_insert_default().show_type_hints = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_parameter_hints",
                description: "settings_page.desc.inlay_hints.show_parameter_hints",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.show_parameter_hints"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.show_parameter_hints.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .inlay_hints
                                .get_or_insert_default()
                                .show_parameter_hints = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_other_hints",
                description: "settings_page.desc.inlay_hints.show_other_hints",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.show_other_hints"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.show_other_hints.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .inlay_hints
                                .get_or_insert_default()
                                .show_other_hints = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.show_background",
                description: "settings_page.desc.inlay_hints.show_background",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.show_background"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.show_background.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.inlay_hints.get_or_insert_default().show_background = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.edit_debounce_ms",
                description: "settings_page.desc.inlay_hints.edit_debounce_ms",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.edit_debounce_ms"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.edit_debounce_ms.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .inlay_hints
                                .get_or_insert_default()
                                .edit_debounce_ms = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.scroll_debounce_ms",
                description: "settings_page.desc.inlay_hints.scroll_debounce_ms",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).inlay_hints.scroll_debounce_ms"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.inlay_hints.as_ref()?.scroll_debounce_ms.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .inlay_hints
                                .get_or_insert_default()
                                .scroll_debounce_ms = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.toggle_on_modifiers_press",
                description: "settings_page.desc.inlay_hints.toggle_on_modifiers_press",
                field: Box::new(
                    SettingField {
                        json_path: Some(
                            "languages.$(language).inlay_hints.toggle_on_modifiers_press",
                        ),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language
                                    .inlay_hints
                                    .as_ref()?
                                    .toggle_on_modifiers_press
                                    .as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language
                                        .inlay_hints
                                        .get_or_insert_default()
                                        .toggle_on_modifiers_press = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn tasks_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.tasks"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.tasks.enabled",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).tasks.enabled"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.tasks.as_ref()?.enabled.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.tasks.get_or_insert_default().enabled = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.variables",
                description: "settings_page.desc.tasks.variables",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).tasks.variables"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.tasks.as_ref()?.variables.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.tasks.get_or_insert_default().variables = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.prefer_lsp",
                description: "settings_page.desc.tasks.prefer_lsp",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).tasks.prefer_lsp"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.tasks.as_ref()?.prefer_lsp.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.tasks.get_or_insert_default().prefer_lsp = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn miscellaneous_section() -> [SettingsPageItem; 6] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.miscellaneous"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.word_diff_enabled",
                description: "settings_page.desc.miscellaneous.word_diff_enabled",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).word_diff_enabled"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.word_diff_enabled.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.word_diff_enabled = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.middle_click_paste",
                description: "settings_page.desc.miscellaneous.middle_click_paste",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).editor.middle_click_paste"),
                    pick: |settings_content| settings_content.editor.middle_click_paste.as_ref(),
                    write: |settings_content, value| {
                        settings_content.editor.middle_click_paste = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.extend_comment_on_newline",
                description: "settings_page.desc.miscellaneous.extend_comment_on_newline",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).extend_comment_on_newline"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.extend_comment_on_newline.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.extend_comment_on_newline = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.colorize_brackets",
                description: "settings_page.desc.miscellaneous.colorize_brackets",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).colorize_brackets"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.colorize_brackets.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.colorize_brackets = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.vim_emacs_modeline_support",
                description: "settings_page.desc.miscellaneous.vim_emacs_modeline_support",
                field: Box::new(SettingField {
                    json_path: Some("modeline_lines"),
                    pick: |settings_content| settings_content.modeline_lines.as_ref(),
                    write: |settings_content, value| {
                        settings_content.modeline_lines = value;
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn global_only_miscellaneous_sub_section() -> [SettingsPageItem; 3] {
        [
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.image_viewer",
                description: "settings_page.desc.global_only_miscellaneous_sub.image_viewer",
                field: Box::new(SettingField {
                    json_path: Some("image_viewer.unit"),
                    pick: |settings_content| {
                        settings_content
                            .image_viewer
                            .as_ref()
                            .and_then(|image_viewer| image_viewer.unit.as_ref())
                    },
                    write: |settings_content, value| {
                        settings_content.image_viewer.get_or_insert_default().unit = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.auto_replace_emoji_shortcode",
                description: "settings_page.desc.global_only_miscellaneous_sub.auto_replace_emoji_shortcode",
                field: Box::new(SettingField {
                    json_path: Some("message_editor.auto_replace_emoji_shortcode"),
                    pick: |settings_content| {
                        settings_content
                            .message_editor
                            .as_ref()
                            .and_then(|message_editor| {
                                message_editor.auto_replace_emoji_shortcode.as_ref()
                            })
                    },
                    write: |settings_content, value| {
                        settings_content
                            .message_editor
                            .get_or_insert_default()
                            .auto_replace_emoji_shortcode = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.drop_size_target",
                description: "settings_page.desc.global_only_miscellaneous_sub.drop_size_target",
                field: Box::new(SettingField {
                    json_path: Some("drop_target_size"),
                    pick: |settings_content| settings_content.workspace.drop_target_size.as_ref(),
                    write: |settings_content, value| {
                        settings_content.workspace.drop_target_size = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
        ]
    }

    let is_global = active_language().is_none();

    let lsp_document_colors_item = [SettingsPageItem::SettingItem(SettingItem {
        title: "settings_page.item.lsp_document_colors",
        description: "settings_page.desc.global_only_miscellaneous_sub.lsp_document_colors",
        field: Box::new(SettingField {
            json_path: Some("lsp_document_colors"),
            pick: |settings_content| settings_content.editor.lsp_document_colors.as_ref(),
            write: |settings_content, value| {
                settings_content.editor.lsp_document_colors = value;
            },
        }),
        metadata: None,
        files: USER,
    })];

    if is_global {
        concat_sections!(
            indentation_section(),
            wrapping_section(),
            indent_guides_section(),
            formatting_section(),
            autoclose_section(),
            whitespace_section(),
            completions_section(),
            inlay_hints_section(),
            lsp_document_colors_item,
            tasks_section(),
            miscellaneous_section(),
            global_only_miscellaneous_sub_section(),
        )
    } else {
        concat_sections!(
            indentation_section(),
            wrapping_section(),
            indent_guides_section(),
            formatting_section(),
            autoclose_section(),
            whitespace_section(),
            completions_section(),
            inlay_hints_section(),
            tasks_section(),
            miscellaneous_section(),
        )
    }
}

/// LanguageSettings items that should be included in the "Languages & Tools" page
/// not the "Editor" page
fn non_editor_language_settings_data() -> Box<[SettingsPageItem]> {
    fn lsp_section() -> [SettingsPageItem; 8] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.lsp"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enable_language_server",
                description: "settings_page.desc.lsp.enable_language_server",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).enable_language_server"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.enable_language_server.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.enable_language_server = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.language_servers",
                description: "settings_page.desc.lsp.language_servers",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).language_servers"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.language_servers.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.language_servers = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.linked_edits",
                description: "settings_page.desc.lsp.linked_edits",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).linked_edits"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.linked_edits.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.linked_edits = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.go_to_definition_fallback",
                description: "settings_page.desc.lsp.go_to_definition_fallback",
                field: Box::new(SettingField {
                    json_path: Some("go_to_definition_fallback"),
                    pick: |settings_content| {
                        settings_content.editor.go_to_definition_fallback.as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content.editor.go_to_definition_fallback = value;
                    },
                }),
                metadata: None,
                files: USER,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.semantic_tokens",
                description: "settings_page.language.semantic_tokens.description",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).semantic_tokens"),
                    pick: |settings_content| {
                        settings_content
                            .project
                            .all_languages
                            .defaults
                            .semantic_tokens
                            .as_ref()
                    },
                    write: |settings_content, value| {
                        settings_content
                            .project
                            .all_languages
                            .defaults
                            .semantic_tokens = value;
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.lsp_folding_ranges",
                description: "settings_page.desc.lsp.lsp_folding_ranges",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).document_folding_ranges"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.document_folding_ranges.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.document_folding_ranges = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.lsp_document_symbols",
                description: "settings_page.desc.lsp.lsp_document_symbols",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).document_symbols"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.document_symbols.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.document_symbols = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    fn lsp_completions_section() -> [SettingsPageItem; 4] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.lsp_completions"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.enabled",
                description: "settings_page.desc.lsp_completions.enabled",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).completions.lsp"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.completions.as_ref()?.lsp.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.completions.get_or_insert_default().lsp = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.fetch_timeout_milliseconds",
                description: "settings_page.desc.lsp_completions.fetch_timeout_milliseconds",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).completions.lsp_fetch_timeout_ms"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.completions.as_ref()?.lsp_fetch_timeout_ms.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language
                                .completions
                                .get_or_insert_default()
                                .lsp_fetch_timeout_ms = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.insert_mode",
                description: "settings_page.desc.lsp_completions.insert_mode",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).completions.lsp_insert_mode"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.completions.as_ref()?.lsp_insert_mode.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.completions.get_or_insert_default().lsp_insert_mode = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }


    fn prettier_section() -> [SettingsPageItem; 5] {
        [
            SettingsPageItem::SectionHeader("settings_page.section.prettier"),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.allowed",
                description: "settings_page.desc.prettier.allowed",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).prettier.allowed"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.prettier.as_ref()?.allowed.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.prettier.get_or_insert_default().allowed = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.parser",
                description: "settings_page.desc.prettier.parser",
                field: Box::new(SettingField {
                    json_path: Some("languages.$(language).prettier.parser"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.prettier.as_ref()?.parser.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.prettier.get_or_insert_default().parser = value;
                        })
                    },
                }),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.plugins",
                description: "settings_page.desc.prettier.plugins",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).prettier.plugins"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.prettier.as_ref()?.plugins.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.prettier.get_or_insert_default().plugins = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
            SettingsPageItem::SettingItem(SettingItem {
                title: "settings_page.item.options",
                description: "settings_page.desc.prettier.options",
                field: Box::new(
                    SettingField {
                        json_path: Some("languages.$(language).prettier.options"),
                        pick: |settings_content| {
                            language_settings_field(settings_content, |language| {
                                language.prettier.as_ref()?.options.as_ref()
                            })
                        },
                        write: |settings_content, value| {
                            language_settings_field_mut(
                                settings_content,
                                value,
                                |language, value| {
                                    language.prettier.get_or_insert_default().options = value;
                                },
                            )
                        },
                    }
                    .unimplemented(),
                ),
                metadata: None,
                files: USER | PROJECT,
            }),
        ]
    }

    concat_sections!(
        lsp_section(),
        lsp_completions_section(),
        prettier_section(),
    )
}

fn edit_prediction_language_settings_section() -> [SettingsPageItem; 4] {
    [
        SettingsPageItem::SectionHeader("settings_page.section.edit_predictions"),
        SettingsPageItem::SubPageLink(SubPageLink {
            title: "settings_page.item.configure_providers".into(),
            r#type: Default::default(),
            json_path: Some("edit_predictions.providers"),
            description: Some("settings_page.desc.edit_predictions.configure_providers".into()),
            in_json: false,
            files: USER,
            render: render_edit_prediction_setup_page
        }),
        SettingsPageItem::SettingItem(SettingItem {
            title: "settings_page.item.show_edit_predictions",
            description: "settings_page.desc.prettier.show_edit_predictions",
            field: Box::new(SettingField {
                json_path: Some("languages.$(language).show_edit_predictions"),
                pick: |settings_content| {
                    language_settings_field(settings_content, |language| {
                        language.show_edit_predictions.as_ref()
                    })
                },
                write: |settings_content, value| {
                    language_settings_field_mut(settings_content, value, |language, value| {
                        language.show_edit_predictions = value;
                    })
                },
            }),
            metadata: None,
            files: USER | PROJECT,
        }),
        SettingsPageItem::SettingItem(SettingItem {
            title: "settings_page.item.disable_in_language_scopes",
            description: "settings_page.desc.prettier.disable_in_language_scopes",
            field: Box::new(
                SettingField {
                    json_path: Some("languages.$(language).edit_predictions_disabled_in"),
                    pick: |settings_content| {
                        language_settings_field(settings_content, |language| {
                            language.edit_predictions_disabled_in.as_ref()
                        })
                    },
                    write: |settings_content, value| {
                        language_settings_field_mut(settings_content, value, |language, value| {
                            language.edit_predictions_disabled_in = value;
                        })
                    },
                }
                .unimplemented(),
            ),
            metadata: None,
            files: USER | PROJECT,
        }),
    ]
}

fn show_scrollbar_or_editor(
    settings_content: &SettingsContent,
    show: fn(&SettingsContent) -> Option<&settings::ShowScrollbar>,
) -> Option<&settings::ShowScrollbar> {
    show(settings_content).or(settings_content
        .editor
        .scrollbar
        .as_ref()
        .and_then(|scrollbar| scrollbar.show.as_ref()))
}

fn dynamic_variants<T>() -> &'static [T::Discriminant]
where
    T: strum::IntoDiscriminant,
    T::Discriminant: strum::VariantArray,
{
    <<T as strum::IntoDiscriminant>::Discriminant as strum::VariantArray>::VARIANTS
}

/// Updates the `vim_mode` setting, disabling `helix_mode` if present and
/// `vim_mode` is being enabled.
fn write_vim_mode(settings: &mut SettingsContent, value: Option<bool>) {
    if value == Some(true) && settings.helix_mode == Some(true) {
        settings.helix_mode = Some(false);
    }
    settings.vim_mode = value;
}

/// Updates the `helix_mode` setting, disabling `vim_mode` if present and
/// `helix_mode` is being enabled.
fn write_helix_mode(settings: &mut SettingsContent, value: Option<bool>) {
    if value == Some(true) && settings.vim_mode == Some(true) {
        settings.vim_mode = Some(false);
    }
    settings.helix_mode = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_vim_helix_mode() {
        // Enabling vim mode while `vim_mode` and `helix_mode` are not yet set
        // should only update the `vim_mode` setting.
        let mut settings = SettingsContent::default();
        write_vim_mode(&mut settings, Some(true));
        assert_eq!(settings.vim_mode, Some(true));
        assert_eq!(settings.helix_mode, None);

        // Enabling helix mode while `vim_mode` and `helix_mode` are not yet set
        // should only update the `helix_mode` setting.
        let mut settings = SettingsContent::default();
        write_helix_mode(&mut settings, Some(true));
        assert_eq!(settings.helix_mode, Some(true));
        assert_eq!(settings.vim_mode, None);

        // Disabling helix mode should only touch `helix_mode` setting when
        // `vim_mode` is not set.
        write_helix_mode(&mut settings, Some(false));
        assert_eq!(settings.helix_mode, Some(false));
        assert_eq!(settings.vim_mode, None);

        // Enabling vim mode should update `vim_mode` but leave `helix_mode`
        // untouched.
        write_vim_mode(&mut settings, Some(true));
        assert_eq!(settings.vim_mode, Some(true));
        assert_eq!(settings.helix_mode, Some(false));

        // Enabling helix mode should update `helix_mode` and disable
        // `vim_mode`.
        write_helix_mode(&mut settings, Some(true));
        assert_eq!(settings.helix_mode, Some(true));
        assert_eq!(settings.vim_mode, Some(false));

        // Enabling vim mode should update `vim_mode` and disable
        // `helix_mode`.
        write_vim_mode(&mut settings, Some(true));
        assert_eq!(settings.vim_mode, Some(true));
        assert_eq!(settings.helix_mode, Some(false));
    }
}
