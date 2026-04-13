// GitHub 릴리즈 기반 Dokkaebi 자동 업데이트 모듈.
// 앱 실행 후 한 번만 최신 릴리즈를 확인하고, 새 버전이 있으면 타이틀바 아이콘으로 알림.
// 사용자가 아이콘을 클릭하면 Windows 기본 다운로드 폴더에 설치 파일을 저장하고 실행한 뒤 앱을 종료한다.

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use gpui::{App, AppContext as _, Context, Entity, Global, Task};
use http_client::HttpClient;
use http_client::github::latest_github_release;
use release_channel::AppVersion;
use semver::Version;
use settings::{Settings, SettingsContent};
use smol::io::AsyncWriteExt;
use util::command::new_command;

/// 앱 실행 시 GitHub 릴리즈 자동 체크 여부를 담는 설정.
/// 설정 키는 최상위 `auto_update` (기본값 true).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoUpdateSetting {
    pub enabled: bool,
}

impl Settings for AutoUpdateSetting {
    fn from_settings(content: &SettingsContent) -> Self {
        // SettingsContent 병합 후 기본값이 주입되므로 unwrap_or(true)로 안전하게 처리.
        Self {
            enabled: content.auto_update.unwrap_or(true),
        }
    }
}

/// 업데이트를 확인할 대상 GitHub 저장소 (owner/repo).
const GITHUB_REPO: &str = "bitleader-dev/DokkaebiTerminal";
/// 앱 실행 후 최초 체크까지 기다리는 시간.
const INITIAL_CHECK_DELAY: Duration = Duration::from_secs(10);
/// GitHub API 체크 요청 타임아웃.
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// 설치 파일 다운로드 타임아웃.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// 릴리즈 자산 파일명 접두사.
const ASSET_NAME_PREFIX: &str = "Dokkaebi-Setup-v";
/// 릴리즈 자산 파일명 확장자.
const ASSET_NAME_SUFFIX: &str = ".exe";

/// GithubUpdater의 현재 상태.
/// `UpdateAvailable`과 `Downloading`만 UI에 표시한다.
#[derive(Clone, Debug, PartialEq)]
pub enum GithubUpdateStatus {
    Idle,
    UpdateAvailable {
        version: Version,
        asset_url: String,
    },
    Downloading {
        version: Version,
    },
    Errored,
}

/// GitHub 릴리즈 기반 업데이트 엔티티.
pub struct GithubUpdater {
    status: GithubUpdateStatus,
    current_version: Version,
    http_client: Arc<dyn HttpClient>,
    pending: Option<Task<()>>,
}

#[derive(Default)]
struct GlobalGithubUpdater(Option<Entity<GithubUpdater>>);

impl Global for GlobalGithubUpdater {}

impl GithubUpdater {
    /// 전역 업데이터 엔티티를 반환한다.
    pub fn get(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalGithubUpdater>()
            .and_then(|g| g.0.clone())
    }

    /// 업데이터를 생성하고 설정이 활성화된 경우에 한해 `INITIAL_CHECK_DELAY` 뒤 1회 자동 체크를 예약한다.
    pub fn init(http_client: Arc<dyn HttpClient>, cx: &mut App) {
        // 설정이 다른 경로에서 이미 register 되어 있을 수 있으므로 register 후 즉시 읽는다.
        AutoUpdateSetting::register(cx);
        let auto_update_enabled = AutoUpdateSetting::get_global(cx).enabled;

        let current_version = AppVersion::global(cx);
        let entity = cx.new(|_cx| Self {
            status: GithubUpdateStatus::Idle,
            current_version,
            http_client,
            pending: None,
        });
        cx.set_global(GlobalGithubUpdater(Some(entity.clone())));

        // 자동 업데이트가 꺼져 있으면 체크 예약을 건너뛴다. 엔티티 자체는 등록하여
        // 타이틀바 observe와 수동 체크 동작을 유지한다.
        if !auto_update_enabled {
            log::info!("github_update: 자동 업데이트 설정이 꺼져 있어 시작 시 체크를 건너뜁니다");
            return;
        }

        // INITIAL_CHECK_DELAY 지연 후 1회만 체크. 프리징 없이 백그라운드 타이머 사용.
        cx.spawn(async move |cx| {
            cx.background_executor().timer(INITIAL_CHECK_DELAY).await;
            let Some(updater) = cx.update(|cx| Self::get(cx)) else {
                return;
            };
            updater.update(cx, |this, cx| this.check(cx));
        })
        .detach();
    }

