# 워크스페이스 그룹 패널 항목 간 4픽셀 간격 추가 (2026-04-16)

## 목표
워크스페이스 그룹 목록 내 항목(`워크스페이스 1`, `워크스페이스 2` …)이 서로 붙어 보이지 않도록 항목 간 세로 간격을 4픽셀 부여한다.

## 범위
- `crates/workspace/src/workspace_group_panel.rs`
  - 그룹 목록 컨테이너 `v_flex().id("workspace-group-list")`에 `.gap(px(4.))` 추가
- 각 항목(`h_flex().id("workspace-group-item", index)`)의 내부 패딩·스타일은 유지

## 승인 필요 여부
- 구조/공개 API/DB 스키마 변경 없음 → 승인 대상 아님
- UI 스타일 소폭 조정 (사용자가 명시적으로 요청)

## 작업 단계
- [x] 1. 그룹 목록 `v_flex()`에 `.gap(px(4.))` 추가
- [x] 2. `cargo check -p workspace` 검증 (4.90s, exit 0, 신규 경고 0건)
- [x] 3. 문서 갱신(`notes.md`)

## 검증 방법
- `cargo check -p workspace` 경고·에러 없음

## 진행 표시
- [ ] 예정 / [/] 진행 중 / [x] 검증 완료
