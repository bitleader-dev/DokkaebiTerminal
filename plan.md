# 터미널 렌더링 멈춤 현상 수정

## 목표
- Dokkaebi 내장 터미널에서 TUI 앱(예: Claude Code CLI) 실행 시 화면이 멈추는 현상 해결
- 포커스 변경 없이도 터미널 렌더링이 안정적으로 지속되도록 개선

## 근본 원인
1. WM_PAINT 생성이 VSync 스레드(~16.67ms 간격)에만 의존하여, dirty 상태에서도 WM_PAINT가 없으면 그리기 불가
2. dispatch_on_main_thread()에서 PostMessageW 실패 시 wake_posted가 true로 남아 task 큐 영구 차단 가능

## 범위

### 수정하는 파일

| 파일 | 변경 내용 |
|------|-----------|
| `crates/gpui_windows/src/platform.rs` | paint check 시 WM_PAINT가 없으면 RedrawWindow(RDW_INVALIDATE) 호출하여 강제 생성 |
| `crates/gpui_windows/src/dispatcher.rs` | PostMessageW 실패 시 wake_posted를 false로 복원 |

### 수정하지 않는 것
- gpui 코어 (platform trait 변경 없음)
- terminal.rs / terminal_view.rs (기존 로직 유지)
- VSync 스레드 로직

## 작업 단계

### [x] 1단계: paint check 시 강제 윈도우 무효화 (platform.rs)
- `run_foreground_task()`의 paint check에서 WM_PAINT 미발견 시 `RedrawWindow(RDW_INVALIDATE)` 호출
- 타임아웃 핸들러의 paint check에도 동일 적용

### [x] 2단계: PostMessageW 실패 안전망 (dispatcher.rs)
- `dispatch_on_main_thread()`에서 PostMessageW 실패 시 `wake_posted`를 false로 복원

### [x] 3단계: 빌드 검증
- `cargo check -p gpui_windows`

### [x] 4단계: 문서 갱신
- notes.md 업데이트

## 검증 방법
1. 빌드 성공 확인
2. 터미널에서 대량 출력 생성 테스트 (예: `yes | head -10000`)
3. Claude Code CLI 실행 시 렌더링 멈춤 없이 지속되는지 확인

## 승인 필요 사항
- 없음 (기존 구조 내 수정, 새 의존성/API 변경 없음)
