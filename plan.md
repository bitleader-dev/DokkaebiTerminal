# 터미널 패널 탭 CWD 복원 실패 수정 — delete_unloaded_items 문제

## 근본 원인
`load_workspace`에서 `item_ids_by_kind`를 빌드할 때 워크스페이스 CENTER 패인의 아이템만 수집한다.
터미널 패널 아이템은 CENTER가 아니므로 포함되지 않는다.
workspace center cleanup과 terminal panel cleanup이 동일한 DB 테이블(`terminals`)을 공유하면서 서로의 아이템을 삭제하는 문제가 발생한다.

- `TerminalView`의 `SerializableItem::cleanup`이 workspace center에서 호출되면 패널 아이템이 alive_items에 포함되지 않아 삭제됨
- 터미널 패널 자체 cleanup(terminal_panel.rs:310-328)이 이미 존재하며 패널 아이템을 올바르게 처리함

## 해결 방안
1. `TerminalView`의 `SerializableItem::cleanup` 구현을 no-op으로 변경 — workspace center에서 터미널 패널 DB 항목을 삭제하지 않도록 함
2. 터미널 패널의 자체 cleanup(terminal_panel.rs:310-328)이 모든 터미널 cleanup을 담당
3. 진단 로그 제거

## 범위
- `crates/terminal_view/src/terminal_view.rs`: `SerializableItem::cleanup` no-op으로 변경, 진단 로그 제거
- `crates/terminal_view/src/persistence.rs`: `dump_all_terminals` 진단 메서드 제거
- `crates/workspace/src/persistence.rs`: `delete_unloaded_items` 진단 로그 제거

## 작업 단계

### [x] 1. TerminalView::cleanup을 no-op으로 변경
### [x] 2. 진단 로그 제거 (terminal_view.rs, persistence.rs, workspace/persistence.rs)
### [x] 3. 빌드 검증
### [x] 4. 문서 갱신
