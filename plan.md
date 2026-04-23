# 서브에이전트 탭 잘못된 워크스페이스 그룹 배치 수정 — 계획

> **현재 단계**: 검증 완료 (2026-04-23, 사용자 확인 "잘됨").

## 문제
워크스페이스 그룹 1(WS1)에서 멀티에이전트 실행 후, WS2에서 멀티에이전트를 실행하면 첫 서브에이전트 탭만 WS2에 생기고 나머지가 WS1의 기존 서브에이전트 pane으로 들러붙는다.

## 근본 원인
- `mark_bell_for_notification` (open_listener.rs:1042~)은 매칭 터미널의 `group_idx`를 알지만 `NotifyTarget`에 담지 않는다.
- `open_subagent_view` (claude_subagent_view/src/view.rs:324~)와 `scan_panes`는 `workspace.active_pane()`/`workspace.panes()`만 사용 → **현재 활성 그룹의 panes 만 본다**.
- 결과: IPC 처리 시점에 활성 그룹이 발신 터미널이 속한 그룹과 다르면, 새 탭이 활성 그룹의 서브에이전트 pane(또는 split 결과)에 잘못 부착된다.

## 수정 방향 (A안 — 사용자 시야 비침습)
타겟 그룹 인덱스를 IPC 경로 끝까지 전파하고, `open_subagent_view`가 해당 그룹(활성/비활성 모두) 의 pane 상태를 직접 읽고 쓰도록 한다.

## 작업 단계
- [x] 1. `crates/workspace/src/workspace.rs` — 타겟 그룹 기준 헬퍼 2개 추가
  - `panes_in_group(group_idx) -> Option<&[Entity<Pane>]>` (활성이면 `&self.panes`, 비활성이면 `workspace_groups[idx].panes`)
  - `split_pane_in_group(group_idx, direction, window, cx) -> Option<Entity<Pane>>` (활성이면 기존 `split_pane` 위임; 비활성이면 `create_inactive_pane` + 그룹 center.split + 그룹 panes.push)
- [x] 2. `crates/zed/src/zed/open_listener.rs`
  - `NotifyTarget`에 `group_idx: usize` 추가
  - `mark_bell_for_notification`에서 첫 매칭 시 `group_idx`도 함께 저장
  - `handle_subagent_request`의 `open_subagent_view` 호출에 `target.group_idx` 전달
- [x] 3. `crates/claude_subagent_view/src/view.rs`
  - `open_subagent_view` 시그니처에 `target_group_idx: usize` 추가
  - `scan_panes`가 `panes_in_group(target_group_idx)` 사용
  - split fallback이 `split_pane_in_group(target_group_idx, ...)` 사용

## 검증
- [x] `cargo check -p Dokkaebi` 통과 (4.99s, 신규 경고/에러 0건)
- [x] `cargo check -p workspace` 통과
- [x] `cargo check -p claude_subagent_view` 통과
- [x] 런타임 검증 완료 (2026-04-23, 사용자 확인): WS1·WS2 멀티에이전트 순차 실행 시 각 그룹의 발신 터미널 소속 그룹에 탭이 정확히 부착됨

## 승인 필요
없음 — 사용자가 A안 진행 승인 (2026-04-23).

## 영향 범위 외 (변경 없음)
- 활성 그룹에서의 기존 동작: 그룹 인덱스가 active와 같으면 기존 `split_pane`/`workspace.panes()` 경로 그대로 위임 → 회귀 없음.
- 설정 기본값·JSON 스키마: 변경 없음.
- IPC 와이어 포맷: `NotifyTarget`은 본체 내부 구조라 wire 무관.
- 비활성 그룹 `panes_by_item`: 서브에이전트 뷰는 bell 알림 발신처가 아니므로 갱신 생략(영향 없음).

## 릴리즈 노트
v0.4.0 신규 기능 개발 중 발견·수정된 버그 → `assets/release_notes.md` 기재 제외 (CLAUDE.md 규칙).
