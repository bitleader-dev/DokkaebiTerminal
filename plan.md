# Claude Code 작업 완료 → Dokkaebi 에디터 내부 알림 (플러그인 통합)

## 목표
1. Claude Code의 `Stop`/`Notification`/`PermissionRequest` 이벤트를 **Dokkaebi 에디터 내부 알림**(workspace toast)으로 표출
2. 알림 송출용 Claude Code 플러그인을 **Dokkaebi와 함께 배포**(인스톨러 번들)
3. 설정 화면에서 **플러그인 설치 UI** 제공 (미설치 시 [설치] 버튼 / 설치 시 "설치됨" 표시)
4. 기존 "Claude Code 작업 완료 알림음" 항목을 **"작업 알림"** 으로 리네이밍하고 동작을 *플러그인 IPC 알림 표시 ON/OFF* 토글로 전환

## 승인된 결정사항 (확정)
- **5. 플러그인 위치**: `assets/claude-plugins/dokkaebi-notify-bridge/` (저장소 내, 인스톨러 번들)
- **6. 활성화 범위**: 글로벌 (`~/.claude/settings.json`)
- **7. Hook 범위**: Stop + Notification + PermissionRequest 3종
- **A. 기존 마커 파일 시스템 처리**: **A-1 완전 교체** — 마커 hook 주입 코드, terminal_view 폴링 블록, 마커 파일 시스템 전부 삭제. UX는 워크스페이스 토스트로 통일. (단, 1회 마이그레이션 코드는 유지하여 기존 사용자 `~/.claude/settings.json`에 남은 마커 hook을 자동 정리)
- **B. 플러그인 제거 기능**: **B-2** — "설치됨" 라벨 옆에 [제거] 버튼 추가. `uninstall_plugin()` 함수 신규.
- **C. "작업 알림" OFF 차단 위치**: **C-1** — Dokkaebi `handle_cli_connection`에서 `notification.task_alert` 체크 후 false면 표시 skip. 플러그인 IPC는 항상 송신.

---

## 전체 구조

```
[Claude Code 세션] --(Stop hook)--> [dokkaebi-notify-bridge 플러그인]
                                          ↓ dispatch.sh
                  Dokkaebi.exe --notify-kind stop --notify-title "..." --notify-message "..." --notify-cwd "..."
                                          ↓ IPC (CliRequest::Notify)
                  open_listener.rs::handle_cli_connection (Notify 분기)
                                          ↓ notification.task_alert 체크 (C-1)
                  workspace.show_toast() / show_app_notification()
                                          ↓
                  [Dokkaebi 에디터 워크스페이스에 토스트 표시]
```

## 작업 범위

### Phase 1 — Dokkaebi IPC 입구 (에디터 본체)
1. **`crates/cli/src/cli.rs`**
   - `pub enum NotifyKind { Stop, Idle, Permission }` 추가
   - `CliRequest::Notify { kind: NotifyKind, title: String, message: String, cwd: Option<String> }` variant 추가
2. **`crates/cli/src/main.rs`** (Args 구조체 + 분기)
   - `--notify-kind <stop|idle|permission>` (Option<String>)
   - `--notify-title <TITLE>` (Option<String>)
   - `--notify-message <MESSAGE>` (Option<String>)
   - `--notify-cwd <PATH>` (Option<String>)
   - `--notify-kind` 지정 시 paths/wait/open_new_workspace 등 무시 → IPC `Notify` 송신 후 즉시 종료
3. **`crates/zed/src/zed/open_listener.rs::handle_cli_connection`**
   - `CliRequest::Notify` 매치 추가
   - **(C-1 적용)**: `notification.task_alert` 설정 false면 즉시 `Exit { status: 0 }` 반환
   - cwd 매칭 워크스페이스 탐색 → `workspace.show_toast(...)` 호출
   - 매칭 실패 시 `show_app_notification`로 전역 표시
   - kind별 timeout: stop=5초 / idle=영구 / permission=영구
4. **`cargo check -p Dokkaebi` 검증**
5. **수동 검증** — 별도 셸에서 `Dokkaebi.exe --notify-kind stop --notify-title "T" --notify-message "M"` 직접 호출 → UI 알림

