# 자식 파일 클릭 시 selection이 워크트리 root로 튀는 문제 수정

## 배경
직전 작업 후 사용자 검증에서 새로운 부작용이 발견됨:
- 프로젝트 패널에서 **하위 파일/폴더를 클릭하면 selection이 즉시 워크트리 root 행으로 이동**한다.

## 원인
1. 자식 파일 클릭 → `selection`이 그 entry로 갱신
2. `Render::render` → `sync_active_worktree_from_selection` → `set_active_worktree(selection.worktree_id)` 호출
3. `set_active_worktree` 안의 `project.set_active_path(Some(ProjectPath { worktree_id, path: root_entry_path }))` 호출 (직전 작업의 issue 2 fix용)
4. `Project::set_active_path`(`crates/project/src/project.rs:4554`)가 `active_entry`를 root entry id로 설정하고 `Event::ActiveEntryChanged(root_entry_id)` emit
5. `project_panel.rs:677`의 구독 핸들러가 `auto_reveal_entries` 설정 켜져 있을 때 `reveal_entry(root_entry_id)`(`project_panel.rs:6325`) 호출
6. `reveal_entry`가 `selection`을 root entry로 덮어씀 → 사용자가 클릭한 자식 파일 selection이 사라짐

즉, 터미널 cwd 동기화를 위해 `Project::active_entry`를 root entry로 강제 설정한 게 auto-reveal 연쇄를 일으킨 것이다.

## 결정 사항 (사용자 승인 완료)
- **terminal_view 측에서 `workspace.active_worktree_override`를 우선 참조**해서 cwd를 결정한다.
- `Project::set_active_path` 호출은 `set_active_worktree`에서 **제거**한다 → `Project::active_entry` 부수효과 제거 → reveal 연쇄 차단.
- title_bar / workspace / git_store 등 다른 곳은 그대로 둔다.

## 범위

### 1. terminal_view — `default_working_directory` 헬퍼들이 override 우선 참조
- **파일**: `crates/terminal_view/src/terminal_view.rs:2195` `current_project_directory`, `:2206` `first_project_directory`
- 두 헬퍼 시작 부분에 다음 분기 추가:
  - `workspace.active_worktree_override()`가 Some이면, 해당 `WorktreeId`로 `workspace.project().read(cx).worktree_for_id(id, cx)`를 찾고, 그 worktree의 `abs_path()`를 `PathBuf`로 반환.
  - root_entry가 dir인지 확인 (단일 파일 worktree 회피).
  - override가 None이거나 worktree를 못 찾으면 기존 흐름으로 폴백.
- 영향 범위:
  - `WorkingDirectory::CurrentFileDirectory`: `active_entry_directory`가 Some이면 그것 우선 (변화 없음). None이면 `current_project_directory`로 폴백 → 여기서 override 적용됨.
  - `WorkingDirectory::CurrentProjectDirectory`: `current_project_directory` 직접 호출 → override 우선 적용됨.
  - `WorkingDirectory::FirstProjectDirectory`: `first_project_directory` 직접 호출 → override 우선 적용됨.
  - `WorkingDirectory::AlwaysHome`/`Always{...}`: 영향 없음.
- import 추가: `WorktreeId`가 이미 import됐는지 확인 후 필요시 추가.

### 2. project_panel — `set_active_worktree`에서 `set_active_path` 호출 제거
- **파일**: `crates/project_panel/src/project_panel.rs:6567` `set_active_worktree`
- `project.update(cx, |project, cx| { project.set_active_path(...) })` 블록 전체 삭제.
- 추출해 두던 `root_entry_path` 변수도 더 이상 필요 없으면 정리.
- workspace.override 갱신 + git_store 직접 매칭 후 `set_as_active_repository` 호출 부분은 유지 (git_panel 전환에 필요).

## 손대지 않는 것
- workspace.rs (그대로)
- title_bar.rs (그대로)
- git_store.rs (그대로)
- project.rs `set_active_path` (그대로)
- terminal_view.rs `default_working_directory` 본체 match문 구조 (그대로)
- project_panel의 시각 강조, sync_active_worktree_from_selection 본체 (그대로)

## 작업 단계
- [x] 0. plan.md 작성 + 사용자 승인 완료
- [x] 1. `terminal_view::current_project_directory`/`first_project_directory`에 override 우선 분기 추가 (헬퍼 `active_override_worktree_directory`로 공통화)
- [x] 2. `project_panel::set_active_worktree`에서 `set_active_path` 호출과 root_entry 추출 제거
- [x] 3. 빌드 검증: `cargo build -p terminal_view` (39s) → `cargo build -p project_panel` (7s) → `cargo build` 전체 (1m 54s) 모두 성공
- [x] 4. notes.md 갱신
- [x] 5. 완료 보고

## 검증 방법
- 빌드: 위 3개
- 수동:
  - 프로젝트 3개 추가
  - 각 프로젝트의 자식 파일을 클릭 → **selection이 클릭한 파일에 그대로 유지**되는지 (root로 튀지 않는지)
  - 각 프로젝트의 자식 파일/폴더 클릭 후 새 터미널 탭 → cwd가 클릭한 프로젝트의 root 디렉토리인지
  - git_panel이 클릭한 프로젝트로 정확히 전환되는지 (1→2→3→1 순서)
  - 시각 강조(좌측 2px accent 바)가 활성 워크트리 root 행에 그대로 그려지는지

## 승인 필요 항목
없음 (사용자 사전 승인 완료)
