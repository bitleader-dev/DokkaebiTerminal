# PR #50221 project_panel sort_order 백포트 (2026-04-16)

## 목표
`util::paths::compare_rel_paths_by` 선행 이식 요구를 포함한 단일 PR #50221 전체 백포트.

## 사전 조사 결과
- PR #50221은 단일 커밋(`320cef37`)으로 아래 파일을 모두 수정:
  - `crates/util/src/paths.rs` (+482/-156) — **`SortMode`/`SortOrder` enum + `compare_rel_paths_by` + 헬퍼 + 기존 함수 삭제**
  - `crates/settings_content/src/workspace.rs` (+58/-0) — `ProjectPanelSortOrder` enum + 설정 필드
  - `crates/settings/src/vscode_import.rs` (+13/-1) — VSCode 설정 import
  - `crates/settings/src/settings_store.rs` (+24/-0) — 테스트만, **생략**
  - `crates/settings_ui/src/page_data.rs` (+19/-1) — Sort Order UI 항목 추가 (배열 크기 28→29)
  - `crates/settings_ui/src/settings_ui.rs` (+1/-0) — 관련 변경 1줄
  - `crates/project_panel/src/project_panel_settings.rs` (+4/-2) — `sort_order` 필드
  - `crates/project_panel/src/project_panel.rs` (+24/-28) — 호출처 교체 및 `cmp_*` 통합
  - `crates/project_panel/benches/sorting.rs` (+27/-33) — 벤치마크 재구성
  - `assets/settings/default.json` (+15/-0) — 기본값 및 주석
  - `docs/*` — **이식 대상 아님**

- Dokkaebi 기존 사용처: `compare_rel_paths_mixed`/`_files_first` 호출은 `project_panel.rs:7450,7455` + util 내부 테스트 12건 → project_panel 호출 2곳은 patch로 교체됨, util 테스트는 `compare_rel_paths_by(...)` 시그너처로 migrate 필요

## 범위 (수정 대상 파일)
1. `crates/util/src/paths.rs` — 전체 변경 반영. 단 **새 테스트 3건**(`compare_rel_paths_upper/lower/unicode`, +320줄)은 **생략**. 기존 테스트 호출 시그너처만 migrate. `compare_rel_paths_mixed_same_name_different_case` 예상 결과값 변경도 이식.
2. `crates/settings_content/src/workspace.rs` — `ProjectPanelSortOrder` enum + `sort_order` 필드 + `From<_>` impl 2개 추가
3. `crates/settings/src/vscode_import.rs` — `sort_mode`/`sort_order` 파싱 확장
4. `crates/settings_ui/src/page_data.rs` — Sort Order UI 항목 + 배열 크기
5. `crates/settings_ui/src/settings_ui.rs` — 1줄 추가 (export 등)
6. `crates/project_panel/src/project_panel_settings.rs` — `sort_order` 필드 2곳
7. `crates/project_panel/src/project_panel.rs` — `cmp_*` 4개 함수 통합, `par_sort_worktree_entries_with_mode` → `par_sort_worktree_entries`, 관찰자·빌더 코드 업데이트
8. `crates/project_panel/benches/sorting.rs` — 벤치마크 재구성 (이식 가능하지만 필수 아님)
9. `assets/settings/default.json` — `sort_order` 기본값 + 주석 추가

## 수정 제외 (가드레일)
- 업스트림 테스트 추가(+320줄, 3개 새 테스트) 생략
- `settings/src/settings_store.rs` 테스트 추가(+24줄) 생략
- `docs/*` 업데이트 대상 아님

## 작업 단계
- [ ] 1. util/paths.rs 이식 + 기존 테스트 migrate + `cargo check -p util`
- [ ] 2. settings_content/workspace.rs 이식 + `cargo check -p settings_content`
- [ ] 3. settings/vscode_import.rs 이식 + `cargo check -p settings`
- [ ] 4. settings_ui 2파일 이식 + `cargo check -p settings_ui`
- [ ] 5. project_panel_settings.rs 이식 + `cargo check -p project_panel`
- [ ] 6. project_panel.rs 이식 + 빌드 재검증
- [ ] 7. benches/sorting.rs 이식 (선택)
- [ ] 8. default.json 업데이트
- [ ] 9. 전체 `cargo check -p Dokkaebi` 최종 검증
- [ ] 10. `notes.md` 갱신 + git commit + push

## 승인 필요 사항
- 사용자 "b" 선택으로 승인 완료