    /// 현재 상태 사본을 반환한다.
    pub fn status(&self) -> GithubUpdateStatus {
        self.status.clone()
    }

    /// 현재 설치된 버전.
    pub fn current_version(&self) -> Version {
        self.current_version.clone()
    }

    /// GitHub 릴리즈를 조회해 새 버전이 있으면 상태를 UpdateAvailable로 전환한다.
    /// 네트워크 호출은 백그라운드 executor에서 수행하여 메인 스레드 프리징을 방지한다.
    pub fn check(&mut self, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            return;
        }
        let http_client = self.http_client.clone();
        let current_version = self.current_version.clone();
        let bg = cx.background_executor().clone();

        // 실제 HTTP 요청과 JSON 파싱은 백그라운드 스레드에서 처리한다.
        // 네트워크가 hang되어 Task가 leak되지 않도록 CHECK_TIMEOUT 으로 상한을 둔다.
        let fetch_task = cx.background_executor().spawn(async move {
            let timeout_bg = bg.clone();
            smol::future::or(
                fetch_latest_release_info(http_client, &current_version),
                async move {
                    timeout_bg.timer(CHECK_TIMEOUT).await;
                    anyhow::bail!("GitHub API 요청 시간 초과 ({CHECK_TIMEOUT:?})")
                },
            )
            .await
        });

        // 결과를 받아 엔티티 상태를 갱신하는 래퍼만 메인 스레드에서 실행한다.
        self.pending = Some(cx.spawn(async move |this, cx| {
            let result = fetch_task.await;
            this.update(cx, |this, cx| {
                this.pending = None;
                match result {
                    Ok(Some((version, asset_url))) => {
                        log::info!(
                            "github_update: 새 버전 감지 v{} (현재 v{})",
                            version,
                            this.current_version
                        );
                        this.status = GithubUpdateStatus::UpdateAvailable { version, asset_url };
                        cx.notify();
                    }
                    Ok(None) => {
                        log::info!("github_update: 최신 버전입니다");
                    }
                    Err(err) => {
                        // 사용자는 아이콘을 보지 않으므로 조용히 로그만 남긴다.
                        log::warn!("github_update: 업데이트 확인 실패 - {err:#}");
                    }
                }
            })
            .ok();
        }));
    }

    /// 업데이트 아이콘 클릭 시 호출한다. 다운로드 → 설치 실행 → 앱 종료.
    /// 대용량 바이너리 다운로드와 디스크 쓰기는 백그라운드 스레드에서 스트리밍으로 수행한다.
    pub fn start_update(&mut self, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            return;
        }
        let GithubUpdateStatus::UpdateAvailable { version, asset_url } = self.status.clone() else {
            return;
        };

        self.status = GithubUpdateStatus::Downloading {
            version: version.clone(),
        };
        cx.notify();

        let http_client = self.http_client.clone();
        let bg_version = version.clone();
        let bg_asset_url = asset_url.clone();
        let bg = cx.background_executor().clone();

        // 다운로드·디스크 쓰기·설치 파일 spawn 까지 모두 백그라운드에서 수행한다.
        // 대용량 파일 전송을 감안해 DOWNLOAD_TIMEOUT 으로 상한을 둔다.
        let download_task = cx.background_executor().spawn(async move {
            let timeout_bg = bg.clone();
            smol::future::or(
                download_and_launch_installer(http_client, &bg_version, &bg_asset_url),
                async move {
                    timeout_bg.timer(DOWNLOAD_TIMEOUT).await;
                    anyhow::bail!("다운로드 시간 초과 ({DOWNLOAD_TIMEOUT:?})")
                },
            )
            .await
        });

        // 완료 처리(앱 종료 또는 오류 UI 전환)만 메인 스레드에서 처리한다.
        self.pending = Some(cx.spawn(async move |this, cx| {
            let result = download_task.await;
            match result {
                Ok(()) => {
                    log::info!("github_update: 설치 파일 실행 성공, 앱 종료");
                    cx.update(|cx| cx.quit());
                }
                Err(err) => {
                    log::error!("github_update: 업데이트 실패 - {err:#}");
                    this.update(cx, |this, cx| {
                        this.pending = None;
                        this.status = GithubUpdateStatus::Errored;
                        cx.notify();
                    })
                    .ok();
                }
            }
        }));
    }

    /// 현재 오류 상태를 닫고 Idle로 되돌린다.
    pub fn dismiss_error(&mut self, cx: &mut Context<Self>) {
        if matches!(self.status, GithubUpdateStatus::Errored) {
            self.status = GithubUpdateStatus::Idle;
            cx.notify();
        }
    }

    /// 디버그용 상태 전이. title_bar의 `SimulateUpdateAvailable` 액션에서 호출된다.
    pub fn update_simulation(&mut self, cx: &mut Context<Self>) {
        let simulated_version = Version::new(1, 99, 0);
        let next_state = match &self.status {
            GithubUpdateStatus::Idle => GithubUpdateStatus::UpdateAvailable {
                version: simulated_version.clone(),
                asset_url: format!(
                    "https://github.com/{GITHUB_REPO}/releases/download/v{simulated_version}/{ASSET_NAME_PREFIX}{simulated_version}{ASSET_NAME_SUFFIX}"
                ),
            },
            GithubUpdateStatus::UpdateAvailable { .. } => GithubUpdateStatus::Downloading {
                version: simulated_version,
            },
            GithubUpdateStatus::Downloading { .. } => GithubUpdateStatus::Errored,
            GithubUpdateStatus::Errored => GithubUpdateStatus::Idle,
        };
        self.status = next_state;
        cx.notify();
    }
}