### Phase 2 — 플러그인 자산 (`assets/claude-plugins/dokkaebi-notify-bridge/`)
1. **디렉터리 구조**
   ```
   assets/claude-plugins/dokkaebi-notify-bridge/
   ├── .claude-plugin/plugin.json
   ├── hooks/hooks.json
   └── scripts/dispatch.sh
   ```
2. **plugin.json** — name/description/version/author
3. **hooks.json** — Stop / Notification(idle_prompt) / PermissionRequest 3종 등록 (warp 패턴)
4. **dispatch.sh**
   - 인자 1개(`stop`/`idle`/`permission`) → kind 결정
   - stdin JSON에서 cwd 추출
   - Dokkaebi 실행 파일 탐색: `DOKKAEBI_EXE` 환경변수 → PATH `Dokkaebi.exe` → `%LOCALAPPDATA%\Programs\Dokkaebi\Dokkaebi.exe`
   - 미발견 시 silent skip (Claude Code UX 영향 방지)
   - 실행 중 인스턴스 체크(`tasklist`) — 없으면 skip(새 창 띄우지 않음)

### Phase 3 — 설정 UI 재구성 (`crates/settings_ui/`)

#### 3-1. 설정 키 변경 (`crates/settings_content/src/settings_content.rs`)
- `claude_code_bell: Option<bool>` → `task_alert: Option<bool>` 로 **리네이밍** (의미 변화)
- 또는 `claude_code_bell` 폐기 + `task_alert` 신설(deprecation 처리)
- 신규 키: `claude_plugin_installed: Option<bool>` 불필요 — 설치 상태는 파일 시스템에서 직접 확인

#### 3-2. `crates/settings_ui/src/page_data.rs::claude_code_section`
- 기존 1개 항목 → 2개 항목으로 확장
- 항목 1: **"플러그인 설치"** — `SettingsPageItem::ActionLink` 또는 신규 `DynamicItem` 활용
  - 미설치: 라벨 "설치 안 됨" + [설치] 버튼 → 클릭 시 `install_plugin()` 호출
  - 설치됨: 라벨 "설치됨" + [제거] 버튼 → 클릭 시 `uninstall_plugin()` 호출 (B-2 확정)
  - 상태 변화는 `cx.notify()` + 파일시스템 재조회로 즉시 반영
- 항목 2: **"작업 알림"** (기존 토글의 의미 변경)
  - `json_path: "notification.task_alert"`
  - 설명: "Claude Code 작업 완료 시 워크스페이스에 알림을 표시합니다."

#### 3-3. `crates/settings_ui/src/pages/notification_setup.rs`
- **기존 마커 파일 hook 주입 코드 처리 (A-1 확정)**:
  - 삭제: `set_stop_hook_bell_enabled` / `is_stop_hook_bell_enabled` / `sync_claude_code_bell_setting`
  - 유지(1회 마이그레이션용): `is_dokkaebi_hook_entry` + 신규 `cleanup_legacy_marker_hook()` — 부팅 시 1회 호출하여 기존 사용자 `~/.claude/settings.json`에 남은 마커 hook 자동 정리. 다음 메이저 버전에서 제거.
- **신규 함수**:
  - `pub fn is_plugin_installed() -> bool` — `~/.claude/settings.json`의 `enabledPlugins["dokkaebi-notify-bridge@local"]` 또는 `extraKnownMarketplaces` 체크
  - `pub fn install_plugin() -> Result<(), String>`
    - 플러그인 source 디렉터리 결정: 설치 환경 = `{exe_dir}/plugins/dokkaebi-notify-bridge`, 개발 = `{repo}/assets/claude-plugins/dokkaebi-notify-bridge`
    - `~/.claude/settings.json`의 `extraKnownMarketplaces`에 로컬 source 등록 + `enabledPlugins`에 추가
    - **또는 더 단순한 방식**: `~/.claude/plugins/`에 심볼릭 링크/복사 후 `--plugin-dir` 등록 (Claude Code 공식 메커니즘 검증 필요)
  - `pub fn uninstall_plugin() -> Result<(), String>` — `enabledPlugins` + `extraKnownMarketplaces`에서 항목 제거 (B-2 확정)

