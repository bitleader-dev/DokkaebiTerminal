# 누적 이슈 일괄 정리 (2026-04-16 종합)

## 목표
세션 말 종합 정리: 미커밋 잔재, 설정 UI i18n 누락, Dependabot 보안 경고, 다음 Zed 릴리즈 확인.

## 작업 단계 (모두 완료)

### (A) 🔴 미커밋 잔재 4개 일괄 커밋 — ✅ 완료 (커밋 `7ecfff6b94`)
- `Cargo.lock`, `crates/zed/Cargo.toml`: Dokkaebi 0.1.0 → 0.1.2
- `crates/zed/src/zed/app_menus.rs`: 이전 세션 메뉴 제거 작업의 실제 코드 변경
- `setup/dokkaebi.iss`: 인스톨러 버전 0.1.0 → 0.1.1
- [ ] 빌드 검증 + 커밋 + push

### (B) 🟡 설정 UI 드롭다운 영문 잔재 전수 점검 + 한글화 — ✅ 완료 (커밋 `a4211e7a11`, 31건 추가)
- `crates/settings_ui/src/settings_ui.rs`의 `add_basic_renderer::<...>` 등록 enum 전수 조회
- 각 enum의 `strum::VariantNames`로 변환되는 라벨이 ko.json에 번역돼 있는지 확인
- 누락된 항목 모두 ko.json/en.json에 추가
- [ ] 빌드 검증 + 커밋 + push

### (C) 🟠 Dependabot 보안 경고 대응 조사 — ✅ 조사 완료, 실제 업그레이드는 wasmtime 36 Zed 업스트림 동기 대기
- `gh api` 또는 GitHub 웹 페이지 확인으로 critical 2 / moderate 10 / low 4 상세 내역 파악
- 각 경고의 취약 패키지 및 권장 수정안 정리 (실제 업그레이드는 별도 승인)
- [ ] 보고

### (D) 🔵 다음 Zed 릴리즈 확인 — ✅ v0.232.2가 최신 stable, 백포트 완료 상태
- `gh release list --repo zed-industries/zed` 로 v0.232.2 이후 릴리즈 확인
- 신규 릴리즈가 있으면 간략 요약 보고 (실제 백포트는 별도 승인)
- [ ] 보고

## 수정 제외 (가드레일)
- (C) Dependabot 대응은 조사만, 실제 의존성 업그레이드는 별도 승인 후 진행
- (D) 신규 릴리즈 백포트도 별도 승인 후 진행

## 승인 필요 사항
- 사용자 "순서대로 모두 진행" 승인 완료
