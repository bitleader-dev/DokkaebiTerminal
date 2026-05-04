# 워크스페이스 그룹별 ProjectPanel / GitPanel 멀티 인스턴스화 plan v1

> **작성일**: 2026-05-04
> **상태**: 🟡 승인 대기 (구현 미착수)
> **트리거**: 사용자 요청 — "프로젝트 패널, git 패널이 단일로 동작하고 있는데 워크스페이스 패널의 워크스페이스 항목에 따라 멀티로 각각 동작하도록 수정"

---

## 0. 용어 정합성 확인

| 사용자 표현 | 코드 매핑 | 비고 |
| --- | --- | --- |
| 워크스페이스 패널 | `WorkspaceGroupPanel` (`crates/workspace/src/workspace_group_panel.rs`) | 좌측 dock에 등록되는 워크스페이스 그룹 목록 UI |
| 워크스페이스 항목 | `WorkspaceGroupState` (`crates/workspace/src/workspace_group.rs`) | 한 `Workspace` 안의 PaneGroup·panes·active_pane 그룹 단위. UUID로 식별 |
| 프로젝트 패널 / git 패널 | `ProjectPanel` (`crates/project_panel/src/project_panel.rs:134`), `GitPanel` (`crates/git_ui/src/git_panel.rs:619`) | 현재 `Workspace` 당 1 인스턴스. dock에서 타입 단일 검색 |

**상위 개념(`MultiWorkspace`, 한 윈도우 안의 다중 `Workspace`)이 아닌 `WorkspaceGroupState` 단위 분리가 본 작업 대상**이라는 점을 먼저 확인 받고 시작한다.

---

## 1. 현재 동작 (확인 완료)

- `Workspace` 1개 = `Project` 1개 = `GitStore` 1개. 같은 Workspace 안의 모든 그룹은 **같은 Project를 공유**한다.
- `Workspace.workspace_groups: Vec<WorkspaceGroupState>` — 각 그룹은 자기 PaneGroup·panes·active_pane 등 **센터 영역 상태**만 분리해 보관 (`workspace.rs:1327`, `workspace_group.rs:11~`).
- `switch_workspace_group(index)` 호출 시 (`workspace.rs:5499` 부근):
  1. 현재 active 그룹 상태를 `workspace_groups[active]`에 캡처
  2. `active_group_index = index` 로 교체
  3. 새 그룹의 PaneGroup/panes/active_pane을 `Workspace`의 라이브 필드로 복원
  4. 패널 dock(좌/우/하)는 손대지 않음 → **ProjectPanel·GitPanel 인스턴스는 그대로 유지, 내부 상태도 그대로**
- `ProjectPanel` / `GitPanel`은 `Workspace::project()` 1개에 1:1 묶여 있고, `Dock::panel<T>()`가 타입 기반 단일 검색이라 같은 타입 다중 인스턴스 보유가 현 구조에서는 어렵다.

---

## 2. 핵심 결정 사항 — 사용자 승인 필요 (즉시 멈춤 항목)

코드 변경 전에 다음 3가지를 확정해야 한다. 같은 Project를 공유하는 구조 위에서 "멀티로 각각 동작"이 의미하는 바가 다중 해석 가능하기 때문.

### Q1. 분리 단위
사용자 의도가 다음 중 어느 것인지:

- **(A) UI 상태만 그룹별 분리** — 표시되는 파일 목록/git 변경 목록은 그룹 무관하게 동일하지만, **펼침/접힘, 선택, 마킹, 스크롤, view_mode** 등이 그룹별로 보존됨. 그룹 전환 시 한 패널 인스턴스 안에서 UI 상태를 swap.
- **(B) 패널 인스턴스를 그룹별로 분리** — 그룹마다 별도 `ProjectPanel`/`GitPanel` 엔티티를 생성. dock 활성 패널을 그룹 전환 시 swap. 결과적으로 (A)와 사용자 체감은 비슷하지만 라이프사이클이 다르다.
- **(C) Project 자체도 그룹별 분리** — 그룹마다 다른 폴더(worktree 집합)를 보고 싶다. 이것은 `WorkspaceGroupState` 범위를 벗어나며, 본질적으로 `MultiWorkspace`(다중 `Workspace`)로 이동해야 한다 → **본 plan 범위 초과**.

**권장: (A) UI 상태만 분리**. 이유:
- 데이터 레이어(`Project`, `GitStore`)는 Workspace 단위라 (B)도 결국 같은 데이터를 보게 된다.
- (B)는 dock 패널 등록·`Workspace::panel<T>()`·`SerializableItem` 트레이트·패널 키바인딩 라우팅 등 광범위한 변경이 필요하고, 그 비용에 비해 (A) 대비 사용자 체감 차이가 거의 없다.
- (A)는 패널 내부에 `HashMap<Uuid, GroupLocalState>` 추가 + 그룹 전환 이벤트 구독 2개로 구현 가능.

