# Zed v0.232.2 남은 미조사 PR 백포트 (2026-04-16, 5차)

## 목표
v0.232.2 잔여 10건 중 소규모·중규모 8건 순차 이식. 대규모 2건(#52886, #53008)은 별도 승인 대상.

## 이식 순서 (규모 순)
1. **#52268** project_symbols diff multibuffer (1파일 +12/-2)
2. **#52538** dev extension git url (1파일 +3/-7)
3. **#53017** agent UI remeasure (1파일 +45/-12)
4. **#53124** restricted modal 오버플로 (1파일 +65/-32)
5. **#51623** SVG 폰트 fallback (1파일 +120/-7)
6. **#53351** worktree 이름 표시 (3파일, branch_picker 부분 적용)
7. **#53209** ACP slash commands 복원 (3파일 +191, 부분 적용)
8. **#53196** markdown HTML 정렬 (3파일 +172)

## 별도 승인 대상 (본 plan 제외)
- **#52886** ESLint 3.0.24 버전업 (+394 줄, LSP 바이너리 버전 변경)
- **#53008** LanguageAwareStyling editor display_map 대규모 리팩토링 (7 파일 +170)

## 작업 단계
- [ ] 1. #52268 이식 + `cargo check -p project_symbols`
- [ ] 2. #52538 이식 + `cargo check -p extension`
- [ ] 3. #53017 이식 + `cargo check -p agent_ui`
- [ ] 4. #53124 이식 + `cargo check -p workspace`
- [ ] 5. #51623 이식 + `cargo check -p gpui`
- [ ] 6. #53351 부분 완성 + `cargo check -p git_ui -p agent_ui`
- [ ] 7. #53209 ACP slash commands 복원 + `cargo check -p acp_thread -p agent_ui`
- [ ] 8. #53196 markdown HTML 정렬 + `cargo check -p markdown`
- [ ] 9. 전체 `cargo check -p Dokkaebi` 최종 검증
- [ ] 10. notes.md 갱신 + git commit + push

## 가드레일
- 업스트림 테스트 추가는 매 단계 생략
- Dokkaebi 정책: macOS/Linux 키맵·설정 제외

## 승인 필요 사항
- 사용자 "a" 선택으로 범위 승인. 대규모 2건(#52886, #53008)은 본 plan 제외.
