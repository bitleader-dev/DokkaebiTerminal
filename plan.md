# 메모장 패널 터미널 메뉴 색상 + 탭 색상 자동 부여 plan v1 (종료)

> **작성일**: 2026-04-30
> **종료일**: 2026-04-30
> **상태**: ✅ 종료 — Phase 1~3 모두 완료, 사용자 수동 검증 대기
> **버전 기준**: `crates/zed/Cargo.toml` **v0.4.4** (bump 완료)
> **이전 plan**: 터미널 탭 색상 커스터마이징 plan v1 종료 (v0.4.3 출시)

## 진행 결과

| Phase | 결과 | 검증 |
|---|---|---|
| 1. 메모장 터미널 메뉴 좌측 컬러 바 | ✅ 완료 | `cargo check -p notepad_panel` |
| 2. 새 터미널 탭 색상 자동 부여 (`pick_auto_tab_color`) | ✅ 완료 | `cargo check -p terminal_view` |
| 3. 검증 + 문서 + v0.4.4 bump | ✅ 완료 | `cargo check -p Dokkaebi` 통과 (신규 warning 0) |

본 plan 종료. 후속 plan 후보로는 "블록 단위 navigation" 또는 "탭 색상 컨텍스트 자동 분류" 가 다음 진입점.

## 목표

v0.4.3 의 탭 색상 인프라를 두 위치로 확장한다.

1. **메모장 패널 → 텍스트 선택 → 우클릭 → 터미널 목록 메뉴**: 각 터미널 항목 라벨 앞에 해당 탭의 색상 dot 표시 (탭 색상 미지정 항목은 dot 없음). 사용자는 메뉴에서 색상으로 터미널을 즉시 식별 가능.
2. **새 터미널 탭 생성 시 색상 자동 부여**: 같은 워크스페이스의 다른 터미널 탭들이 사용 중이지 않은 색상 중 하나를 자동 부여. 모든 색상이 이미 사용 중이면 카운트가 가장 적은 색상. 영구 저장에서 복원된 사용자 명시 색상은 그대로 유지.

## 비목표

- 자동 부여 색상에 대한 사용자 설정 토글 (현재는 항상 ON, 후속 plan 후보)
- 색상이 task 터미널에도 적용되도록 확장 (현재는 일반 셸 한정)
- 메모장 패널 외 다른 위치(예: agent panel 의 터미널 목록)의 색상 표시

## 라이선스 게이트

- v0.4.3 의 색상 매핑 그대로 재사용 — 외부 코드 미참조.
- 외부 의존성 추가 0건 예상.

## 작업 단계

### Phase 1 — 메모장 패널 터미널 메뉴 좌측 컬러 바 표시 [/]
- [ ] `crates/notepad_panel/src/notepad_panel.rs::deploy_terminal_send_menu` 의 터미널 entry 생성부 수정.
- [ ] 각 TerminalView 에서 `custom_color()` 게터 호출 (v0.4.3 에서 `pub fn` 으로 추가됨).
- [ ] `terminal_view::TerminalTabColor` import 후 `color.hsla()` 로 컬러 바 색상 산출.
- [ ] 터미널 entry 를 `ContextMenu::custom_entry` 로 그려 `h_flex { 좌측 3px × h_4 컬러 바 또는 동일 너비 투명 영역, 라벨 }` 형식. 탭의 좌측 컬러 바와 폭/모서리 일관성 유지(`rounded_sm`). custom_color None 인 항목은 동일 너비 투명 div 로 정렬을 맞춘다.
- [ ] 단축키/dispatch 동작은 기존과 동일 — handler 만 그대로 유지.

### Phase 2 — 새 터미널 탭 생성 시 자동 색상 부여 [/]
- [ ] `crates/terminal_view/src/terminal_view.rs` 에 신규 자유 함수 `pick_auto_tab_color(workspace: &Workspace, cx: &App) -> TerminalTabColor`.
  - 워크스페이스의 모든 `TerminalView` 순회 (TerminalPanel + 중앙 panes), 각 `custom_color()` 카운트.
  - `TerminalTabColor::ALL` 순서로 첫 미사용 색상 반환.
  - 모두 사용 중이면 카운트 가장 작은 색상 반환 (동률 시 ALL 순서상 앞).
- [ ] `TerminalView::new` 끝에서 `custom_color` 가 None 이고 task 가 None 이고 workspace 가 살아 있으면 `pick_auto_tab_color` 결과로 set. 단 영구화 트리거(`needs_serialize`) 는 그대로.
  - **deserialize 경로 호환**: `deserialize` 의 마지막 `if custom_color.is_some() { view.custom_color = custom_color; }` 분기는 그대로 두면 사용자 명시 색상이 자동 부여 색상을 덮어쓴다. 추가 처리: deserialize 시 DB 의 custom_color 가 Some 이면 자동 부여 결과를 무시하기 위해, `new` 의 자동 부여 결과를 `view.custom_color = ...` 로 설정 후 deserialize 분기가 그것을 다시 덮어쓰는 순서가 자연스럽다 → 별도 변경 불필요.

### Phase 3 — 검증 + 문서
- [ ] `cargo check -p notepad_panel -p terminal_view -p Dokkaebi` 통과 (신규 warning 0).
- [ ] 사용자 환경 수동 검증:
  1. 새 터미널 1개 생성 → 자동 색상 부여 확인 (좌측 컬러 바)
  2. 새 터미널 추가 (총 2개) → 첫 번째와 다른 색상 부여
  3. 8개 추가 → 모든 색상 사용 → 9번째는 카운트 가장 작은 색상
  4. 메모장에서 텍스트 선택 → 우클릭 → 터미널 목록에 각 탭의 dot 표시
  5. 사용자가 명시 변경한 색상은 재시작 후 유지 (자동 부여가 덮어쓰지 않음)
- [ ] `crates/zed/Cargo.toml` v0.4.3 → v0.4.4 bump
- [ ] `assets/release_notes.md` v0.4.4 신규 섹션 — 새 기능 2 (메뉴 dot, 자동 색상)
- [ ] `notes.md` Phase 별 변경 기록

## 승인 필요 사항

| 항목 | 사유 |
|---|---|
| **Phase 2** TerminalView::new 끝에서 색상 자동 set | 새 터미널 동작 변경. 사용자 환경 영향 |
| **버전 bump** | v0.4.4 |

## 리스크 및 대응

| 리스크 | 대응 |
|---|---|
| `pick_auto_tab_color` 가 모든 TerminalView 순회 — 다수 탭일 때 비용 | O(N), N=탭 수. 일반 N<20 가정 시 무시 가능 |
| 영구화된 색상이 자동 부여로 덮어쓰기 | new → deserialize 순서로 덮어쓰는 분기가 우선이라 보존됨 |
| task 터미널에 자동 색상 부여 | task=Some 분기 skip 으로 차단 (기존 정책 일관) |
| 색상 부여 직후 영구화 — 새 탭 닫고 재오픈 시 색상 변경 가능 | 자동 부여도 영구화되므로 재오픈 시 동일 색상. 단 워크스페이스 외부에서 보면 색상이 무작위로 보일 수 있음 (수용) |

---

**다음 액션**: 본 plan 승인 시 Phase 1 착수.
