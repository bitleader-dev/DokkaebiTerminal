# 자동 업데이트 설정 연동

## 목표
설정 화면의 `자동 업데이트` 토글 값을 `GithubUpdater`가 존중하도록 수정.
- off: 앱 실행 시 자동 체크 안 함
- on: 앱 실행 시 자동 체크 (기본값)

## 범위
- `crates/github_update/Cargo.toml`: workspace 의존성 `settings` 추가 (승인 완료)
- `crates/github_update/src/github_update.rs`:
  - `AutoUpdateSetting` (Settings trait 구현) 추가
  - `init()` 진입 시 register 후 값 조회, false면 자동 체크 예약 스킵
  - 엔티티 생성·전역 등록은 항상 수행 (title_bar observe 유지)

## 작업 단계
- [x] `Cargo.toml` 의존성 추가
- [x] `AutoUpdateSetting` 타입 및 `Settings` impl 추가
- [x] `init()` 분기 추가 (자동 체크만 조건부)
- [x] `cargo build -p github_update -p zed` 검증
- [x] `notes.md` 갱신

## 검증 방법
- `cargo build -p github_update -p zed` 경고·에러 없음
- 설정 토글이 UI에 이미 노출되어 있으므로 별도 UI 작업 없음
- 설정 변경 → 앱 재시작 시 로그로 확인 가능

## 승인 필요 사항
- (승인됨) workspace 내부 crate `settings` 의존성 추가
