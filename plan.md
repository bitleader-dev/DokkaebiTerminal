# 탭 back/forward 광범위 fix plan v1

> **작성일**: 2026-05-03
> **상태**: ✅ 종료 (2026-05-03)
> **트리거**: 사용자 보고 — 탭이 여러 개 있는 상태에서 뒤로 가기/앞으로 가기 버튼이 비활성. "모든 탭에서 발생" + "옵션 B(광범위 fix)" 선택

## 목표
Pane 탭 바 좌측의 ←/→ 버튼이 모든 Item 타입(서브에이전트 뷰·터미널 뷰·마크다운 미리보기·Editor 등)에서 일관되게 동작하도록 한다. 탭 사이 이동을 nav_history 에 자동 push 해서 VSCode/IntelliJ 와 유사한 "탭 이동 history" 동작을 제공.

## 현재 상태 (재진단)
- `crates/workspace/src/pane.rs:914-919` — `can_navigate_backward/forward()` 가 `nav_history.backward_stack/forward_stack` 의 비어있음으로 결정
- `crates/editor/src/items.rs:821-824` — Editor 만 `Item::deactivated()` 에서 cursor 위치를 `push_to_nav_history` 로 push
- 다른 Item (ClaudeSubagentView, TerminalView, MarkdownPreview 등) 의 `set_nav_history` / `deactivated` 는 default empty → nav_history 에 어떤 entry 도 추가되지 않음
- 결과: Editor 가 한 번도 활성화 안 된 워크스페이스(서브에이전트 뷰만 있는 시나리오 등)에서는 backward_stack 영구히 비어 → 버튼 비활성

## 범위 (코드 변경)

### 1. `crates/workspace/src/pane.rs::NavHistory` 에 helper 추가
신규 메서드 `push_dedup_by_item()` — Item 핸들만 받아 NavigationEntry 를 backward_stack 에 push 하되, **마지막 entry 의 item_id 가 같으면 skip** (Editor 가 이미 deactivated 시 push 한 케이스와의 중복 방지).

```rust
pub fn push_dedup_by_item(
    &mut self,
    item: Arc<dyn WeakItemHandle + Send + Sync>,
    cx: &mut App,
) {
    let state = &mut *self.0.lock();
    if !matches!(state.mode, NavigationMode::Normal) {
        return; // GoingBack/GoingForward/Disabled/ClosingItem/ReopeningClosedItem 시 skip
    }
    let new_item_id = item.id();
    if state.backward_stack.back().is_some_and(|e| e.item.id() == new_item_id) {
        return; // 마지막 entry 와 같은 item — 직전 deactivated push 와의 중복 방지
    }
    if state.backward_stack.len() >= MAX_NAVIGATION_HISTORY_LEN {
        state.backward_stack.pop_front();
    }
    state.backward_stack.push_back(NavigationEntry {
        item,
        data: None,
        timestamp: state.next_timestamp.fetch_add(1, Ordering::SeqCst),
        is_preview: false,
        row: None,
    });
    state.forward_stack.clear();
    state.did_update(cx);
}
```

### 2. `Pane::activate_item()` 에서 prev_item push
`prev_item.deactivated(window, cx)` 호출 직후에 `self.nav_history.push_dedup_by_item(prev_item.downgrade_item().to_weak_arc(), cx)` 추가.

**주의**: `Box<dyn WeakItemHandle>` → `Arc<dyn WeakItemHandle + Send + Sync>` 변환이 필요할 수 있음. Editor 의 `push_to_nav_history` 에서 `nav_history.push(...)` 시 어떻게 하는지 참고. 필요시 NavHistory helper 시그너처를 `Box<dyn WeakItemHandle>` 받도록 변경.

## 작업 단계 (순서 준수)

1. **[x]** `Pane::activate_item` 에서 `prev_item.downgrade_item()` 의 반환 타입과 NavHistory.push 시그너처 호환성 확인
2. **[x]** `NavHistory::push_dedup_by_item` 메서드 추가 (위 코드)
3. **[x]** `Pane::activate_item` 의 `prev_item.deactivated()` 호출 직후 push_dedup_by_item 호출 추가
4. **[x]** `cargo check -p Dokkaebi --tests` 통과 확인 — 신규 warning/error 0건
5. **[x]** `notes.md` 갱신 — 변경 위치 + 동작 변화 기록
6. **[x]** `assets/release_notes.md` v0.5.0 `### 버그 수정` 또는 `### UI/UX 개선` 에 1항목 추가
7. **[x]** 완료 보고 + 사용자 환경 검증 권장