/// GitHub API에서 최신 릴리즈를 조회한다.
/// 반환값이 None이면 현재 버전과 같거나 낮은 경우(업데이트 불필요).
async fn fetch_latest_release_info(
    http_client: Arc<dyn HttpClient>,
    current_version: &Version,
) -> Result<Option<(Version, String)>> {
    // `latest_github_release`는 releases 목록에서 pre-release 여부로 필터링하고 첫 번째를 반환한다.
    // 안정 릴리즈만 대상으로 한다.
    let release = latest_github_release(GITHUB_REPO, true, false, http_client).await?;

    let tag = release.tag_name.trim();
    let version_str = tag.strip_prefix('v').unwrap_or(tag);
    let latest_version = Version::parse(version_str)
        .with_context(|| format!("GitHub 태그 버전 파싱 실패: {tag}"))?;

    if latest_version <= *current_version {
        return Ok(None);
    }

    let expected_name = format!("{ASSET_NAME_PREFIX}{version_str}{ASSET_NAME_SUFFIX}");
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == expected_name)
        .with_context(|| format!("릴리즈 {tag}에 자산 {expected_name}가 없습니다"))?;

    let url = asset.browser_download_url.clone();
    if !is_trusted_github_url(&url) {
        anyhow::bail!("신뢰할 수 없는 자산 URL: {url}");
    }

    Ok(Some((latest_version, url)))
}

/// 다운로드 URL이 GitHub 공식 도메인에서 온 것인지 확인한다.
fn is_trusted_github_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
        || url.starts_with("https://objects.githubusercontent.com/")
        || url.starts_with("https://api.github.com/")
}

/// 기본 다운로드 폴더를 반환한다.
/// `dirs::download_dir()`이 실패하면 `%USERPROFILE%\Downloads`로 폴백한다.
fn resolve_download_dir() -> Result<PathBuf> {
    if let Some(dir) = dirs::download_dir() {
        return Ok(dir);
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join("Downloads"));
    }
    anyhow::bail!("Windows 기본 다운로드 폴더를 찾을 수 없습니다");
}

