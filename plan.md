# 업스트림 Zed v0.232.2 선별 백포트 (2026-04-16)

## 목표
Zed v0.232.2 릴리즈 노트에서 추출한 3개 고우선 PR을 Dokkaebi에 백포트하고, 그 후 C 카테고리(다수 PR) 상세 조사를 진행한다.

## 사전 조사 결과 (확정 사실)
- **#53563**: `crates/project_symbols/src/project_symbols.rs` L288 근처에 `ceil_char_boundary` 미사용, UTF-8 패닉 위험 있음
- **#53036**: `crates/settings_ui/src/components/input_field.rs`에 `first_render_initial_text`/`on_focus_out` 없음. 구조체 필드 `confirm`/`display_confirm_button`/`display_clear_button`/`clear_on_confirm` 전부 존재 → 패치 이식 가능
- **#51756**:
  - `crates/ui/src/components/label/spinner_label.rs:7` `SpinnerVariant::{Dots, DotsVariant, Sand}` 존재, `SpinnerLabel::with_variant` 존재 → 선행 의존 OK
  - `crates/agent_ui/src/conversation_view.rs:54` use 문에만 `SpinnerLabel` 포함, 본문 사용 0건 → 제거 안전
  - `crates/agent_ui/src/conversation_view/thread_view.rs:5106, 5118` 스피너 호출 2곳 상류와 일치
  - L7457(subagent diff 아이콘) `SpinnerLabel::new()`는 업스트림 패치 범위 외 → 그대로 둔다
  - L4275 `pub(crate) fn render_entries` — 업스트림은 `fn`으로 변경했지만 visibility 변경은 필수 아님. 현 visibility 유지

## 범위 (수정 대상)
### PR #53563 — project_symbols UTF-8 패닉 수정
- `crates/project_symbols/src/project_symbols.rs` L288 근처: `(*pos..pos + 1, ...)` → `(*pos..label.ceil_char_boundary(pos + 1), ...)`
- `StyledText::new(label)` → `StyledText::new(&label)` (같은 PR 내 소소한 borrow 변경)
- 테스트 추가(+100줄 근방)는 본 작업에서는 **생략**(Dokkaebi 테스트 스위트 부담 최소화)

### PR #53036 — 설정 입력 blur 저장
- `crates/settings_ui/src/components/input_field.rs`:
  - `first_render_initial_text` useState 추가
  - id 분기/no-id 분기 양쪽에 `on_focus_out` 등록 블록 추가
  - reconcile 블록을 `first_initial` 비교 + `window.defer(...)` 패턴으로 교체

### PR #51756 — agent spinner GPU 감소
- `crates/agent_ui/src/conversation_view.rs:54-57` use 문에서 `SpinnerLabel,` 제거, 포맷 재정렬
- `crates/agent_ui/src/conversation_view/thread_view.rs`:
  - use 문에 `SpinnerLabel, SpinnerVariant` 포함되도록 조정
  - `ThreadFeedbackState` 아래(L164 근처)에 `GeneratingSpinner`/`GeneratingSpinnerElement` 구조체 및 impl 추가
  - L5106 `SpinnerLabel::sand().size(LabelSize::Small)` → `h_flex().w_2().justify_center().child(GeneratingSpinnerElement::new(SpinnerVariant::Sand))`
  - L5118 `SpinnerLabel::new().size(LabelSize::Small)` → `h_flex().w_2().justify_center().child(GeneratingSpinnerElement::new(SpinnerVariant::Dots))`
  - L7457 `SpinnerLabel::new()` (subagent diff 아이콘, 범위 외) 그대로

## 수정 제외 (가드레일)
- 상류 패치의 테스트 코드 추가는 생략(안정성 개선과 무관한 부분)
- 상류 패치의 함수 visibility 변경(`pub(crate) fn` → `fn` 등) 미수용(호출처 영향 최소화)
- C 카테고리 조사 단계는 본 plan 적용 **이후** 별도 진행, 코드 수정 없음(조사만)

## 작업 단계
- [x] 1. PR #53563 project_symbols 이식
- [x] 2. `cargo check -p project_symbols` 빌드 검증 (44.53s, exit 0)
- [x] 3. PR #53036 input_field 이식
- [x] 4. `cargo check -p settings_ui` 빌드 검증 (28.03s, exit 0)
- [x] 5. PR #51756 spinner 이식 (ui import + GeneratingSpinner 구조체 + 2곳 교체)
- [x] 6. `cargo check -p agent_ui` 빌드 검증 (2m 36s, exit 0)
- [x] 7. 전체 `cargo check -p Dokkaebi` 최종 검증 (28.51s, exit 0, 신규 경고 0건)
- [x] 8. `notes.md` 갱신
- [x] 9. C 카테고리 PR 일괄 상세 조사 (파일 부재 11건 식별, 잔여 PR은 개별 심층 대조 필요로 분류)
- [x] 10. 조사 결과 요약 보고 (본 보고서)
- [x] 11. CLAUDE.md에 업스트림 백포트 절차·주의사항 추가

## 검증 방법
- 각 단계마다 개별 crate 빌드 → 최종 전체 Dokkaebi 빌드
- `cargo check -p Dokkaebi` exit 0, 신규 경고 0건

## 승인 필요 사항
- 사용자가 "즉시 적용 권장 작업후 C 카테고리 상세 조사"로 승인 완료
- 의존성 추가/구조 변경 없음
