# 시작 섹션 "새 터미널" 단축키 추가

## 목표
환영(Welcome) 페이지의 시작하기 섹션에 있는 "새 터미널"(`workspace::NewCenterTerminal`) 항목에 단축키를 매핑하여, 시작 섹션 버튼 우측에 단축키 표시가 자동으로 나타나도록 한다.

## 현재 상태
- 시작 섹션의 "새 터미널" 액션: `workspace::NewCenterTerminal { local: false }` (`crates/workspace/src/welcome.rs:184`)
- 액션 정의: `crates/workspace/src/workspace.rs:449` 부근
- 키맵 매핑: **없음** → 시작 섹션 우측 단축키 라벨 미표시
- 상단 + 메뉴의 "새 터미널"(`workspace::NewTerminal`)은 `Ctrl-Shift-\`` (default-windows.json:578)로 이미 매핑되어 있음 — 별개 액션

## 충돌 조사 (Windows 키맵)

| 후보 | 충돌 | 결과 |
|---|---|---|
| `ctrl-\`` | 없음 (macOS/Linux는 `terminal_panel::Toggle`) | **채택** |
| `ctrl-shift-\`` | `workspace::NewTerminal` | 불가 |
| `ctrl-shift-t` | `pane::ReopenClosedItem` | 불가 |
| `ctrl-alt-t` | `agent::NewThread` | 불가 |

## 변경 후 동작
- `Ctrl-\``를 누르면 어디에 포커스가 있든 `NewCenterTerminal` 액션이 디스패치되어 중앙 pane에 새 터미널 탭이 열린다.
- 환영 페이지의 시작하기 섹션 "새 터미널" 우측에 단축키 라벨 `Ctrl-\``가 자동으로 표시된다 (`SectionEntry`가 액션의 첫 번째 키 바인딩을 자동 조회).

## 범위
- `assets/keymaps/default-windows.json` — 한 줄 추가
- (확장 옵션) macOS/Linux 키맵에 동일 키를 추가할지 검토. macOS/Linux는 `Ctrl-\``가 이미 `terminal_panel::Toggle`로 점유 중 → **이번 작업에서는 Windows만 추가**.

## 수정 내용
`assets/keymaps/default-windows.json`의 Workspace 컨텍스트 내, `ctrl-shift-\`` 라인 근처에 한 줄 추가:
```json
"ctrl-`": "workspace::NewCenterTerminal",
```

## 검증
1. `cargo build -p workspace` (혹은 `cargo build`)
2. `assets/keymaps/default-windows.json`이 keymap parser에 의해 정상 로드되는지 확인 (cargo test 또는 실행 시 에러 없음)
3. 수동 검증(빌드 후 실행 권장): 환영 페이지에서 "새 터미널" 우측에 `Ctrl-\`` 표시 확인 + 단축키로 동작 확인

## 문서 갱신
- `notes.md`에 변경 항목 추가

## 작업 단계
- [x] 1. 승인 대기
- [x] 2. `default-windows.json`에 `ctrl-\`` → `workspace::NewCenterTerminal` 매핑 추가
- [x] 3. 빌드 검증 (`cargo build -p workspace` 통과, JSONC 파싱 검증 통과)
- [x] 4. notes.md 갱신
- [x] 5. 완료 보고

## 승인 필요 항목
- 단축키 선택: `Ctrl-\`` (변경 원하면 알려주세요)
- 적용 OS: Windows 키맵에만. macOS/Linux는 기존 `terminal_panel::Toggle`과 충돌하므로 제외
