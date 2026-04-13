# Zed Cloud 업데이트 경로 제거 + GitHub 릴리즈 기반 업데이트 신규 구현

## 목표
1. 기존 Zed Cloud 기반 업데이트 경로(`auto_update`, `auto_update_helper`, `auto_update_ui` 크레이트) 전면 삭제
2. GitHub 릴리즈 기반 신규 업데이트 구현
   - 앱 실행 후 30초 지연 → 1회 체크
   - 타이틀바 아이콘으로 "업데이트 가능" 표시
   - 클릭 시 Windows 기본 다운로드 폴더에 설치파일 저장 → 실행 → 앱 종료

## 요구사항 (승인된 질의 결과)
- **GitHub 저장소**: `bitleader-dev/DokkaebiTerminal`
- **자산 파일명**: `Dokkaebi-Setup-v{version}.exe`
- **태그 형식**: `v{version}` (예: `v0.0.2`)
- **UX**: 아이콘 클릭 즉시 다운로드 + 설치 실행 + 앱 종료
- **체크**: 앱 실행 30초 후 1회만
- **API**: `https://api.github.com/repos/bitleader-dev/DokkaebiTerminal/releases/latest`, 인증 없음, 실패 시 무시
- **다운로드 폴더**: Windows 기본 다운로드 폴더 (`dirs::download_dir()`, fallback `%USERPROFILE%\Downloads`)

## 영향 범위 조사 결과

### 삭제 대상 크레이트 (전체 제거)
1. `crates/auto_update/` — Zed Cloud 폴링/다운로드/설치 (1600+ 라인)
2. `crates/auto_update_helper/` — 앱 종료 후 바이너리 교체 헬퍼 (Windows 전용)
3. `crates/auto_update_ui/` — 업데이트 관련 다이얼로그·토스트 UI

### `auto_update` 참조 제거 필요 파일
| 파일 | 사용 내용 | 처리 방안 |
|------|-----------|-----------|
| `Cargo.toml` workspace members | 3개 auto_update 크레이트 등록 | 제거 |
| `Cargo.toml` workspace deps | `auto_update = { path = ... }` | 제거 |
| `crates/zed/Cargo.toml` | `auto_update.workspace = true` | 제거 |
| `crates/zed/src/main.rs:658` | 이미 주석처리된 init 호출 | 주석 라인 제거 |
| `crates/zed/src/zed/app_menus.rs:67` | `auto_update::Check` 메뉴 | 메뉴 항목 삭제 (또는 GitHub 수동 체크로 대체) |
| `crates/title_bar/Cargo.toml` | 의존성 | 제거 |
| `crates/title_bar/src/update_version.rs` | AutoUpdater/AutoUpdateStatus observe | GithubUpdater로 전환 |
| `crates/activity_indicator/Cargo.toml` | 의존성 | 제거 |
| `crates/activity_indicator/src/activity_indicator.rs` | `auto_update::DismissMessage` 액션 사용 (line 1, 297, 504, 533) | 로컬 action으로 재정의하거나 다른 용도로 변경 |
| `crates/remote_connection/Cargo.toml` | 의존성 | 제거 |
| `crates/remote_connection/src/remote_connection.rs:5, 484, 515` | `AutoUpdater::download_remote_server_release`, `get_remote_server_release_url` 호출 | **⚠ 중요 이슈** (아래 참조) |

### ⚠ remote_connection 이슈 (승인 필요)
`remote_connection` 크레이트의 `download_server_binary_locally` (484) / `get_download_url` (515) 함수가 SSH 원격 접속 시 `zed-remote-server` 바이너리를 Zed Cloud에서 받아오는 데 사용됨.

이 두 함수는 `crates/remote_connection/src/remote_connection.rs`의 `RemoteClientDelegate` 내부 메서드이며, SSH 원격 개발(remote development) 기능의 일부.

**선택지**:
- (A) **두 메서드 스텁화**: `anyhow::bail!("SSH 원격 서버 자동 다운로드는 Dokkaebi에서 지원하지 않습니다")` — 최소 수정, 빌드 통과. SSH 원격 연결 시도 시 친절한 에러 표시
- (B) **SSH 원격 기능 유지하되 수동 경로만**: 두 메서드를 제거하고 호출부를 정리 (RemoteClient 로직 일부 재작성 필요, 범위 확대)
- (C) **SSH 원격 기능 자체를 비활성화**: `remote_connection` 크레이트 전체 제거 또는 init 호출 제거 (범위 크게 확대)

**권장안**: **(A) 스텁화** — 원격 개발 기능을 쓰지 않는 현재 사용 흐름에서 문제가 없고, 수정 범위가 최소. 필요 시 추후 선택지 확장 가능.

### DismissMessage 액션 처리
`activity_indicator`에서 쓰는 `auto_update::DismissMessage` 액션은 상태 인디케이터 메시지를 닫는 범용 액션으로 쓰이고 있음. 업데이트와 무관하므로 `activity_indicator` 내부에 동명 액션을 로컬 정의하여 대체.

### Check 메뉴 처리
`crates/zed/src/zed/app_menus.rs`의 "Check for Updates" 메뉴 항목은 **삭제**. (요구사항에서 수동 체크가 요구되지 않음. 자동 체크 1회만.)

## 신규 모듈: `crates/auto_update/src/github_update.rs`
**크레이트 이름을 재활용 방식 변경**: `auto_update` 크레이트를 삭제하고 같은 이름으로 **새로 만들기**보다는, 혼란을 피하기 위해 **새 크레이트 `crates/github_update/`로 생성**.

