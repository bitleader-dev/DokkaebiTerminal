// 워크스페이스 그룹 - 독립된 작업 그룹 상태를 관리하는 모듈

use collections::HashMap;
use gpui::{Entity, EntityId, WeakEntity};
use uuid::Uuid;

use crate::{Pane, PaneGroup};

/// 하나의 워크스페이스 그룹 상태.
/// 각 그룹은 독립된 센터 PaneGroup, 패인 목록, 활성 패인을 유지한다.
#[derive(Clone)]
pub struct WorkspaceGroupState {
    /// 그룹 안정 식별자. 이름 변경·인덱스 이동에도 동일하게 유지된다.
    /// 외부 저장소(메모장 패널 등) 가 그룹을 가리킬 때 사용.
    pub uuid: Uuid,
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
    /// 비활성 상태에서 알림(bell 등) 발생 여부
    pub has_notification: bool,
    /// 아이콘 색상 팔레트 인덱스. None 이면 기본 시맨틱(Default/Muted) 사용
    pub color: Option<u8>,
}

impl WorkspaceGroupState {
    /// 현재 Workspace 상태에서 그룹 스냅샷 생성. uuid 는 기존 그룹의 식별자를 그대로 전달한다.
    pub fn capture(
        uuid: Uuid,
        name: String,
        center: &PaneGroup,
        panes: &[Entity<Pane>],
        active_pane: &Entity<Pane>,
        last_active_center_pane: &Option<WeakEntity<Pane>>,
        panes_by_item: &HashMap<EntityId, WeakEntity<Pane>>,
        has_notification: bool,
        color: Option<u8>,
    ) -> Self {
        Self {
            uuid,
            name,
            center: center.clone(),
            panes: panes.to_vec(),
            active_pane: active_pane.clone(),
            last_active_center_pane: last_active_center_pane.clone(),
            panes_by_item: panes_by_item.clone(),
            has_notification,
            color,
        }
    }
}