#### 3-4. `crates/settings_ui/src/settings_ui.rs:398`
- `pages::sync_claude_code_bell_setting(cx)` 호출 → `pages::cleanup_legacy_marker_hook(cx)`로 교체 (A-1 + 1회 마이그레이션)

### Phase 4 — Terminal View 마커 폴링 코드 정리 (A-1 확정)
- **`crates/terminal_view/src/terminal_view.rs:1127~1152`** 마커 파일 폴링 블록 삭제
- `last_bell_file_check` 필드 + `Duration::from_secs(1)` 폴링 인프라 정리
- `has_bell` 필드, `notify_bell_for_item` 호출 — IPC 알림 경로에서 재사용 가능하면 유지(워크스페이스 토스트 + 탭 인디케이터 동시 표시 옵션). 미사용이면 제거.
- `terminal/src/terminal.rs:129` `DOKKAEBI_TERMINAL_ID` 환경변수 — 마커 파일 전용이었다면 함께 제거 검토

### Phase 5 — 인스톨러 (`setup/dokkaebi.iss`)
1. **`[Files]` 섹션**
   ```pascal
   Source: "{#ResourcesDir}\plugins\dokkaebi-notify-bridge\*"; \
       DestDir: "{app}\plugins\dokkaebi-notify-bridge"; \
       Flags: ignoreversion recursesubdirs createallsubdirs
   ```
2. **`[UninstallDelete]`**: `{app}\plugins` 추가
3. **빌드 워크플로우** — 인스톨러 작업 디렉터리에 `assets/claude-plugins/` → `setup/plugins/` 복사 단계 추가
   - 위치: `script/bundle-windows.ps1` 또는 GitHub Actions release workflow (확인 후 결정)

### Phase 6 — i18n
1. **변경**:
   - `settings_page.item.claude_code_task_completion_bell` → 값을 "작업 알림" / "Task Notification" 으로 갱신
     (또는 키 자체를 `settings_page.item.claude_code_task_alert`로 리네이밍 — 권장)
2. **신규 키**:
   - `settings_page.item.claude_code_plugin_install` ("플러그인 설치" / "Install plugin")
   - `settings_page.desc.claude_code_plugin_install` (설명 한 줄)
   - `settings_page.action.install` ("설치" / "Install")
   - `settings_page.label.installed` ("설치됨" / "Installed")
   - `settings_page.label.not_installed` ("설치 안 됨" / "Not installed")
3. **welcome 페이지 갱신** — `welcome.help.claude_code_sound.*` 키 의미 변경:
   - label: "Claude Code 작업 알림 받기" / "Get notified for Claude Code tasks"
   - value: "설정 → 알림 → Claude Code → 플러그인 설치 + 작업 알림 ON" / "Settings → Notifications → Claude Code → Install plugin + Task Notification ON"

### Phase 7 — 문서
1. **`notes.md`** — Phase 1~6 변경 내역 (1개월 이내 항목)
2. **`assets/release_notes.md`** — `### 새로운 기능` (작업 알림 플러그인 통합), `### 정리` (마커 파일 시스템 제거)

---

## 작업 단계
### Phase 1 — IPC
- [x] 1. `cli.rs`: `NotifyKind` enum + `CliRequest::Notify` variant
- [x] 2. `cli/main.rs`: 4개 신규 인자 + 분기
- [x] 3. `open_listener.rs`: `Notify` 매치 + `task_alert` 체크 + `show_app_notification` (cwd 라우팅은 후속 — 첫 구현은 글로벌 표시)
- [x] 4. `cargo check -p Dokkaebi` 클린 (신규 경고/에러 0건)
- [/] 5. 수동 IPC 테스트 — 모든 Phase 완료 후 일괄 검증

### Phase 2 — 플러그인 자산
- [x] 6. `assets/claude-plugins/dokkaebi-notify-bridge/` 4개 파일 작성 (plugin.json + hooks.json + dispatch.sh)