### Q2. 어떤 상태를 그룹별로 보존할 것인가
권장안 (A) 채택 시 분리 대상:

**ProjectPanel** (`project_panel.rs:134~165`)
- ✅ `marked_entries`, `selection`
- ✅ `state` (내부적으로 `expanded_dir_ids`, `unfolded_dir_ids` 등 펼침 상태 보관) — 필드 구조 추가 확인 필요
- ✅ `scroll_handle` 위치 (스크롤 오프셋만 추출 보존)
- ❌ `project`, `fs`, `workspace`, `filename_editor`, `clipboard`, `diagnostics*`, `update_visible_entries_task` 등 — Workspace 단위 공유 유지

**GitPanel** (`git_panel.rs:619~660`)
- ✅ `selected_entry`, `marked_entries`, `view_mode`, `bulk_staging`, `scroll_handle` 위치
- ❌ `active_repository`, `entries`, `entries_indices`, `commit_editor`, `pending_commit`, 카운트 필드들 — 데이터 측이라 공유. 단, `commit_editor` 의 입력 텍스트는 사용자에 따라 그룹별로 분리하고 싶을 가능성 있음 → **별도 확인**.

### Q3. dock 측 동기화
- 패널 dock의 위치(좌/우/하)/크기/열림 상태는 **그룹 무관 공통 유지** 권장. 그룹 전환만으로 패널이 닫혔다 열렸다 하면 사용성이 떨어짐.
- 그룹 삭제 시 해당 그룹의 패널 로컬 상태는 즉시 폐기.

---

## 3. 범위 (코드 변경 — 권장안 (A) 기준)

> 권장안이 사용자 승인을 받지 못하면 본 절은 폐기하고 (B)/(C)에 맞춰 재작성한다.

### 3-1. `crates/workspace` — 그룹 전환 이벤트 노출
- `Workspace`가 그룹 전환 시 emit 하는 이벤트 또는 노출 hook이 있는지 grep 후 확인.
- 없으면 신규 `Event::WorkspaceGroupSwitched { from: Uuid, to: Uuid }` (또는 기존 `Event` enum에 variant 추가) 도입.
- `switch_workspace_group` 종료 직전 / `add_workspace_group` 종료 직후 / `remove_workspace_group` 종료 직후 emit.
- 그룹 식별자는 인덱스가 아닌 **UUID** (`WorkspaceGroupState.uuid`) 사용 — 인덱스 이동·삭제 시에도 안정적.

### 3-2. `crates/project_panel` — 그룹별 UI 상태 보존
- `ProjectPanel` 에 `group_states: HashMap<Uuid, ProjectPanelGroupState>` + `current_group: Option<Uuid>` 필드 추가.
- `ProjectPanelGroupState { marked_entries, selection, expanded_dir_ids, unfolded_dir_ids, scroll_offset }` 구조 정의.
- `Workspace` 의 그룹 전환 이벤트 구독:
  - 직전 그룹 UUID 기준으로 현재 라이브 필드 → `group_states[prev]` 캡처
  - 신규 그룹 UUID의 `group_states` 항목을 라이브 필드로 복원 (없으면 기본값)
- 그룹 삭제 이벤트 구독 → 해당 UUID 항목 drop.
- `SerializableItem` 직렬화 키에 그룹 UUID 포함 여부는 **Q4 (별도 확인)** — 우선은 활성 그룹 상태만 직렬화.

### 3-3. `crates/git_ui` — 동일 패턴
- `GitPanel` 에 `group_states: HashMap<Uuid, GitPanelGroupState>` + `current_group` 필드 추가.
- `GitPanelGroupState { selected_entry, marked_entries, view_mode, scroll_offset, bulk_staging }`.
- 그룹 전환·삭제 이벤트 구독은 ProjectPanel 과 동일.
- `commit_editor` 텍스트 분리 여부는 **Q3 (별도 확인)**.

### 3-4. 비대상 (수정 금지)
- macOS/Linux 전용 코드, 키맵 (CLAUDE.md 규칙)
- `MultiWorkspace` (다중 `Workspace`) — 본 작업 범위 외
- `Project`, `GitStore`, `Repository` 등 데이터 레이어
- 다른 dock 패널들 (Outline/Terminal/Notepad)

---

## 4. 작업 단계 (순서 준수)

> 모든 단계는 **Q1~Q3 사용자 승인** 후 시작.

