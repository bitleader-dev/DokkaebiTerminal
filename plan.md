# 워크스페이스 그룹 기능 추가 계획

## 목표
- 센터 화면을 독립된 워크스페이스 그룹으로 전환할 수 있는 기능 추가
- 각 그룹은 독립된 PaneGroup(탭/분할 패널) 유지
- 왼쪽 패널에서 그룹 관리 (추가/삭제/전환)

## 범위
- `crates/workspace/` 내 새 모듈 2개 추가
- `Workspace` 구조체에 그룹 관리 필드/메서드 추가
- 상태바에 토글 버튼 추가
- `crates/zed/src/zed.rs`에 패널 등록

## 작업 단계

### [x] 1. WorkspaceGroupState 데이터 구조 (`workspace_group.rs`)
- `WorkspaceGroupState` 구조체: name, center, panes, active_pane, last_active_center_pane, panes_by_item
- 현재 Workspace 상태에서 저장/복원 헬퍼

### [x] 2. Workspace 구조체 수정 (`workspace.rs`)
- 필드 추가: `workspace_groups: Vec<WorkspaceGroupState>`, `active_group_index: usize`
- `Workspace::new()`에서 기본 그룹 1개 초기화
- 메서드: `switch_workspace_group()`, `add_workspace_group()`, `remove_workspace_group()`
- 접근자: `workspace_groups()`, `active_group_index()`, `workspace_group_count()`

### [x] 3. WorkspaceGroupPanel UI (`workspace_group_panel.rs`)
- `Panel` trait 구현 (left dock, activation_priority=0)
- 상단 추가 버튼 ("+" 아이콘)
- 그룹 목록: 이름 + 활성 표시 + X 삭제 버튼 (1개일 때 숨김)
- 클릭 시 `switch_workspace_group()` 호출
- 새 그룹 추가 시 터미널 1개 자동 생성

### [x] 4. 패널 등록 및 통합
- `workspace.rs` lib에 모듈/export 추가
- `zed.rs` `initialize_panels()`에 WorkspaceGroupPanel 등록
- 상태바 좌측 독 패널 아이콘(ListTree)으로 토글

### [x] 5. 빌드 검증
- `cargo check -p workspace` 성공
- `cargo check -p zed` 성공

## 핵심 설계

### 그룹 전환 방식 (swap)
```
switch_workspace_group(new_index):
  1. 현재 center/panes/active_pane/panes_by_item → workspace_groups[현재] 저장
  2. workspace_groups[new_index] → center/panes/active_pane/panes_by_item 복원
  3. active_group_index = new_index
  4. 포커스 이동 + cx.notify()
```
- 기존 코드(렌더링, 이벤트 등)는 항상 `self.center`/`self.panes` 참조 → 변경 불필요

### 새 그룹 생성
1. 새 Pane 생성 + PaneGroup 생성
2. TerminalView::deploy로 터미널 추가
3. 새 그룹으로 전환

## 승인 필요 사항
- [x] Workspace 구조체 필드 추가 — 승인됨
- [x] 새 모듈 2개 추가 (workspace_group.rs, workspace_group_panel.rs) — 승인됨
- [x] zed.rs 패널 등록 수정 — 승인됨