### Phase 3 — 설정 UI
- [x] 7. `settings_content.rs`: `task_alert` 신규 + `claude_code_bell` deprecation 주석 (Phase 1과 함께 처리)
- [x] 8. `page_data.rs::claude_code_section`: SectionHeader + PluginAction + SettingItem 3항목으로 확장
- [x] 9. `notification_setup.rs` 전체 재작성: 기존 마커 코드 제거 + `cleanup_legacy_marker_hook`/`is_plugin_installed`/`install_plugin`/`uninstall_plugin` 신규
- [x] 10. `settings_ui.rs:398` `observe_global` 블록 → `cleanup_legacy_marker_hook(cx)` 1회 호출로 교체. 신규 `SettingsPageItem::PluginAction` variant + `struct PluginAction` 정의 + 4개 매치문(Debug/render/filter/search) 분기 추가
- [x] 11. `cargo check -p Dokkaebi` 클린 (6.81s, 신규 경고 0건)

### Phase 4 — Terminal View 정리 (A-1 확정)
- [x] 12. `terminal_view.rs:1127~1152` 마커 폴링 블록 + `last_bell_file_check` 필드/초기화 제거
- [/] 13. `DOKKAEBI_TERMINAL_ID`/`terminal_id()`/`TERMINAL_ID_COUNTER` — 마커 시스템 잔재이나 dead code 경고가 안 나서 보류 (후속 정리). `has_bell` 필드는 `Event::Bell` 경로에서 일반 터미널 bell용으로 유지
- [x] 14. `cargo check -p Dokkaebi` 클린 (7.33s, 신규 경고 0건)

### Phase 5 — 인스톨러
- [x] 15. `setup/dokkaebi.iss` `[Files]`에 `#ifexist` 가드로 `{#ResourcesDir}\plugins\dokkaebi-notify-bridge\*` 우선, fallback으로 `..\assets\claude-plugins\dokkaebi-notify-bridge\*` 사용 + `[UninstallDelete]`에 `{app}\plugins` 추가
- [n/a] 16. 빌드 워크플로우 — script/.github에 인스톨러 자동화 스크립트 부재. iss `#ifexist`로 두 경로(setup/plugins/ vs ../assets/) 모두 지원하므로 별도 복사 단계 불필요
- [/] 17. 로컬 `iscc dokkaebi.iss` 빌드 검증 — 사용자 환경 위임

### Phase 6 — i18n
- [x] 18. ko.json/en.json — `claude_code_task_completion_bell` 키 삭제 + 신규 8개 키 추가(`claude_code_plugin_install` item/desc, `claude_code_task_alert` item/desc, `label.installed`/`not_installed`, `action.install`/`uninstall`) + `welcome.help.claude_code_sound.*` 두 줄 갱신

### Phase 7 — 문서
- [x] 19. `notes.md` 최상단에 `2026-04-19` 변경 항목 추가 (전체 11개 파일 + 인스톨러/i18n 상세)
- [x] 20. `assets/release_notes.md` v0.3.0 섹션에 "Claude Code 작업 알림 통합"(새로운 기능) + "알림 메뉴 재구성"(UI/UX) + "마커 파일 기반 알림 시스템 폐기"(정리) 3항목 추가

### 사후 fix (2026-04-19): cli 바이너리 분리 구조 누락
- [x] 22. **원인**: `dokkaebi.exe`(GUI 본체, `crates/zed/src/main.rs`)와 `cli.exe`(별도 IPC 클라이언트, `crates/cli/src/main.rs`)가 분리된 구조인데 `--notify-kind` 인자를 cli에만 추가했고 인스톨러는 `dokkaebi.exe`만 복사. dispatch.sh가 본체에 직접 인자 전달 → clap unknown argument로 실패.
- [x] 23. `dispatch.sh` 수정 — `dokkaebi.exe` 호출 → `dokkaebi-cli.exe` 호출로 변경. `DOKKAEBI_CLI` 환경변수 + PATH + `%LOCALAPPDATA%\Programs\Dokkaebi\dokkaebi-cli.exe` 순으로 탐색.
- [x] 24. `setup/dokkaebi.iss` 수정 — `[Files]`에 `dokkaebi-cli.exe` 추가 (`#ifexist` 가드).
- [x] 25. `crates/cli/Cargo.toml` `[[bin]] name`을 `cli` → `dokkaebi-cli`로 변경. 빌드 결과 즉시 `dokkaebi-cli.exe` 생성. 인스톨러 `DestName` 옵션 불필요. 개발 환경/배포 환경 이름 통일. `[package] name = "cli"`는 그대로(workspace dep 호환). `CLAUDE.md` "이미 리네이밍된 식별자" 섹션에 추가하여 상류 백포트 시 충돌 방지.

