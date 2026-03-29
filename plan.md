# i18n (국제화) 시스템 추가 계획

## 목표
- 설정에서 언어(한국어/영어) 선택 기능 추가
- 실시간 언어 전환 지원 (앱 재시작 불필요)
- 1단계: 주요 메뉴/설정 UI 문자열 대상

## 범위
- 앱 메뉴 (Zed, File, Edit, Selection, View, Go, Run, Window, Help)
- 향후 확장: 웰컴 화면, 컨텍스트 메뉴, 다이얼로그 등

## 작업 단계

### [x] 1. i18n 크레이트 생성 (`crates/i18n/`)
- Locale enum (En, Ko) — settings_content에 정의
- I18n 글로벌 리소스 (번역 데이터 보관)
- `t()` 함수 (키 기반 번역 문자열 조회)
- 설정 변경 시 실시간 반영

### [x] 2. 리소스 파일 생성
- `assets/locales/en.json` — 영어 리소스 (120+ 키)
- `assets/locales/ko.json` — 한국어 리소스 (120+ 키)
- Assets 크레이트에 locales 포함 설정

### [x] 3. 설정 시스템 통합
- `settings_content`에 `locale` 필드 추가
- `default.json`에 `"locale": "en"` 기본값 추가
- 설정 변경 시 I18n 글로벌 갱신 + 메뉴 재구성

### [x] 4. 앱 메뉴 문자열 교체
- `app_menus.rs`의 하드코딩 문자열을 `t()` 호출로 교체

### [x] 5. 빌드 검증
- `cargo check -p zed` 성공

## 향후 확장 (2단계)
- [ ] 웰컴 화면 문자열
- [ ] 컨텍스트 메뉴 (프로젝트 패널, 에디터)
- [ ] 다이얼로그 / 토스트 메시지
- [ ] 설정 UI 패널 레이블
- [ ] 상태바 문자열

## 승인 필요 사항
- [x] 새 크레이트 추가 (i18n) — 승인됨
- [x] 외부 의존성 추가 없음 (기존 workspace 의존성만 사용)
- [x] settings_content 구조 변경 (locale 필드) — 승인됨
