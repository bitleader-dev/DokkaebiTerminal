# 워크스페이스 그룹 영속화(Persistence) 계획

## 목표
- 워크스페이스 그룹 목록(이름, 순서, 활성 상태)을 DB에 저장하여 앱 재시작 시 복원
- 각 그룹의 탭/분할 구조(패인, 아이템)도 모두 저장하여 그대로 복원

## 범위
- `crates/workspace/src/persistence.rs` — DB 마이그레이션 + 저장/로드 로직 수정
- `crates/workspace/src/persistence/model.rs` — `SerializedWorkspaceGroup` 추가
- `crates/workspace/src/workspace.rs` — 직렬화/역직렬화 수정

## 작업 단계

### [x] 1. 모델 추가 (`persistence/model.rs`)
- `SerializedWorkspaceGroup` 구조체 추가 (name, center_group, active)
- `SerializedWorkspace`에 `workspace_groups: Vec<SerializedWorkspaceGroup>`, `active_group_index: usize` 필드 추가

### [x] 2. DB 마이그레이션 (`persistence.rs`)
- `workspace_groups` 테이블 생성: workspace_group_id, workspace_id, name, position, active
- `pane_groups` 테이블에 `workspace_group_id` 컬럼 추가
- `panes` 테이블에 `workspace_group_id` 컬럼 추가

### [x] 3. 저장 로직 수정 (`persistence.rs`)
- `save_workspace()`: workspace_groups 테이블 클리어 후 각 그룹 저장
- `save_pane_group()`, `save_pane()`: workspace_group_id 매개변수 추가
- 각 그룹의 패인 트리를 workspace_group_id와 함께 저장

### [x] 4. 로드 로직 수정 (`persistence.rs`)
- `get_workspace_groups()` 메서드 추가
- `get_pane_group()`, `get_center_pane_group()`: workspace_group_id 필터링 추가
- `workspace_for_roots_internal()`, `workspace_for_id()`: workspace_groups 로드

### [x] 5. 직렬화 수정 (`workspace.rs`)
- `serialize_workspace_internal()`: 모든 워크스페이스 그룹 직렬화
- 활성 그룹 동기화 후 각 그룹의 패인 트리 빌드

### [x] 6. 역직렬화 수정 (`workspace.rs`)
- `load_workspace()`: workspace_groups가 있으면 모든 그룹 복원
- 각 그룹의 패인/아이템 역직렬화하여 WorkspaceGroupState 생성
- active_group_index로 활성 그룹 설정

### [x] 7. 빌드 검증
- `cargo check -p workspace` 성공
- `cargo check -p zed` 성공

## 승인 필요 사항
- [x] DB 스키마 변경 (workspace_groups 테이블 추가, pane_groups/panes 컬럼 추가) — 승인됨