/// 자산을 다운로드 폴더에 저장하고 설치 파일을 실행한다.
/// 실행 후 호출자가 앱을 종료하기 때문에 Inno Setup 마법사가 사용자 안내를 대신한다.
///
/// 원자적 쓰기 전략: 먼저 `{파일명}.partial`에 스트리밍 다운로드한 뒤,
/// 성공 시 최종 파일명으로 rename한다. 실패 시 `.partial`을 삭제해 쓰레기 파일이 남지 않도록 한다.
async fn download_and_launch_installer(
    http_client: Arc<dyn HttpClient>,
    version: &Version,
    asset_url: &str,
) -> Result<()> {
    let download_dir = resolve_download_dir()?;
    // 폴더가 없으면 생성 (드물지만 안전 차원).
    smol::fs::create_dir_all(&download_dir).await.ok();

    let file_name = format!("{ASSET_NAME_PREFIX}{version}{ASSET_NAME_SUFFIX}");
    let target_path = download_dir.join(&file_name);
    let partial_path = download_dir.join(format!("{file_name}.partial"));
    log::info!(
        "github_update: 다운로드 시작 -> {}",
        target_path.display()
    );

    // 이전 시도에서 남은 .partial 파일이 있으면 먼저 제거 (클린 재시작 보장).
    if smol::fs::metadata(&partial_path).await.is_ok() {
        smol::fs::remove_file(&partial_path).await.ok();
    }

    // 실제 다운로드는 내부 함수로 위임해 실패 시 cleanup 흐름을 명확히 한다.
    if let Err(err) = download_to_partial(&http_client, asset_url, &partial_path).await {
        // 실패 시 부분 파일을 정리한다. remove 실패는 무시 (파일이 없거나 잠김 등).
        smol::fs::remove_file(&partial_path).await.ok();
        return Err(err);
    }

    // 최종 파일이 이미 있으면 먼저 제거한다.
    // Windows의 rename은 대상이 존재할 때 덮어쓰기가 제한될 수 있어 명시적으로 지운다.
    if smol::fs::metadata(&target_path).await.is_ok() {
        smol::fs::remove_file(&target_path).await.with_context(|| {
            format!("기존 파일 제거 실패: {}", target_path.display())
        })?;
    }

    // 원자적 rename: 이 시점까지 성공했다면 반드시 완성본이다.
    smol::fs::rename(&partial_path, &target_path)
        .await
        .with_context(|| {
            format!(
                "파일 이동 실패: {} -> {}",
                partial_path.display(),
                target_path.display()
            )
        })?;

    log::info!(
        "github_update: 설치 파일 준비 완료 ({}), 실행",
        target_path.display()
    );

    // Inno Setup installer를 detach하여 실행한다. 앱이 종료돼도 설치는 계속된다.
    let mut cmd = new_command(&target_path);
    cmd.spawn()
        .with_context(|| format!("설치 파일 실행 실패: {}", target_path.display()))?;

    Ok(())
}

/// `.partial` 파일로 HTTP 응답을 스트리밍 복사한다.
/// 성공하면 `.partial`에 완전한 바이너리가 쓰여 있다.
async fn download_to_partial(
    http_client: &Arc<dyn HttpClient>,
    asset_url: &str,
    partial_path: &std::path::Path,
) -> Result<()> {
    let mut response = http_client
        .get(asset_url, Default::default(), true)
        .await
        .context("자산 다운로드 요청 실패")?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("자산 다운로드 실패: HTTP status {}", status);
    }

    let mut file = smol::fs::File::create(partial_path)
        .await
        .with_context(|| format!("파일 생성 실패: {}", partial_path.display()))?;

    // 응답 본문을 청크 단위로 스트리밍하여 파일에 바로 기록한다.
    // 수백 MB 파일을 메모리에 전부 버퍼링하지 않아 RAM 사용이 일정하고 지연도 균일하다.
    let bytes_written = futures_lite::io::copy(response.body_mut(), &mut file)
        .await
        .context("자산 스트리밍 복사 실패")?;
    file.flush().await.ok();
    drop(file);

    log::info!("github_update: 다운로드 완료 ({} bytes)", bytes_written);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_url_accepts_known_domains() {
        assert!(is_trusted_github_url(
            "https://github.com/bitleader-dev/DokkaebiTerminal/releases/download/v0.0.2/Dokkaebi-Setup-v0.0.2.exe"
        ));
        assert!(is_trusted_github_url(
            "https://objects.githubusercontent.com/github-production-release-asset/foo"
        ));
        assert!(!is_trusted_github_url("http://malicious.example.com/foo"));
        assert!(!is_trusted_github_url(
            "https://example.com/repos/foo/bar/releases/latest"
        ));
    }
}
