use gpui::Pixels;
use settings::{RegisterSetting, Settings};
use ui::px;
use crate::dock::DockPosition;

#[derive(Debug, Clone, Copy, PartialEq, RegisterSetting)]
pub struct WorkspaceGroupPanelSettings {
    pub button: bool,
    pub dock: DockPosition,
    pub default_width: Pixels,
    pub starts_open: bool,
}

impl Settings for WorkspaceGroupPanelSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        let workspace_group_panel = content.workspace_group_panel.clone().unwrap();
        Self {
            button: workspace_group_panel.button.unwrap(),
            dock: workspace_group_panel.dock.unwrap().into(),
            default_width: px(workspace_group_panel.default_width.unwrap()),
            starts_open: workspace_group_panel.starts_open.unwrap(),
        }
    }
}
