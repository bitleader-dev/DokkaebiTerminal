# 설정 → 일반 → "자동 실행" 토글 추가 (Windows 부팅 시 자동 시작)

## 목표
설정 화면 `일반` → `일반 설정` 섹션에 **자동 실행** 토글을 추가한다.
- ON: Windows 사용자 로그인 시 Dokkaebi가 자동 실행되도록 OS에 등록
- OFF: 자동 실행 등록 해제

UX는 기존 `시스템 모니터링` 등 다른 토글과 동일한 `SettingItem` + `bool` 형태.

## 범위

### 포함 (이번 작업)
1. **설정 스키마** (`crates/settings_content/src/workspace.rs`)
   - `WorkspaceSettings`(또는 `WorkspaceSettingsContent`)에 `pub auto_start: Option<bool>` 추가
   - 기본값 주석 `Default: false`
2. **기본값** (`assets/settings/default.json`)
   - `"auto_start": false` 추가 (기존 `system_monitoring` 옆)
3. **설정 UI 항목** (`crates/settings_ui/src/page_data.rs`)
   - `general_settings_section()` 배열 크기 `[SettingsPageItem; 8]` → `[SettingsPageItem; 9]`
   - `system_monitoring` 다음에 `SettingItem` 1건 추가 (json_path="auto_start", USER 스코프)
4. **i18n 키** (`assets/locales/ko.json`, `assets/locales/en.json`)
   - `settings_page.item.auto_start` ("자동 실행" / "Auto Start")
   - `settings_page.desc.general_settings.auto_start` ("Windows 시작 시 Dokkaebi를 자동으로 실행합니다." / "Launch Dokkaebi automatically when Windows starts.")
5. **OS 연동 로직** (Windows 레지스트리)
   - 위치: `crates/zed/src/zed.rs`의 `init_ui` 또는 적절한 초기화 시점에 `cx.observe_global::<SettingsStore>` 또는 `WorkspaceSettings`의 변경 감지
   - 등록 방식: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` 의 `Dokkaebi` 값
     - ON: `RegSetValueEx`로 `std::env::current_exe()` 경로(따옴표 감싼 형태) 기록
     - OFF: `RegDeleteValue`로 제거 (없으면 무시)
   - 등록 실패 시 panic 금지, `log::warn!` 정도만 남김
   - 신규 의존성: `windows-registry` (Cargo.lock 확인 결과 `crates/recent_projects`·`gpui_windows`에서 이미 사용 중) → `crates/zed/Cargo.toml`에 추가
6. **변경 시 자동 적용 + 기동 시 일관성 보장**
   - 설정 ON인데 레지스트리 값 없음 → 기록
   - 설정 OFF인데 레지스트리 값 있음 → 삭제
   - 매 변경마다 동일 로직 호출

### 제외 (요청 범위 밖이라 수동 확인 후 후속 작업)
- "관리자 권한으로 자동 실행" 옵션 (Task Scheduler 사용) — 현재 요구는 단순 ON/OFF
- Stable/Preview 등 채널별 별도 등록값 — Dokkaebi 단일 채널 가정
- 자동 실행 시 인자(예: `--minimized`) 전달 — 요구 없음
- macOS/Linux 분기 (Windows 전용 프로젝트 규칙)

## 설계 상세

### `default.json` 위치
```jsonc
  // 상태 표시줄에 CPU/메모리/GPU 사용량 표시
  "system_monitoring": false,
  // Windows 시작 시 Dokkaebi를 자동 실행할지 여부
  "auto_start": false,
```

### `page_data.rs` 추가 항목 (system_monitoring 직후)
```rust
SettingsPageItem::SettingItem(SettingItem {
    title: "settings_page.item.auto_start",
    description: "settings_page.desc.general_settings.auto_start",
    field: Box::new(SettingField {
        json_path: Some("auto_start"),
        pick: |sc| sc.workspace.auto_start.as_ref(),
        write: |sc, v| { sc.workspace.auto_start = v; },
    }),
    metadata: None,
    files: USER,
}),
```
배열 크기 8 → 9 동시 갱신 (CLAUDE.md "설정 UI 섹션 배치" 주의 사항).

### 레지스트리 모듈 (신규 함수 1세트, `zed.rs` 또는 신설 `auto_start.rs`)
```rust
fn apply_auto_start(enabled: bool) -> std::io::Result<()> {
    use windows_registry::CURRENT_USER;
    let key = CURRENT_USER.create(r"Software\Microsoft\Windows\CurrentVersion\Run")?;
    if enabled {
        let exe = std::env::current_exe()?;
        let value = format!("\"{}\"", exe.display());
        key.set_string("Dokkaebi", &value)?;
    } else {
        let _ = key.remove_value("Dokkaebi"); // 없을 수도 있음
    }
    Ok(())
}
```
호출 지점: 앱 부팅 직후 1회 + `WorkspaceSettings` 변경 옵저버 안에서 값 변화 시. 실패는 `log::warn!`로만 남김.

## 작업 단계
- [x] 1. `WorkspaceSettings`에 `auto_start` 필드 추가 + 주석
- [x] 2. `default.json`에 키/주석 추가
- [x] 3. `page_data.rs` 항목 추가 + 배열 크기 9로 변경
- [x] 4. ko.json / en.json 키 2건 × 2언어 = 총 4건 추가
- [x] 5. `zed.rs` (또는 신설 모듈)에 `apply_auto_start` + 옵저버 + 부팅 1회 호출 구현
- [x] 6. `crates/zed/Cargo.toml`에 `windows-registry` 의존성 추가
- [x] 7. `cargo check -p Dokkaebi` 신규 경고/에러 0건 (5.20s, exit 0)
- [ ] 8. 사용자 환경 실행 검증 — 토글 ON/OFF 시 `regedit`에서 `HKCU\...\Run\Dokkaebi` 추가/삭제 확인
- [x] 9. `notes.md` + `assets/release_notes.md` 갱신 (`### 새로운 기능`)

## 승인 필요 사항 (CLAUDE.md 1단계 기준)
1. **의존성 추가**: `crates/zed/Cargo.toml`에 `windows-registry` 추가 (또는 기존 `windows` 크레이트 features 활용 — 어느 쪽이 좋은지 확인 필요)
2. **외부 호출**: Windows 레지스트리 쓰기/삭제 (HKCU 범위, 관리자 권한 불필요)
3. **공개 설정 스키마 변경**: `WorkspaceSettings`에 신규 필드 추가 (하위 호환 OK — `Option<bool>`이므로 기존 사용자 settings.json 비파괴)
4. **i18n 신규 키**: ko/en 양쪽에 `auto_start` 항목 2개

## 리스크 / 미해결
- 레지스트리 경로 따옴표 처리: 공백 포함 경로(예: `C:\Program Files\Dokkaebi\Dokkaebi.exe`) 대비 `\"...\"` 감싸기 필수. 위 코드에 반영.
- `current_exe()`가 심볼릭 링크 / 업데이트 후 갱신되는 경로일 가능성 — 토글 ON 후 앱 이동 시 등록 경로가 어긋날 수 있음. 부팅 시 자동 갱신 로직(설정 ON이면 매 시작마다 최신 exe 경로로 set_string 재기록)으로 완화.
- 기존 사용자가 직접 만든 `Dokkaebi` 레지스트리 값이 있는 경우 OFF 토글 시 그것까지 삭제됨 — 의도된 동작으로 간주.

**승인 후 착수.** 단계 1~6은 승인 전 수행하지 않는다.
