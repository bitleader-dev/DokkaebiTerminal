# 확장 탭 UI 한글화 (i18n 적용)

## 목표
- Extensions 페이지의 모든 UI 문자열에 i18n 적용하여 한글 번역 표시

## 범위
- `crates/extensions_ui/src/extensions_ui.rs`: 하드코딩된 영문 문자열 → `i18n::t()` 호출로 교체
- `assets/locales/ko.json`: 한글 번역 키 추가
- `assets/locales/en.json`: 영문 키 추가

## 작업 단계

### [x] 1. ko.json, en.json에 i18n 키 추가
### [x] 2. extensions_ui.rs에 i18n 적용
### [x] 3. 빌드 검증
### [x] 4. 문서 갱신

## 승인 필요 사항
- 없음
