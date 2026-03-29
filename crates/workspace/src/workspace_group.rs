// 워크스페이스 그룹 - 독립된 작업 그룹 상태를 관리하는 모듈

use collections::HashMap;
use gpui::{Entity, EntityId, WeakEntity};

use crate::{Pane, PaneGroup};

/// 하나의 워크스페이스 그룹 상태.
/// 각 그룹은 독립된 센터 PaneGroup, 패인 목록, 활성 패인을 유지한다.
#[derive(Clone)]
pub struct WorkspaceGroupState {
    /// 그룹 이름 (UI 표시용)
    pub name: String,
    /// 센터 영역 PaneGroup (탭/분할 구조)
    pub center: PaneGroup,
    /// 이 그룹에 속한 모든 패인
    pub panes: Vec<Entity<Pane>>,
    /// 현재 활성 패인
    pub active_pane: Entity<Pane>,
    /// 마지막 활성 센터 패인 (약한 참조)
    pub last_active_center_pane: Option<WeakEntity<Pane>>,
    /// 아이템 ID → 패인 매핑
    pub panes_by_item: HashMap<EntityId, WeakEntity<Pane>>,
}

impl WorkspaceGroupState {
    /// 현재 Workspace 상태에서 그룹 스냅샷 생성
    pub fn capture(
        name: String,
        center: &PaneGroup,
        panes: &[Entity<Pane>],
        active_pane: &Entity<Pane>,
        last_active_center_pane: &Option<WeakEntity<Pane>>,
        panes_by_item: &HashMap<EntityId, WeakEntity<Pane>>,
    ) -> Self {
        Self {
            name,
            center: center.clone(),
            panes: panes.to_vec(),
            active_pane: active_pane.clone(),
            last_active_center_pane: last_active_center_pane.clone(),
            panes_by_item: panes_by_item.clone(),
        }
    }
}