## 검증 방법
- `cargo check -p Dokkaebi --tests` — 신규 warning/error 0건
- 사용자 환경 수동 검증 (자동 테스트 불가):
  - (a) 서브에이전트 뷰 탭 여러 개 → 다른 탭 클릭 → ← 활성화 + 클릭 시 이전 탭으로 이동
  - (b) Editor 탭 → cursor 이동(예: 100줄 아래) → 다른 탭 클릭 → ← → Editor 의 100줄 아래 위치로 (기존 cursor history 동작 유지)
  - (c) Terminal 탭 → 다른 탭 → ← → 터미널 탭으로
  - (d) ← 한 번 → → 한 번 → 원래 탭으로 (forward stack 동작)

## 승인 필요 항목
1. **본 plan 자체 승인** — `feedback_plan_approval.md` 룰
2. **`Pane::activate_item` 동작 변경 동의** — 모든 Item 타입의 탭 활성화 변경이 nav_history 에 기록됨. 기존 동작과 차이: Editor 외 Item 도 back/forward 추적 대상
3. **NavigationEntry 의 의미 확장 동의** — 원래는 "에디터 내 cursor 위치" 추적이 주 용도. 이제 "탭 활성화 변경" 도 같은 stack 에 섞임. 두 종류가 dedup 으로 자연스럽게 섞임 (마지막 entry 가 같은 item 이면 skip)

## 리스크 및 대응
- **리스크 1**: Editor 탭 cursor 이동 후 다른 탭 클릭 → Editor.deactivated() push (row=Some(N)) → Pane push_dedup (row=None, 같은 item) skip 됨 → backward_stack 에 (Editor, row=N) 한 번만 있음. 사용자 ← → Editor 의 row=N 위치로. **기존 동작 유지 ✓**
- **리스크 2**: 두 Editor 탭 사이 이동: A.deactivated push (A, row=10) → B 활성. Pane push_dedup → 마지막이 A 라 dedup 가 (A, row=10) 와 (A, row=None) 비교. row 가 다르므로 NavigationEntry 의 `is_same_location` 기준 다른 entry. **하지만 우리는 item_id 만 비교하므로 같으니 skip**. 결과: backward_stack 에 (A, row=10) 만. ← → A 의 row=10 으로. **자연스러움 ✓**
- **리스크 3**: 서브에이전트 뷰 A → B → A (왔다갔다): A push_dedup (A) → B 활성 → B.deactivated 는 push 안 함 (Item default) → Pane push_dedup (B). A→B 이동 시 stack: [A]. B→A 이동 시 stack: [A, B]. ← 누르면 B 로. ← 한 번 더 누르면 A 로. **VSCode 와 동일 ✓**
- **리스크 4**: forward_stack clear 가 새 push 마다 발생 → ← 한 번 누르고 다른 탭 클릭하면 forward 사라짐. 브라우저/VSCode 표준 동작이라 의도된 결과
- **리스크 5**: NavigationMode 가 GoingBack/GoingForward/ClosingItem 일 때는 push_dedup 가 skip 되므로 nav 동작 중 의도치 않은 push 없음 ✓
- **리스크 6**: 새 push 시 `MAX_NAVIGATION_HISTORY_LEN` 초과 시 가장 오래된 entry pop_front. 기존 동작과 동일 ✓
- **리스크 7**: 같은 탭 안에서 pane.activate_item 이 여러 번 호출될 수 있음 (예: 같은 탭 다시 클릭). 이 경우 prev_active_item_ix == self.active_item_index 라 deactivated 호출 안 됨 → push_dedup 도 호출 안 됨. **부작용 없음 ✓**
- **롤백**: 단일 파일 변경 (pane.rs) 이라 git checkout 으로 즉시 복구

## 비범위 (다음 plan 후보)
- Item::deactivated 마이그레이션 (각 Item 이 자체 push 하도록) — 본 plan 의 광범위 fix 가 모든 케이스 커버하므로 불필요
- `Ctrl+-`/`Ctrl+Shift+-` 같은 추가 키바인딩 — 기존 GoBack/GoForward 키바인딩이 이미 활성 (Alt-Left 등)