### 최종 검증
- [/] 21. **사용자 검증 필요** — 아래 단계 직접 확인 부탁:
  1. **cli 빌드**: `cargo build -p cli --release` → `target/release/dokkaebi-cli.exe` 생성
  2. **인스톨러용 복사**: `cp target/release/dokkaebi-cli.exe setup/`
  3. **인스톨러 빌드 + 재설치**: `iscc setup/dokkaebi.iss` → 생성된 setup 실행 → `{LocalAppData}\Programs\Dokkaebi\`에 `dokkaebi.exe`, `dokkaebi-cli.exe`, `plugins\dokkaebi-notify-bridge\` 모두 존재 확인
  4. **IPC 직접 테스트**: 셸에서 `"%LOCALAPPDATA%\Programs\Dokkaebi\dokkaebi-cli.exe" --notify-kind stop --notify-title "T" --notify-message "M"` → Dokkaebi UI 토스트 표시
  5. **설정 UI 확인**: 설정 → 알림 → Claude Code 섹션에 "Claude Code 플러그인" + "작업 알림" 2항목 표시
  6. **[설치] 클릭** → `~/.claude/settings.json`에 `enabledPlugins`/`extraKnownMarketplaces` 등록 + 라벨이 즉시 "설치됨" + [제거] 버튼으로 변경
  7. **End-to-end**: Claude Code 세션 실행 → 작업 완료 시 Dokkaebi 워크스페이스 토스트 알림 수신
  8. **토글 OFF 차단**: "작업 알림" 토글 OFF 후 알림 차단 동작 확인

### 빠른 디버그 테스트 (인스톨러 없이)
- 개발 빌드 환경에서: `cargo build -p cli` → `target/debug/dokkaebi-cli.exe`. 셸에서 직접 호출:
  `target\debug\dokkaebi-cli.exe --notify-kind stop --notify-title "T" --notify-message "M"`
  cli가 `target/debug/dokkaebi.exe`를 같은 디렉터리에서 찾아 IPC 전달 (이미 `cargo run -p Dokkaebi` 등으로 본체 실행 중이어야 함).

---

## 인스톨러 배포 검토 결과

**결론: 플러그인을 인스톨러에 함께 번들해야 한다.**

### 이유
1. **버전 동기화** — 신규 IPC 스키마(`CliRequest::Notify`)는 Dokkaebi 본체와 짝이라 외부 git clone 방식은 버전 어긋남 위험
2. **사용성** — 사용자가 별도로 git/디렉터리 셋업 없이 앱 설치만으로 사용 가능
3. **Claude Code 미사용자 보호** — 설치 시 자동 활성화하지 않음. 설정 UI [설치] 버튼 명시 동의 후에만 `~/.claude/settings.json` 수정

### 배포 흐름
```
저장소: assets/claude-plugins/dokkaebi-notify-bridge/  (소스)
   ↓ 빌드 워크플로우 복사
인스톨러 작업 디렉터리: setup/plugins/dokkaebi-notify-bridge/
   ↓ Inno Setup [Files]
설치 위치: {app}\plugins\dokkaebi-notify-bridge\        (앱 옆)
   ↓ 설정 UI [설치] 버튼 클릭