### 새 크레이트: `crates/github_update/`
- `Cargo.toml`: `name = "github_update"`, 의존성: `anyhow`, `gpui`, `http_client`, `semver`, `serde`, `serde_json`, `release_channel`(AppVersion), `dirs`, `util`, `log`, `smol`
- `src/github_update.rs`:

```rust
pub struct GithubUpdater {
    status: GithubUpdateStatus,
    current_version: Version,
    http_client: Arc<HttpClientWithUrl>,
    pending: Option<Task<()>>,
}

pub enum GithubUpdateStatus {
    Idle,
    UpdateAvailable { version: Version, asset_url: String },
    Downloading { version: Version },
    Errored,
}
```

핵심 API:
- `pub fn init(http_client, cx: &mut App)` — 전역 엔티티 등록 + 30초 지연 후 1회 check
- `fn check(&mut self, cx)` — GitHub API 호출 → 파싱 → 새 버전이면 상태 전환
- `pub fn start_update(&mut self, cx)` — 다운로드 → installer spawn → `cx.quit()`
- `pub fn status(&self) -> GithubUpdateStatus`

상수: `const GITHUB_REPO: &str = "bitleader-dev/DokkaebiTerminal";`

### 타이틀바 연동
`crates/title_bar/src/update_version.rs` 재작성:
- AutoUpdater 관련 참조 전부 제거
- GithubUpdater observe
- `Render`: `UpdateAvailable`이면 클릭 가능한 아이콘 + tooltip (버전 표시), 클릭 시 `GithubUpdater::start_update`
- `Downloading`이면 로딩 아이콘
- 그 외 상태는 `Empty`

### 초기화
`crates/zed/src/main.rs` 또는 `zed.rs`의 init 경로에서 `github_update::init(client.http_client(), cx)` 호출 (위치: 기존 `auto_update::init` 호출 위치였던 곳 근처).

## i18n 키 추가
`assets/locales/ko.json`, `assets/locales/en.json`:
- `update.available` — "업데이트 가능: v{version}" / "Update available: v{version}"
- `update.downloading` — "다운로드 중..." / "Downloading..."
- `update.click_to_install` — "클릭하여 설치" / "Click to install"

## UI 프리징 방지 보증
- 30초 지연: `cx.background_executor().timer()` 사용
- API/다운로드: 비동기 `http_client`
- 상태 변경 시만 `cx.notify()`

## 보안
- HTTPS 강제
- 자산 URL 도메인 검증 (`github.com`, `githubusercontent.com`)
- 다운로드 파일은 기본 다운로드 폴더에만 쓰기, 동일 파일 덮어쓰기

## 검증 방법
1. `cargo build -p zed -p title_bar -p activity_indicator -p remote_connection -p github_update` — 에러 없음
2. 전체 `cargo build` — auto_update 참조 누락 없음
3. 앱 실행 → UI 프리징 없음, 30초 내 반응성 확인
4. 시나리오:
   - (A) 최신 버전이 현재와 같음 → 아이콘 미노출
   - (B) 새 버전 존재 → 30초 후 아이콘, 클릭 → 다운로드 진행 → 다운로드 완료 시 installer 실행 + 앱 종료
   - (C) 네트워크 오류 → 아이콘 미노출
   - (D) SSH 원격 시도 → 스텁 에러 메시지 표시 (승인된 경우)

## 작업 단계
- [ ] 1. `crates/auto_update/`, `crates/auto_update_helper/`, `crates/auto_update_ui/` 전체 삭제
- [ ] 2. `Cargo.toml` workspace 멤버/의존성 정리
- [ ] 3. `crates/github_update/` 신규 크레이트 생성 + 구현
- [ ] 4. `title_bar/src/update_version.rs` 재작성 (GithubUpdater 연동)
- [ ] 5. `activity_indicator/src/activity_indicator.rs` DismissMessage 로컬 재정의
- [ ] 6. `remote_connection/src/remote_connection.rs` 두 메서드 스텁화 (승인 시)
- [ ] 7. `zed/src/zed/app_menus.rs` Check 메뉴 항목 제거
- [ ] 8. `zed/src/main.rs` init 호출 라인 정리 및 github_update init 추가
- [ ] 9. i18n 리소스 추가
- [ ] 10. `cargo build` 전체 검증
- [ ] 11. notes.md 갱신
- [ ] 12. 앱 실행 테스트 (사용자 확인)

## 승인 필요 항목 (진행 전 모두 승인 요청)
1. ✅ **크레이트 삭제**: `auto_update`, `auto_update_helper`, `auto_update_ui` 3개 전체 삭제
2. ✅ **워크스페이스 Cargo.toml 수정**: members/dependencies에서 위 3개 제거
3. ✅ **신규 크레이트 생성**: `crates/github_update/` (의존성 모두 기존 워크스페이스 의존성)
4. ✅ **title_bar 재작성**: `update_version.rs`의 AutoUpdater 관련 코드 전부 교체
5. ✅ **activity_indicator 수정**: `DismissMessage` 액션 로컬 재정의 (Zed Cloud 무관화)
6. ⚠ **remote_connection 처리 방안 선택 (A/B/C 중 택1)** — **권장: (A) 스텁화**
7. ✅ **앱 메뉴 수정**: "Check for Updates" 메뉴 항목 삭제
8. ✅ **main.rs init 호출 정리**
9. ✅ **i18n 키 3개 추가**

**승인 요청 항목 6번(remote_connection)** 선택을 포함해 승인 주시면 1단계부터 진행합니다.