1. **[ ]** Q1~Q3 사용자 승인 확보 (필요 시 추가 질의)
2. **[ ]** `Workspace` 그룹 전환·추가·삭제 시점에 emit 되는 기존 이벤트가 있는지 전수 grep, 없으면 신규 variant 도입 + 호출 지점 3곳에 emit 추가 (`workspace.rs:5499` 부근 `switch`, `add_workspace_group`, `remove_workspace_group`)
3. **[ ]** `cargo check -p workspace` — 신규 경고/에러 0건
4. **[ ]** `ProjectPanel` 에 `group_states` HashMap + 캡처/복원 헬퍼 (`capture_group_state`, `restore_group_state`) 도입. 라이브 필드와 1:1 매핑되는 구조체 신설
5. **[ ]** `ProjectPanel::new` 에서 워크스페이스 그룹 전환·삭제 이벤트 구독 + 초기 `current_group = active_group uuid` 설정
6. **[ ]** `cargo check -p project_panel` + `cargo check -p project_panel --tests` — 신규 경고/에러 0건
7. **[ ]** `GitPanel` 에 동일 패턴 적용 (`group_states`, 캡처/복원 헬퍼, 이벤트 구독)
8. **[ ]** `cargo check -p git_ui` + `cargo check -p git_ui --tests` — 신규 경고/에러 0건
9. **[ ]** `cargo check -p Dokkaebi` 풀 빌드 — 신규 경고/에러 0건
10. **[ ]** `notes.md` 에 변경 내역 추가
11. **[ ]** `assets/release_notes.md` v0.5.0 `### UI/UX 개선` 에 사용자 가치 기준 1~2 항목 추가 (예: "워크스페이스 전환 시 프로젝트 트리 펼침/스크롤 위치 보존", "워크스페이스 전환 시 git 패널 선택·뷰 모드 보존"). 각 항목 100~150자

---

## 5. 검증 방법

### 정적 검증
- 위 작업 단계의 `cargo check` (lib + `--tests`) — 신규 경고/에러 0건 (CLAUDE.md 메모리 규칙)

### 동적 검증 (사용자 수동)
- 시나리오 1 (ProjectPanel 펼침 보존): 그룹 A에서 디렉터리 3개 펼치기 → 그룹 B 전환 (펼침 초기화 확인) → 그룹 A 복귀 (펼침 복원 확인)
- 시나리오 2 (ProjectPanel 선택 보존): 그룹 A에서 파일 선택 → B → A 복귀 시 동일 파일 selected
- 시나리오 3 (GitPanel 선택·뷰 모드 보존): 그룹 A에서 파일 선택 + 트리 뷰 토글 → B 전환 후 다른 모드 → A 복귀 시 원상태
- 시나리오 4 (그룹 삭제): 활성 그룹 삭제 → 다음 그룹으로 자동 전환되며 패널 상태가 그 그룹 것으로 복원
- 시나리오 5 (회귀 방지): 그룹이 1개뿐일 때 기존 동작과 100% 동일 (이벤트 emit 시에도 부작용 없음)

### Dev Drive os error 448 회피
- `cargo check` 실패 시 메모리 노트(`reference_dev_drive_build_error_448.md`) 절차로 복구

---

## 6. 승인 필요 사항 (CLAUDE.md "1단계 범위 확인" 기준)

- **승인 필수**:
  - Q1 분리 단위 결정 ((A)/(B)/(C))
  - Q2 분리 대상 필드 목록 — 권장안 채택 여부
  - Q3 dock 동기화 정책 — 그룹 무관 공통 유지 권장 채택 여부
  - 추가 질의: GitPanel `commit_editor` 텍스트 그룹별 분리 여부, ProjectPanel/GitPanel `SerializableItem` 직렬화 그룹 UUID 포함 여부
- **승인 불필요 (이미 승인된 사항)**:
  - 본 plan 자체 작성 — 사용자가 "상세 계획을 작성해" 라고 요청

---

## 7. 잠재 리스크

- **이벤트 누락**: `Workspace` 의 그룹 전환 시 캡처가 누락되면 직전 그룹 상태가 영구 소실. 캡처/복원 분기 단위 테스트 우선 작성.
- **UUID 안정성**: `WorkspaceGroupState.uuid` 가 그룹 분할/병합 시 어떻게 부여되는지 재확인 필요 (코드상 capture 시 외부에서 전달받음).
- **드래그 진행 중 그룹 전환**: ProjectPanel 의 진행 중 드래그(`drag_target_entry`, `_dragged_entry_destination`)는 그룹별 상태에 포함하지 않고 전환 시 폐기 권장 — 사용자 승인 시 명시.
- **`cargo check --tests` 실패**: 메모리 규칙대로 lib 만 검증하면 누락. 단계 6/8/9 에서 반드시 `--tests` 포함.
- **상류 Zed 백포트 충돌**: 본 변경은 상류에 없는 Dokkaebi 독자 변경 (워크스페이스 그룹 자체가 Dokkaebi 독자). 향후 ProjectPanel/GitPanel 백포트 시 본 변경과 병합 충돌 가능 → notes.md 기록 필수.