활성화: ~/.claude/settings.json 의 enabledPlugins + extraKnownMarketplaces 갱신
```

### 인스톨러 변경 최소 범위
- `[Files]`: 1줄 추가 (recursesubdirs)
- `[UninstallDelete]`: 1줄 추가 (`{app}\plugins`)
- 빌드 스크립트: 복사 단계 1개 추가

---

## 승인 필요 사항 (CLAUDE.md 1단계 기준)

### 구조/공개 API 변경
1. **IPC 스키마 변경**: `CliRequest::Notify` variant 신규 (상류 Zed와 divergence — 백포트 시 주의 필요)
2. **CLI 신규 인자 4종**: `--notify-kind`, `--notify-title`, `--notify-message`, `--notify-cwd`
3. **설정 키 마이그레이션**: `notification.claude_code_bell` → `notification.task_alert` (의미 변화 + 리네이밍)
4. **신규 SettingsPageItem 패턴**: ActionLink/DynamicItem로 동적 라벨+버튼 구현 (필요 시 신규 variant)

### 코드 삭제 (대규모)
5. **기존 마커 파일 시스템 제거** (A-1 채택 시):
   - `notification_setup.rs`의 hook 주입 코드 (~80 라인)
   - `terminal_view.rs:1127~1152` 폴링 블록
   - 관련 필드/메서드

### 외부 영향
6. **`~/.claude/settings.json` 수정**: 이미 기존 코드가 read/write 중. 등록 키 추가(`enabledPlugins`, `extraKnownMarketplaces`)
7. **인스톨러 배포 자산 추가**: `{app}\plugins\` 디렉터리 — 약 5~10KB
8. **빌드 워크플로우 수정**: plugin 복사 단계 추가

### 결정 사항 (확정)
- A-1 / B-2 / C-1 모두 확정 — 위 "승인된 결정사항" 섹션 참조

---

## 리스크 / 미해결

### 기술적 리스크
- **상류 Zed 백포트**: `CliRequest::Notify`는 Dokkaebi 독자 변경. 향후 Zed CLI/IPC 관련 PR 백포트 시 매번 분기 추가 필요. CLAUDE.md "백포트 체크리스트"에 항목 추가 검토.
- **마커 파일 마이그레이션**: A-1 채택 시 기존 사용자의 `~/.claude/settings.json`에 남은 마커 hook은 자동 정리 안 됨. 1회 마이그레이션 코드 유지 권장(부팅 시 1회 실행 후 다음 버전에서 제거).
- **다중 워크스페이스 라우팅 정확도**: cwd 기반 매칭은 sub-path/symlink/대소문자 환경에서 빗나갈 수 있음. 정규화 로직 필요.
- **Dokkaebi 미실행 상태 IPC**: `Dokkaebi.exe` 호출 시 인스턴스 없으면 새 창 뜨는 기본 동작. dispatch.sh에서 `tasklist` 사전 체크로 회피.
- **Claude Code 플러그인 활성화 메커니즘 검증**: `enabledPlugins` + `extraKnownMarketplaces` 조합 vs `--plugin-dir` 단순 등록 — 실제 동작하는 방식 확인 필요. 첫 구현 단계에서 양쪽 모두 시도해보고 결정.

### UX 리스크
- **알림 dismiss 동작**: stop=5초 자동, idle/permission=영구. 사용자 호불호 갈릴 수 있음 — 검증 후 조정.
- **인스톨러 크기 증가**: ~10KB 미만이라 무시 가능.
- **B-2 [제거] 버튼 클릭 시 확인 다이얼로그**: 의도치 않은 클릭 방지 위해 confirm 다이얼로그 1회 표시 권장 — 구현 시 결정.

### Dokkaebi 상류 호환성
- **이미 리네이밍된 식별자(IconName::Dokkaebi*, Plan::Dokkaebi*)와 충돌 없음** — 신규 variant라 기존 코드 영향 zero
- **백포트 노트 업데이트 필요**: CLAUDE.md "이미 리네이밍된 식별자" 섹션에 `CliRequest::Notify` 추가 검토 (기존 variant 리네이밍이 아니라 신규라 사실은 별도 항목)

---

**승인 후 착수.** 단계 1~21은 승인 전 수행하지 않는다.

**A/B/C 결정사항 확정 완료.** 남은 승인:
- **승인 필요 항목 1~8 일괄 승인**: IPC 스키마 변경, CLI 인자 4종, 설정 키 마이그레이션, 신규 SettingsPageItem 패턴, 마커 시스템 대규모 삭제, `~/.claude/settings.json` 자동 갱신, 인스톨러 자산 추가, 빌드 워크플로우 수정
