#![allow(
    clippy::disallowed_methods,
    reason = "We are not in an async environment, so std::process::Command is fine"
)]
#![cfg_attr(
    any(target_os = "linux", target_os = "freebsd", target_os = "windows"),
    allow(dead_code)
)]

use anyhow::{Context as _, Result};
use clap::Parser;
use cli::{CliRequest, CliResponse, IpcHandshake, NotifyKind, SubagentPayload, ipc::IpcOneShotServer};
use parking_lot::Mutex;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::{Child, ExitStatus},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// cli가 `App::launch()`에서 `cmd.spawn()`으로 본체를 새로 띄운 경우 반환된
/// `Child` 핸들을 저장한다. handshake 워치독이 타임아웃 시 이 핸들로
/// spawn된 본체를 강제 종료해 좀비 프로세스가 남지 않도록 한다. pipe 경로
/// (기존 본체 재사용)에서는 저장하지 않는다.
static SPAWNED_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn spawned_child_slot() -> &'static Mutex<Option<Child>> {
    SPAWNED_CHILD.get_or_init(|| Mutex::new(None))
}

/// handshake 대기 최대 시간. 이보다 길면 본체가 UI 초기화에 실패한 좀비로
/// 간주하고 spawn된 Child를 kill한 뒤 cli가 실패 종료한다.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

/// cli가 본체에 대한 IPC handshake 완료를 `HANDSHAKE_TIMEOUT` 내 감지하지
/// 못하면 spawn된 Child를 kill하고 프로세스를 에러 코드로 즉시 종료한다.
/// 이 보장 덕에 cli가 띄운 본체가 UI 초기화 실패로 좀비화하더라도 다음 번
/// cli 호출이 계속 같은 좀비에 IPC를 보내 실패하는 연쇄 문제가 방지된다.
fn spawn_handshake_watchdog(sender_done: Arc<AtomicBool>) {
    thread::Builder::new()
        .name("CliHandshakeWatchdog".to_string())
        .spawn(move || {
            thread::sleep(HANDSHAKE_TIMEOUT);
            if sender_done.load(Ordering::SeqCst) {
                return;
            }
            // 타임아웃. spawn된 Child가 있으면 종료한다.
            // lock 확보 후 `sender_done` 을 재검사해 sleep 종료 직후 sender가
            // 완료된 경우의 race를 방지한다.
            let killed_pid = {
                let mut slot = spawned_child_slot().lock();
                if sender_done.load(Ordering::SeqCst) {
                    return;
                }
                slot.take().map(|mut child| {
                    let pid = child.id();
                    let _ = child.kill();
                    let _ = child.wait();
                    pid
                })
            };
            match killed_pid {
                Some(pid) => eprintln!(
                    "dokkaebi-cli: handshake timeout ({}s). Killed spawned instance pid={}.",
                    HANDSHAKE_TIMEOUT.as_secs(),
                    pid,
                ),
                None => eprintln!(
                    "dokkaebi-cli: handshake timeout ({}s). No spawned child to kill (pipe path).",
                    HANDSHAKE_TIMEOUT.as_secs(),
                ),
            }
            std::process::exit(1);
        })
        .expect("spawn handshake watchdog thread");
}
use tempfile::{NamedTempFile, TempDir};
use util::paths::PathWithPosition;
use walkdir::WalkDir;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use std::io::IsTerminal;

const URL_PREFIX: [&'static str; 5] = ["zed://", "http://", "https://", "file://", "ssh://"];

struct Detect;

trait InstalledApp {
    fn zed_version_string(&self) -> String;
    fn launch(&self, ipc_url: String, user_data_dir: Option<&str>) -> anyhow::Result<()>;
    fn run_foreground(
        &self,
        ipc_url: String,
        user_data_dir: Option<&str>,
    ) -> io::Result<ExitStatus>;
    fn path(&self) -> PathBuf;
}

#[derive(Parser, Debug)]
#[command(
    name = "zed",
    disable_version_flag = true,
    before_help = "The Zed CLI binary.
This CLI is a separate binary that invokes Zed.

Examples:
    `zed`
          Simply opens Zed
    `zed --foreground`
          Runs in foreground (shows all logs)
    `zed path-to-your-project`
          Open your project in Zed
    `zed -n path-to-file `
          Open file/folder in a new window",
    after_help = "To read from stdin, append '-', e.g. 'ps axf | zed -'"
)]
struct Args {
    /// Wait for all of the given paths to be opened/closed before exiting.
    ///
    /// When opening a directory, waits until the created window is closed.
    #[arg(short, long)]
    wait: bool,
    /// Add files to the currently open workspace
    #[arg(short, long, overrides_with_all = ["new", "reuse"])]
    add: bool,
    /// Create a new workspace
    #[arg(short, long, overrides_with_all = ["add", "reuse"])]
    new: bool,
    /// Reuse an existing window, replacing its workspace
    #[arg(short, long, overrides_with_all = ["add", "new"])]
    reuse: bool,
    /// Sets a custom directory for all user data (e.g., database, extensions, logs).
    /// This overrides the default platform-specific data directory location:
    #[cfg_attr(target_os = "macos", doc = "`~/Library/Application Support/Zed`.")]
    #[cfg_attr(target_os = "windows", doc = "`%LOCALAPPDATA%\\Zed`.")]
    #[cfg_attr(
        not(any(target_os = "windows", target_os = "macos")),
        doc = "`$XDG_DATA_HOME/zed`."
    )]
    #[arg(long, value_name = "DIR")]
    user_data_dir: Option<String>,
    /// The paths to open in Zed (space-separated).
    ///
    /// Use `path:line:column` syntax to open a file at the given line and column.
    paths_with_position: Vec<String>,
    /// Print Zed's version and the app path.
    #[arg(short, long)]
    version: bool,
    /// Run zed in the foreground (useful for debugging)
    #[arg(long)]
    foreground: bool,
    /// Custom path to Zed.app or the zed binary
    #[arg(long)]
    zed: Option<PathBuf>,
    /// Run zed in dev-server mode
    #[arg(long)]
    dev_server_token: Option<String>,
    /// The username and WSL distribution to use when opening paths. If not specified,
    /// Zed will attempt to open the paths directly.
    ///
    /// The username is optional, and if not specified, the default user for the distribution
    /// will be used.
    ///
    /// Example: `me@Ubuntu` or `Ubuntu`.
    ///
    /// WARN: You should not fill in this field by hand.
    #[cfg(target_os = "windows")]
    #[arg(long, value_name = "USER@DISTRO")]
    wsl: Option<String>,
    /// Not supported in Zed CLI, only supported on Zed binary
    /// Will attempt to give the correct command to run
    #[arg(long)]
    system_specs: bool,
    /// Pairs of file paths to diff. Can be specified multiple times.
    /// When directories are provided, recurses into them and shows all changed files in a single multi-diff view.
    #[arg(long, action = clap::ArgAction::Append, num_args = 2, value_names = ["OLD_PATH", "NEW_PATH"])]
    diff: Vec<String>,
    /// Uninstall Zed from user system
    #[cfg(all(
        any(target_os = "linux", target_os = "macos"),
        not(feature = "no-bundled-uninstall")
    ))]
    #[arg(long)]
    uninstall: bool,

    /// Used for SSH/Git password authentication, to remove the need for netcat as a dependency,
    /// by having Zed act like netcat communicating over a Unix socket.
    #[arg(long, hide = true)]
    askpass: Option<String>,

    /// Claude Code 플러그인 → Dokkaebi 작업 알림 전달용 인자.
    /// 지정 시 paths/wait/diff 등 다른 인자는 무시되고 알림 IPC만 송신 후 즉시 종료한다.
    /// 값: "stop" | "idle" | "permission" | "subagent-start" | "subagent-stop"
    #[arg(long, hide = true, value_name = "KIND")]
    notify_kind: Option<String>,
    /// 알림 발생 위치 cwd. 다중 워크스페이스 라우팅 힌트로 사용된다.
    #[arg(long, hide = true, value_name = "PATH", requires = "notify_kind")]
    notify_cwd: Option<String>,
    /// 알림 송신 프로세스의 PID(dispatch.sh의 `$PPID`). 본체가 parent chain을
    /// 따라가며 정확한 터미널을 식별하는 데 쓴다.
    #[arg(long, hide = true, value_name = "PID", requires = "notify_kind")]
    notify_pid: Option<u32>,
    /// Stop 이벤트용 사용자 프롬프트 요약 (200자 truncate 권장).
    #[arg(long, hide = true, value_name = "PROMPT", requires = "notify_kind")]
    notify_prompt: Option<String>,
    /// Stop 이벤트용 어시스턴트 응답 요약 (200자 truncate 권장).
    #[arg(long, hide = true, value_name = "RESPONSE", requires = "notify_kind")]
    notify_response: Option<String>,
    /// Permission 이벤트용 도구 이름.
    #[arg(long, hide = true, value_name = "TOOL", requires = "notify_kind")]
    notify_tool_name: Option<String>,
    /// Permission 이벤트용 도구 입력 preview (command/file_path 등, 120자 truncate 권장).
    #[arg(long, hide = true, value_name = "PREVIEW", requires = "notify_kind")]
    notify_tool_preview: Option<String>,
    /// Idle 이벤트용 Claude Code 원본 메시지.
    #[arg(long, hide = true, value_name = "SUMMARY", requires = "notify_kind")]
    notify_idle_summary: Option<String>,
    /// Subagent 이벤트용 Claude Code session id.
    #[arg(long, hide = true, value_name = "SESSION_ID", requires = "notify_kind")]
    notify_session_id: Option<String>,
    /// Subagent 이벤트용 transcript JSONL 경로.
    #[arg(long, hide = true, value_name = "PATH", requires = "notify_kind")]
    notify_transcript_path: Option<String>,
    /// Subagent 이벤트용 tool_input 기반 안정적 해시 id.
    #[arg(long, hide = true, value_name = "ID", requires = "notify_kind")]
    notify_subagent_id: Option<String>,
    /// Subagent 시작용 subagent_type.
    #[arg(long, hide = true, value_name = "TYPE", requires = "notify_kind")]
    notify_subagent_type: Option<String>,
    /// Subagent 시작용 description (200자 truncate 권장).
    #[arg(long, hide = true, value_name = "DESC", requires = "notify_kind")]
    notify_subagent_description: Option<String>,
    /// Subagent 시작용 prompt (500자 truncate 권장).
    #[arg(long, hide = true, value_name = "PROMPT", requires = "notify_kind")]
    notify_subagent_prompt: Option<String>,
    /// Subagent 종료용 최종 결과 텍스트 (1000자 truncate 권장).
    #[arg(long, hide = true, value_name = "RESULT", requires = "notify_kind")]
    notify_subagent_result: Option<String>,

    /// 인스톨러 언인스톨 훅 전용. 지정 시 다른 인자를 무시하고
    /// ~/.claude/settings.json 에서 Dokkaebi 알림 브리지 등록 항목을
    /// 제거한 뒤 즉시 종료한다. 사용자 직접 호출 경로가 아니므로 hidden.
    #[arg(long, hide = true)]
    uninstall_claude_plugin: bool,
}

/// Windows에서 현재 프로세스(cli)의 Win32 parent chain을 수집한다.
/// 반환값 `[cli, parent(bash), grandparent(Claude), ..., root]` 형태이며
/// dispatch.sh가 아직 살아있는 IPC 호출 전 시점에 Toolhelp snapshot으로
/// 확보하므로 본체 쪽 sysinfo가 exit한 부모를 놓쳐 chain이 끊기는 문제를
/// 우회한다.
///
/// 주의: cli/main.rs 안쪽에 `mod windows {...}` 내부 모듈이 있어 crate root의
/// `windows` 이름을 shadow하므로 외부 `windows` crate는 `::windows::` 절대
/// 경로로 접근해야 한다.
#[cfg(target_os = "windows")]
fn windows_ancestor_pids() -> Vec<u32> {
    use ::windows::Win32::Foundation::CloseHandle;
    use ::windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use std::collections::HashMap;

    // 1) 전체 프로세스의 (PID, ParentPID) 맵 구축.
    let mut pid_to_parent: HashMap<u32, u32> = HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return vec![std::process::id()];
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                pid_to_parent.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }

    // 2) 자기 PID부터 parent 연쇄 추적. cycle 방지 guard 64.
    let mut chain = Vec::with_capacity(8);
    let mut current = std::process::id();
    let mut guard = 0usize;
    loop {
        if guard > 64 {
            break;
        }
        guard += 1;
        chain.push(current);
        let Some(&parent) = pid_to_parent.get(&current) else {
            break;
        };
        // PID 0(System Idle) 또는 자기 루프 방지
        if parent == 0 || parent == current || chain.contains(&parent) {
            break;
        }
        current = parent;
    }
    chain
}

#[cfg(not(target_os = "windows"))]
fn windows_ancestor_pids() -> Vec<u32> {
    Vec::new()
}

/// Parses a path containing a position (e.g. `path:line:column`)
/// and returns its canonicalized string representation.
///
/// If a part of path doesn't exist, it will canonicalize the
/// existing part and append the non-existing part.
///
/// This method must return an absolute path, as many zed
/// crates assume absolute paths.
fn parse_path_with_position(argument_str: &str) -> anyhow::Result<String> {
    match Path::new(argument_str).canonicalize() {
        Ok(existing_path) => Ok(PathWithPosition::from_path(existing_path)),
        Err(_) => PathWithPosition::parse_str(argument_str).map_path(|mut path| {
            let curdir = env::current_dir().context("retrieving current directory")?;
            let mut children = Vec::new();
            let root;
            loop {
                // canonicalize handles './', and '/'.
                if let Ok(canonicalized) = fs::canonicalize(&path) {
                    root = canonicalized;
                    break;
                }
                // The comparison to `curdir` is just a shortcut
                // since we know it is canonical. The other one
                // is if `argument_str` is a string that starts
                // with a name (e.g. "foo/bar").
                if path == curdir || path == Path::new("") {
                    root = curdir;
                    break;
                }
                children.push(
                    path.file_name()
                        .with_context(|| format!("parsing as path with position {argument_str}"))?
                        .to_owned(),
                );
                if !path.pop() {
                    unreachable!("parsing as path with position {argument_str}");
                }
            }
            Ok(children.iter().rev().fold(root, |mut path, child| {
                path.push(child);
                path
            }))
        }),
    }
    .map(|path_with_pos| path_with_pos.to_string(&|path| path.to_string_lossy().into_owned()))
}

fn expand_directory_diff_pairs(
    diff_pairs: Vec<[String; 2]>,
) -> anyhow::Result<(Vec<[String; 2]>, Vec<TempDir>)> {
    let mut expanded = Vec::new();
    let mut temp_dirs = Vec::new();

    for pair in diff_pairs {
        let left = PathBuf::from(&pair[0]);
        let right = PathBuf::from(&pair[1]);

        if left.is_dir() && right.is_dir() {
            let (mut pairs, temp_dir) = expand_directory_pair(&left, &right)?;
            expanded.append(&mut pairs);
            if let Some(temp_dir) = temp_dir {
                temp_dirs.push(temp_dir);
            }
        } else {
            expanded.push(pair);
        }
    }

    Ok((expanded, temp_dirs))
}

fn expand_directory_pair(
    left: &Path,
    right: &Path,
) -> anyhow::Result<(Vec<[String; 2]>, Option<TempDir>)> {
    let left_files = collect_files(left)?;
    let right_files = collect_files(right)?;

    let mut rel_paths = BTreeSet::new();
    rel_paths.extend(left_files.keys().cloned());
    rel_paths.extend(right_files.keys().cloned());

    let mut temp_dir = TempDir::new()?;
    let mut temp_dir_used = false;
    let mut pairs = Vec::new();

    for rel in rel_paths {
        match (left_files.get(&rel), right_files.get(&rel)) {
            (Some(left_path), Some(right_path)) => {
                pairs.push([
                    left_path.to_string_lossy().into_owned(),
                    right_path.to_string_lossy().into_owned(),
                ]);
            }
            (Some(left_path), None) => {
                let stub = create_empty_stub(&mut temp_dir, &rel)?;
                temp_dir_used = true;
                pairs.push([
                    left_path.to_string_lossy().into_owned(),
                    stub.to_string_lossy().into_owned(),
                ]);
            }
            (None, Some(right_path)) => {
                let stub = create_empty_stub(&mut temp_dir, &rel)?;
                temp_dir_used = true;
                pairs.push([
                    stub.to_string_lossy().into_owned(),
                    right_path.to_string_lossy().into_owned(),
                ]);
            }
            (None, None) => {}
        }
    }

    let temp_dir = if temp_dir_used { Some(temp_dir) } else { None };
    Ok((pairs, temp_dir))
}

fn collect_files(root: &Path) -> anyhow::Result<BTreeMap<PathBuf, PathBuf>> {
    let mut files = BTreeMap::new();

    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(root)
                .context("stripping directory prefix")?
                .to_path_buf();
            files.insert(rel, entry.into_path());
        }
    }

    Ok(files)
}

fn create_empty_stub(temp_dir: &mut TempDir, rel: &Path) -> anyhow::Result<PathBuf> {
    let stub_path = temp_dir.path().join(rel);
    if let Some(parent) = stub_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::File::create(&stub_path)?;
    Ok(stub_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use util::path;
    use util::paths::SanitizedPath;
    use util::test::TempTree;

    macro_rules! assert_path_eq {
        ($left:expr, $right:expr) => {
            assert_eq!(
                SanitizedPath::new(Path::new(&$left)),
                SanitizedPath::new(Path::new(&$right))
            )
        };
    }

    fn cwd() -> PathBuf {
        env::current_dir().unwrap()
    }

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn with_cwd<T>(path: &Path, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
        let _lock = CWD_LOCK.lock();
        let old_cwd = cwd();
        env::set_current_dir(path)?;
        let result = f();
        env::set_current_dir(old_cwd)?;
        result
    }

    #[test]
    fn test_parse_non_existing_path() {
        // Absolute path
        let result = parse_path_with_position(path!("/non/existing/path.txt")).unwrap();
        assert_path_eq!(result, path!("/non/existing/path.txt"));

        // Absolute path in cwd
        let path = cwd().join(path!("non/existing/path.txt"));
        let expected = path.to_string_lossy().to_string();
        let result = parse_path_with_position(&expected).unwrap();
        assert_path_eq!(result, expected);

        // Relative path
        let result = parse_path_with_position(path!("non/existing/path.txt")).unwrap();
        assert_path_eq!(result, expected)
    }

    #[test]
    fn test_parse_existing_path() {
        let temp_tree = TempTree::new(json!({
            "file.txt": "",
        }));
        let file_path = temp_tree.path().join("file.txt");
        let expected = file_path.to_string_lossy().to_string();

        // Absolute path
        let result = parse_path_with_position(file_path.to_str().unwrap()).unwrap();
        assert_path_eq!(result, expected);

        // Relative path
        let result = with_cwd(temp_tree.path(), || parse_path_with_position("file.txt")).unwrap();
        assert_path_eq!(result, expected);
    }

    // NOTE:
    // While POSIX symbolic links are somewhat supported on Windows, they are an opt in by the user, and thus
    // we assume that they are not supported out of the box.
    #[cfg(not(windows))]
    #[test]
    fn test_parse_symlink_file() {
        let temp_tree = TempTree::new(json!({
            "target.txt": "",
        }));
        let target_path = temp_tree.path().join("target.txt");
        let symlink_path = temp_tree.path().join("symlink.txt");
        std::os::unix::fs::symlink(&target_path, &symlink_path).unwrap();

        // Absolute path
        let result = parse_path_with_position(symlink_path.to_str().unwrap()).unwrap();
        assert_eq!(result, target_path.to_string_lossy());

        // Relative path
        let result =
            with_cwd(temp_tree.path(), || parse_path_with_position("symlink.txt")).unwrap();
        assert_eq!(result, target_path.to_string_lossy());
    }

    #[cfg(not(windows))]
    #[test]
    fn test_parse_symlink_dir() {
        let temp_tree = TempTree::new(json!({
            "some": {
                "dir": { // symlink target
                    "ec": {
                        "tory": {
                            "file.txt": "",
        }}}}}));

        let target_file_path = temp_tree.path().join("some/dir/ec/tory/file.txt");
        let expected = target_file_path.to_string_lossy();

        let dir_path = temp_tree.path().join("some/dir");
        let symlink_path = temp_tree.path().join("symlink");
        std::os::unix::fs::symlink(&dir_path, &symlink_path).unwrap();

        // Absolute path
        let result =
            parse_path_with_position(symlink_path.join("ec/tory/file.txt").to_str().unwrap())
                .unwrap();
        assert_eq!(result, expected);

        // Relative path
        let result = with_cwd(temp_tree.path(), || {
            parse_path_with_position("symlink/ec/tory/file.txt")
        })
        .unwrap();
        assert_eq!(result, expected);
    }
}

fn parse_path_in_wsl(source: &str, wsl: &str) -> Result<String> {
    let mut source = PathWithPosition::parse_str(source);

    let (user, distro_name) = if let Some((user, distro)) = wsl.split_once('@') {
        if user.is_empty() {
            anyhow::bail!("user is empty in wsl argument");
        }
        (Some(user), distro)
    } else {
        (None, wsl)
    };

    let mut args = vec!["--distribution", distro_name];
    if let Some(user) = user {
        args.push("--user");
        args.push(user);
    }

    let command = [
        OsStr::new("realpath"),
        OsStr::new("-s"),
        source.path.as_ref(),
    ];

    let output = util::command::new_std_command("wsl.exe")
        .args(&args)
        .arg("--exec")
        .args(&command)
        .output()?;
    let result = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        let fallback = util::command::new_std_command("wsl.exe")
            .args(&args)
            .arg("--")
            .args(&command)
            .output()?;
        String::from_utf8_lossy(&fallback.stdout).to_string()
    };

    source.path = Path::new(result.trim()).to_owned();

    Ok(source.to_string(&|path| path.to_string_lossy().into_owned()))
}

fn main() -> Result<()> {
    // Claude Code hook이 cli를 백그라운드 실행(`&`)하고 bash/dispatch.sh가
    // 즉시 종료할 수 있으므로, 부모 프로세스 snapshot은 cli 진입 최상단에서
    // 바로 찍어야 한다. 늦게 호출하면 bash가 이미 exit 상태라 chain이
    // 2단계에서 끊긴다. clap 파싱·서버 생성 전 1회 수집 후 전역에 저장.
    //
    // 일반 파일 열기 경로(예: `dokkaebi-cli path/to/file`)에서는 ancestor
    // chain이 필요 없으므로 Toolhelp snapshot(O(전체 프로세스))을 건너뛴다.
    // notify 호출 여부는 clap 파싱 전에 argv 원본에서 `--notify-kind` 존재
    // 여부로만 저비용 판정한다.
    let initial_ancestors = if std::env::args_os().any(|a| a == "--notify-kind") {
        windows_ancestor_pids()
    } else {
        Vec::new()
    };

    #[cfg(unix)]
    util::prevent_root_execution();

    // Exit flatpak sandbox if needed
    #[cfg(target_os = "linux")]
    {
        flatpak::try_restart_to_host();
        flatpak::ld_extra_libs();
    }

    // Intercept version designators
    #[cfg(target_os = "macos")]
    if let Some(channel) = std::env::args().nth(1).filter(|arg| arg.starts_with("--")) {
        // When the first argument is a name of a release channel, we're going to spawn off the CLI of that version, with trailing args passed along.
        use std::str::FromStr as _;

        if let Ok(channel) = release_channel::ReleaseChannel::from_str(&channel[2..]) {
            return mac_os::spawn_channel_cli(channel, std::env::args().skip(2).collect());
        }
    }
    let args = Args::parse();

    // `zed --askpass` Makes zed operate in nc/netcat mode for use with askpass
    if let Some(socket) = &args.askpass {
        askpass::main(socket);
        return Ok(());
    }

    // 인스톨러 언인스톨 훅 전용. IPC/서버 초기화 없이 JSON 파일만 편집하고 즉시 종료.
    // 실패해도 언인스톨러 전체 실패로 번지지 않도록 exit code 만 구분한다.
    if args.uninstall_claude_plugin {
        return match claude_plugin_registry::remove_plugin_registration() {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("warning: failed to clean up Claude Code plugin registration: {e}");
                std::process::exit(1);
            }
        };
    }

    // Set custom data directory before any path operations
    let user_data_dir = args.user_data_dir.clone();
    if let Some(dir) = &user_data_dir {
        paths::set_custom_data_dir(dir);
    }

    #[cfg(target_os = "linux")]
    let args = flatpak::set_bin_if_no_escape(args);

    let app = Detect::detect(args.zed.as_deref()).context("Bundle detection")?;

    if args.version {
        println!("{}", app.zed_version_string());
        return Ok(());
    }

    if args.system_specs {
        let path = app.path();
        let msg = [
            "The `--system-specs` argument is not supported in the Zed CLI, only on Zed binary.",
            "To retrieve the system specs on the command line, run the following command:",
            &format!("{} --system-specs", path.display()),
        ];
        anyhow::bail!(msg.join("\n"));
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "macos"),
        not(feature = "no-bundled-uninstall")
    ))]
    if args.uninstall {
        static UNINSTALL_SCRIPT: &[u8] = include_bytes!("../../../script/uninstall.sh");

        let tmp_dir = tempfile::tempdir()?;
        let script_path = tmp_dir.path().join("uninstall.sh");
        fs::write(&script_path, UNINSTALL_SCRIPT)?;

        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;

        let status = std::process::Command::new("sh")
            .arg(&script_path)
            .env("ZED_CHANNEL", &*release_channel::RELEASE_CHANNEL_NAME)
            .status()
            .context("Failed to execute uninstall script")?;

        std::process::exit(status.code().unwrap_or(1));
    }

    let (server, server_name) =
        IpcOneShotServer::<IpcHandshake>::new().context("Handshake before Zed spawn")?;
    let url = format!("zed-cli://{server_name}");

    let open_new_workspace = if args.new {
        Some(true)
    } else if args.add {
        Some(false)
    } else {
        None
    };

    let env = {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            use collections::HashMap;

            // On Linux, the desktop entry uses `cli` to spawn `zed`.
            // We need to handle env vars correctly since std::env::vars() may not contain
            // project-specific vars (e.g. those set by direnv).
            // By setting env to None here, the LSP will use worktree env vars instead,
            // which is what we want.
            if !std::io::stdout().is_terminal() {
                None
            } else {
                Some(std::env::vars().collect::<HashMap<_, _>>())
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, by default, a child process inherits a copy of the environment block of the parent process.
            // So we don't need to pass env vars explicitly.
            None
        }

        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "windows")))]
        {
            use collections::HashMap;

            Some(std::env::vars().collect::<HashMap<_, _>>())
        }
    };

    let exit_status = Arc::new(Mutex::new(None));
    let mut paths = vec![];
    let mut urls = vec![];
    let mut diff_paths = vec![];
    let mut stdin_tmp_file: Option<fs::File> = None;
    let mut anonymous_fd_tmp_files = vec![];

    // Check if any diff paths are directories to determine diff_all mode
    let diff_all_mode = args
        .diff
        .chunks(2)
        .any(|pair| Path::new(&pair[0]).is_dir() || Path::new(&pair[1]).is_dir());

    for path in args.diff.chunks(2) {
        diff_paths.push([
            parse_path_with_position(&path[0])?,
            parse_path_with_position(&path[1])?,
        ]);
    }

    let (expanded_diff_paths, temp_dirs) = expand_directory_diff_pairs(diff_paths)?;
    diff_paths = expanded_diff_paths;
    // Prevent automatic cleanup of temp directories containing empty stub files
    // for directory diffs. The CLI process may exit before Zed has read these
    // files (e.g., when RPC-ing into an already-running instance). The files
    // live in the OS temp directory and will be cleaned up on reboot.
    for temp_dir in temp_dirs {
        let _ = temp_dir.keep();
    }

    #[cfg(target_os = "windows")]
    let wsl = args.wsl.as_ref();
    #[cfg(not(target_os = "windows"))]
    let wsl = None;

    for path in args.paths_with_position.iter() {
        if URL_PREFIX.iter().any(|&prefix| path.starts_with(prefix)) {
            urls.push(path.to_string());
        } else if path == "-" && args.paths_with_position.len() == 1 {
            let file = NamedTempFile::new()?;
            paths.push(file.path().to_string_lossy().into_owned());
            let (file, _) = file.keep()?;
            stdin_tmp_file = Some(file);
        } else if let Some(file) = anonymous_fd(path) {
            let tmp_file = NamedTempFile::new()?;
            paths.push(tmp_file.path().to_string_lossy().into_owned());
            let (tmp_file, _) = tmp_file.keep()?;
            anonymous_fd_tmp_files.push((file, tmp_file));
        } else if let Some(wsl) = wsl {
            urls.push(format!("file://{}", parse_path_in_wsl(path, wsl)?));
        } else {
            paths.push(parse_path_with_position(path)?);
        }
    }

    // When only diff paths are provided (no regular paths), add the current
    // working directory so the workspace opens with the right context.
    if paths.is_empty() && urls.is_empty() && !diff_paths.is_empty() {
        if let Ok(cwd) = env::current_dir() {
            paths.push(cwd.to_string_lossy().into_owned());
        }
    }

    anyhow::ensure!(
        args.dev_server_token.is_none(),
        "Dev servers were removed in v0.157.x please upgrade to SSH remoting: https://zed.dev/docs/remote-development"
    );

    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .stack_size(10 * 1024 * 1024)
        .thread_name(|ix| format!("RayonWorker{}", ix))
        .build_global()
        .unwrap();

    let sender: JoinHandle<anyhow::Result<()>> = thread::Builder::new()
        .name("CliReceiver".to_string())
        .spawn({
            let exit_status = exit_status.clone();
            let user_data_dir_for_thread = user_data_dir.clone();
            let initial_ancestors = initial_ancestors.clone();
            move || {
                let (_, handshake) = server.accept().context("Handshake after Zed spawn")?;
                let (tx, rx) = (handshake.requests, handshake.responses);

                #[cfg(target_os = "windows")]
                let wsl = args.wsl;
                #[cfg(not(target_os = "windows"))]
                let wsl = None;

                // Claude Code 플러그인 알림 모드: 다른 인자(paths/wait/diff 등)를 무시하고
                // Notify IPC만 송신한 뒤 응답(Exit) 대기.
                let request = if let Some(kind_str) = args.notify_kind.as_deref() {
                    let kind = match kind_str {
                        "stop" => NotifyKind::Stop,
                        "idle" => NotifyKind::Idle,
                        "permission" => NotifyKind::Permission,
                        "subagent-start" => NotifyKind::SubagentStart,
                        "subagent-stop" => NotifyKind::SubagentStop,
                        other => anyhow::bail!(
                            "invalid --notify-kind value '{other}', expected one of: stop|idle|permission|subagent-start|subagent-stop"
                        ),
                    };
                    // main 진입 최상단에서 찍은 snapshot 재사용.
                    let ancestors = initial_ancestors.clone();
                    let pid = ancestors.get(1).copied().or(args.notify_pid);
                    // Subagent payload 는 SubagentStart/Stop 때만 의미 있지만, Args 는
                    // 훅 진입 시점에 어떤 kind 인지 구분 없이 플래그를 채워 오므로 kind 로
                    // 분기해 채운다. 그 외 토스트 경로는 None.
                    let subagent = match kind {
                        NotifyKind::SubagentStart | NotifyKind::SubagentStop => {
                            Some(SubagentPayload {
                                session_id: args.notify_session_id,
                                transcript_path: args.notify_transcript_path,
                                subagent_id: args.notify_subagent_id,
                                subagent_type: args.notify_subagent_type,
                                description: args.notify_subagent_description,
                                prompt: args.notify_subagent_prompt,
                                result: args.notify_subagent_result,
                            })
                        }
                        _ => None,
                    };
                    CliRequest::Notify {
                        kind,
                        cwd: args.notify_cwd,
                        pid,
                        ancestors,
                        notify_prompt: args.notify_prompt,
                        notify_response: args.notify_response,
                        notify_tool_name: args.notify_tool_name,
                        notify_tool_preview: args.notify_tool_preview,
                        notify_idle_summary: args.notify_idle_summary,
                        subagent,
                    }
                } else {
                    CliRequest::Open {
                        paths,
                        urls,
                        diff_paths,
                        diff_all: diff_all_mode,
                        wsl,
                        wait: args.wait,
                        open_new_workspace,
                        reuse: args.reuse,
                        env,
                        user_data_dir: user_data_dir_for_thread,
                    }
                };
                tx.send(request)?;

                while let Ok(response) = rx.recv() {
                    match response {
                        CliResponse::Ping => {}
                        CliResponse::Stdout { message } => println!("{message}"),
                        CliResponse::Stderr { message } => eprintln!("{message}"),
                        CliResponse::Exit { status } => {
                            exit_status.lock().replace(status);
                            return Ok(());
                        }
                    }
                }

                Ok(())
            }
        })
        .unwrap();

    let stdin_pipe_handle: Option<JoinHandle<anyhow::Result<()>>> =
        stdin_tmp_file.map(|mut tmp_file| {
            thread::Builder::new()
                .name("CliStdin".to_string())
                .spawn(move || {
                    let mut stdin = std::io::stdin().lock();
                    if !io::IsTerminal::is_terminal(&stdin) {
                        io::copy(&mut stdin, &mut tmp_file)?;
                    }
                    Ok(())
                })
                .unwrap()
        });

    let anonymous_fd_pipe_handles: Vec<_> = anonymous_fd_tmp_files
        .into_iter()
        .map(|(mut file, mut tmp_file)| {
            thread::Builder::new()
                .name("CliAnonymousFd".to_string())
                .spawn(move || io::copy(&mut file, &mut tmp_file))
                .unwrap()
        })
        .collect();

    if args.foreground {
        app.run_foreground(url, user_data_dir.as_deref())?;
    } else {
        app.launch(url, user_data_dir.as_deref())?;

        // handshake 워치독: `HANDSHAKE_TIMEOUT` 내 sender 스레드가 완료하지
        // 않으면 UI 초기화에 실패한 좀비 본체로 간주해 spawn된 Child를 kill
        // 하고 프로세스 종료. pipe 경로(기존 본체 재사용)에서는 Child가 없어
        // 경고 로그만 남기고 그대로 실패 종료한다.
        let sender_done = Arc::new(AtomicBool::new(false));
        spawn_handshake_watchdog(sender_done.clone());

        let sender_result = sender.join().unwrap();
        sender_done.store(true, Ordering::SeqCst);
        sender_result?;
        if let Some(handle) = stdin_pipe_handle {
            handle.join().unwrap()?;
        }
        for handle in anonymous_fd_pipe_handles {
            handle.join().unwrap()?;
        }
    }

    if let Some(exit_status) = exit_status.lock().take() {
        std::process::exit(exit_status);
    }
    Ok(())
}

fn anonymous_fd(path: &str) -> Option<fs::File> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::{self, FromRawFd};

        let fd_str = path.strip_prefix("/proc/self/fd/")?;

        let link = fs::read_link(path).ok()?;
        if !link.starts_with("memfd:") {
            return None;
        }

        let fd: fd::RawFd = fd_str.parse().ok()?;
        let file = unsafe { fs::File::from_raw_fd(fd) };
        Some(file)
    }
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        use std::os::{
            fd::{self, FromRawFd},
            unix::fs::FileTypeExt,
        };

        let fd_str = path.strip_prefix("/dev/fd/")?;

        let metadata = fs::metadata(path).ok()?;
        let file_type = metadata.file_type();
        if !file_type.is_fifo() && !file_type.is_socket() {
            return None;
        }
        let fd: fd::RawFd = fd_str.parse().ok()?;
        let file = unsafe { fs::File::from_raw_fd(fd) };
        Some(file)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
    {
        _ = path;
        // not implemented for bsd, windows. Could be, but isn't yet
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux {
    use std::{
        env,
        ffi::OsString,
        io,
        os::unix::net::{SocketAddr, UnixDatagram},
        path::{Path, PathBuf},
        process::{self, ExitStatus},
        thread,
        time::Duration,
    };

    use anyhow::{Context as _, anyhow};
    use cli::FORCE_CLI_MODE_ENV_VAR_NAME;
    use fork::Fork;

    use crate::{Detect, InstalledApp};

    struct App(PathBuf);

    impl Detect {
        pub fn detect(path: Option<&Path>) -> anyhow::Result<impl InstalledApp> {
            let path = if let Some(path) = path {
                path.to_path_buf().canonicalize()?
            } else {
                let cli = env::current_exe()?;
                let dir = cli.parent().context("no parent path for cli")?;

                // libexec is the standard, lib/zed is for Arch (and other non-libexec distros),
                // ./zed is for the target directory in development builds.
                let possible_locations =
                    ["../libexec/zed-editor", "../lib/zed/zed-editor", "./zed"];
                possible_locations
                    .iter()
                    .find_map(|p| dir.join(p).canonicalize().ok().filter(|path| path != &cli))
                    .with_context(|| {
                        format!("could not find any of: {}", possible_locations.join(", "))
                    })?
            };

            Ok(App(path))
        }
    }

    impl InstalledApp for App {
        fn zed_version_string(&self) -> String {
            format!(
                "Zed {}{}{} – {}",
                if *release_channel::RELEASE_CHANNEL_NAME == "stable" {
                    "".to_string()
                } else {
                    format!("{} ", *release_channel::RELEASE_CHANNEL_NAME)
                },
                option_env!("RELEASE_VERSION").unwrap_or_default(),
                match option_env!("ZED_COMMIT_SHA") {
                    Some(commit_sha) => format!(" {commit_sha} "),
                    None => "".to_string(),
                },
                self.0.display(),
            )
        }

        fn launch(&self, ipc_url: String, user_data_dir: Option<&str>) -> anyhow::Result<()> {
            let data_dir = user_data_dir
                .map(PathBuf::from)
                .unwrap_or_else(|| paths::data_dir().clone());

            let sock_path = data_dir.join(format!(
                "zed-{}.sock",
                *release_channel::RELEASE_CHANNEL_NAME
            ));
            let sock = UnixDatagram::unbound()?;
            if sock.connect(&sock_path).is_err() {
                self.boot_background(ipc_url, user_data_dir)?;
            } else {
                sock.send(ipc_url.as_bytes())?;
            }
            Ok(())
        }

        fn run_foreground(
            &self,
            ipc_url: String,
            user_data_dir: Option<&str>,
        ) -> io::Result<ExitStatus> {
            let mut cmd = std::process::Command::new(self.0.clone());
            cmd.arg(ipc_url);
            if let Some(dir) = user_data_dir {
                cmd.arg("--user-data-dir").arg(dir);
            }
            cmd.status()
        }

        fn path(&self) -> PathBuf {
            self.0.clone()
        }
    }

    impl App {
        fn boot_background(
            &self,
            ipc_url: String,
            user_data_dir: Option<&str>,
        ) -> anyhow::Result<()> {
            let path = &self.0;

            match fork::fork() {
                Ok(Fork::Parent(_)) => Ok(()),
                Ok(Fork::Child) => {
                    unsafe { std::env::set_var(FORCE_CLI_MODE_ENV_VAR_NAME, "") };
                    if fork::setsid().is_err() {
                        eprintln!("failed to setsid: {}", std::io::Error::last_os_error());
                        process::exit(1);
                    }
                    if fork::close_fd().is_err() {
                        eprintln!("failed to close_fd: {}", std::io::Error::last_os_error());
                    }
                    let mut args: Vec<OsString> =
                        vec![path.as_os_str().to_owned(), OsString::from(ipc_url)];
                    if let Some(dir) = user_data_dir {
                        args.push(OsString::from("--user-data-dir"));
                        args.push(OsString::from(dir));
                    }
                    let error = exec::execvp(path.clone(), &args);
                    // if exec succeeded, we never get here.
                    eprintln!("failed to exec {:?}: {}", path, error);
                    process::exit(1)
                }
                Err(_) => Err(anyhow!(io::Error::last_os_error())),
            }
        }

        fn wait_for_socket(
            &self,
            sock_addr: &SocketAddr,
            sock: &mut UnixDatagram,
        ) -> Result<(), std::io::Error> {
            for _ in 0..100 {
                thread::sleep(Duration::from_millis(10));
                if sock.connect_addr(sock_addr).is_ok() {
                    return Ok(());
                }
            }
            sock.connect_addr(sock_addr)
        }
    }
}

#[cfg(target_os = "linux")]
mod flatpak {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::process::Command;
    use std::{env, process};

    const EXTRA_LIB_ENV_NAME: &str = "ZED_FLATPAK_LIB_PATH";
    const NO_ESCAPE_ENV_NAME: &str = "ZED_FLATPAK_NO_ESCAPE";

    /// Adds bundled libraries to LD_LIBRARY_PATH if running under flatpak
    pub fn ld_extra_libs() {
        let mut paths = if let Ok(paths) = env::var("LD_LIBRARY_PATH") {
            env::split_paths(&paths).collect()
        } else {
            Vec::new()
        };

        if let Ok(extra_path) = env::var(EXTRA_LIB_ENV_NAME) {
            paths.push(extra_path.into());
        }

        unsafe { env::set_var("LD_LIBRARY_PATH", env::join_paths(paths).unwrap()) };
    }

    /// Restarts outside of the sandbox if currently running within it
    pub fn try_restart_to_host() {
        if let Some(flatpak_dir) = get_flatpak_dir() {
            let mut args = vec!["/usr/bin/flatpak-spawn".into(), "--host".into()];
            args.append(&mut get_xdg_env_args());
            args.push("--env=ZED_UPDATE_EXPLANATION=Please use flatpak to update zed".into());
            args.push(
                format!(
                    "--env={EXTRA_LIB_ENV_NAME}={}",
                    flatpak_dir.join("lib").to_str().unwrap()
                )
                .into(),
            );
            args.push(flatpak_dir.join("bin").join("zed").into());

            let mut is_app_location_set = false;
            for arg in &env::args_os().collect::<Vec<_>>()[1..] {
                args.push(arg.clone());
                is_app_location_set |= arg == "--zed";
            }

            if !is_app_location_set {
                args.push("--zed".into());
                args.push(flatpak_dir.join("libexec").join("zed-editor").into());
            }

            let error = exec::execvp("/usr/bin/flatpak-spawn", args);
            eprintln!("failed restart cli on host: {:?}", error);
            process::exit(1);
        }
    }

    pub fn set_bin_if_no_escape(mut args: super::Args) -> super::Args {
        if env::var(NO_ESCAPE_ENV_NAME).is_ok()
            && env::var("FLATPAK_ID").is_ok_and(|id| id.starts_with("dev.zed.Zed"))
            && args.zed.is_none()
        {
            args.zed = Some("/app/libexec/zed-editor".into());
            unsafe { env::set_var("ZED_UPDATE_EXPLANATION", "Please use flatpak to update zed") };
        }
        args
    }

    fn get_flatpak_dir() -> Option<PathBuf> {
        if env::var(NO_ESCAPE_ENV_NAME).is_ok() {
            return None;
        }

        if let Ok(flatpak_id) = env::var("FLATPAK_ID") {
            if !flatpak_id.starts_with("dev.zed.Zed") {
                return None;
            }

            let install_dir = Command::new("/usr/bin/flatpak-spawn")
                .arg("--host")
                .arg("flatpak")
                .arg("info")
                .arg("--show-location")
                .arg(flatpak_id)
                .output()
                .unwrap();
            let install_dir = PathBuf::from(String::from_utf8(install_dir.stdout).unwrap().trim());
            Some(install_dir.join("files"))
        } else {
            None
        }
    }

    fn get_xdg_env_args() -> Vec<OsString> {
        let xdg_keys = [
            "XDG_DATA_HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
        ];
        env::vars()
            .filter(|(key, _)| xdg_keys.contains(&key.as_str()))
            .map(|(key, val)| format!("--env=FLATPAK_{}={}", key, val).into())
            .collect()
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use anyhow::Context;
    use release_channel::app_identifier;
    use windows::{
        Win32::{
            Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GENERIC_WRITE, GetLastError},
            Storage::FileSystem::{
                CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE, OPEN_EXISTING, WriteFile,
            },
            System::Threading::CreateMutexW,
        },
        core::HSTRING,
    };

    use crate::{Detect, InstalledApp};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process::ExitStatus;

    fn check_single_instance() -> bool {
        let mutex = unsafe {
            CreateMutexW(
                None,
                false,
                &HSTRING::from(format!("{}-Instance-Mutex", app_identifier())),
            )
            .expect("Unable to create instance sync event")
        };
        let last_err = unsafe { GetLastError() };
        let _ = unsafe { CloseHandle(mutex) };
        last_err != ERROR_ALREADY_EXISTS
    }

    struct App(PathBuf);

    impl InstalledApp for App {
        fn zed_version_string(&self) -> String {
            format!(
                "Zed {}{}{} – {}",
                if *release_channel::RELEASE_CHANNEL_NAME == "stable" {
                    "".to_string()
                } else {
                    format!("{} ", *release_channel::RELEASE_CHANNEL_NAME)
                },
                option_env!("RELEASE_VERSION").unwrap_or_default(),
                match option_env!("ZED_COMMIT_SHA") {
                    Some(commit_sha) => format!(" {commit_sha} "),
                    None => "".to_string(),
                },
                self.0.display(),
            )
        }

        fn launch(&self, ipc_url: String, user_data_dir: Option<&str>) -> anyhow::Result<()> {
            if check_single_instance() {
                let mut cmd = std::process::Command::new(self.0.clone());
                cmd.arg(ipc_url);
                if let Some(dir) = user_data_dir {
                    cmd.arg("--user-data-dir").arg(dir);
                }
                // spawn된 Child를 전역 슬롯에 보관. 워치독이 handshake 타임아웃
                // 시 이 Child를 kill해 UI 초기화 실패 좀비가 남지 않게 한다.
                let child = cmd.spawn()?;
                crate::spawned_child_slot().lock().replace(child);
            } else {
                unsafe {
                    let pipe = CreateFileW(
                        &HSTRING::from(format!("\\\\.\\pipe\\{}-Named-Pipe", app_identifier())),
                        GENERIC_WRITE.0,
                        FILE_SHARE_MODE::default(),
                        None,
                        OPEN_EXISTING,
                        FILE_FLAGS_AND_ATTRIBUTES::default(),
                        None,
                    )?;
                    let message = ipc_url.as_bytes();
                    let mut bytes_written = 0;
                    WriteFile(pipe, Some(message), Some(&mut bytes_written), None)?;
                    CloseHandle(pipe)?;
                }
            }
            Ok(())
        }

        fn run_foreground(
            &self,
            ipc_url: String,
            user_data_dir: Option<&str>,
        ) -> io::Result<ExitStatus> {
            let mut cmd = std::process::Command::new(self.0.clone());
            cmd.arg(ipc_url).arg("--foreground");
            if let Some(dir) = user_data_dir {
                cmd.arg("--user-data-dir").arg(dir);
            }
            cmd.spawn()?.wait()
        }

        fn path(&self) -> PathBuf {
            self.0.clone()
        }
    }

    impl Detect {
        pub fn detect(path: Option<&Path>) -> anyhow::Result<impl InstalledApp> {
            let path = if let Some(path) = path {
                path.to_path_buf().canonicalize()?
            } else {
                let cli = std::env::current_exe()?;
                let dir = cli.parent().context("no parent path for cli")?;

                // ../Dokkaebi.exe is the standard, lib/zed is for MSYS2, ./dokkaebi.exe is for the target
                // directory in development builds.
                let possible_locations = ["../Dokkaebi.exe", "../lib/zed/zed-editor.exe", "./dokkaebi.exe"];
                possible_locations
                    .iter()
                    .find_map(|p| dir.join(p).canonicalize().ok().filter(|path| path != &cli))
                    .context(format!(
                        "could not find any of: {}",
                        possible_locations.join(", ")
                    ))?
            };

            Ok(App(path))
        }
    }
}

#[cfg(target_os = "macos")]
mod mac_os {
    use anyhow::{Context as _, Result};
    use core_foundation::{
        array::{CFArray, CFIndex},
        base::TCFType as _,
        string::kCFStringEncodingUTF8,
        url::{CFURL, CFURLCreateWithBytes},
    };
    use core_services::{LSLaunchURLSpec, LSOpenFromURLSpec, kLSLaunchDefaults};
    use serde::Deserialize;
    use std::{
        ffi::OsStr,
        fs, io,
        path::{Path, PathBuf},
        process::{Command, ExitStatus},
        ptr,
    };

    use cli::FORCE_CLI_MODE_ENV_VAR_NAME;

    use crate::{Detect, InstalledApp};

    #[derive(Debug, Deserialize)]
    struct InfoPlist {
        #[serde(rename = "CFBundleShortVersionString")]
        bundle_short_version_string: String,
    }

    enum Bundle {
        App {
            app_bundle: PathBuf,
            plist: InfoPlist,
        },
        LocalPath {
            executable: PathBuf,
        },
    }

    fn locate_bundle() -> Result<PathBuf> {
        let cli_path = std::env::current_exe()?.canonicalize()?;
        let mut app_path = cli_path.clone();
        while app_path.extension() != Some(OsStr::new("app")) {
            anyhow::ensure!(
                app_path.pop(),
                "cannot find app bundle containing {cli_path:?}"
            );
        }
        Ok(app_path)
    }

    impl Detect {
        pub fn detect(path: Option<&Path>) -> anyhow::Result<impl InstalledApp> {
            let bundle_path = if let Some(bundle_path) = path {
                bundle_path
                    .canonicalize()
                    .with_context(|| format!("Args bundle path {bundle_path:?} canonicalization"))?
            } else {
                locate_bundle().context("bundle autodiscovery")?
            };

            match bundle_path.extension().and_then(|ext| ext.to_str()) {
                Some("app") => {
                    let plist_path = bundle_path.join("Contents/Info.plist");
                    let plist =
                        plist::from_file::<_, InfoPlist>(&plist_path).with_context(|| {
                            format!("Reading *.app bundle plist file at {plist_path:?}")
                        })?;
                    Ok(Bundle::App {
                        app_bundle: bundle_path,
                        plist,
                    })
                }
                _ => Ok(Bundle::LocalPath {
                    executable: bundle_path,
                }),
            }
        }
    }

    impl InstalledApp for Bundle {
        fn zed_version_string(&self) -> String {
            format!("Zed {} – {}", self.version(), self.path().display(),)
        }

        fn launch(&self, url: String, user_data_dir: Option<&str>) -> anyhow::Result<()> {
            match self {
                Self::App { app_bundle, .. } => {
                    let app_path = app_bundle;

                    let status = unsafe {
                        let app_url = CFURL::from_path(app_path, true)
                            .with_context(|| format!("invalid app path {app_path:?}"))?;
                        let url_to_open = CFURL::wrap_under_create_rule(CFURLCreateWithBytes(
                            ptr::null(),
                            url.as_ptr(),
                            url.len() as CFIndex,
                            kCFStringEncodingUTF8,
                            ptr::null(),
                        ));
                        // equivalent to: open zed-cli:... -a /Applications/Zed\ Preview.app
                        let urls_to_open =
                            CFArray::from_copyable(&[url_to_open.as_concrete_TypeRef()]);
                        LSOpenFromURLSpec(
                            &LSLaunchURLSpec {
                                appURL: app_url.as_concrete_TypeRef(),
                                itemURLs: urls_to_open.as_concrete_TypeRef(),
                                passThruParams: ptr::null(),
                                launchFlags: kLSLaunchDefaults,
                                asyncRefCon: ptr::null_mut(),
                            },
                            ptr::null_mut(),
                        )
                    };

                    anyhow::ensure!(
                        status == 0,
                        "cannot start app bundle {}",
                        self.zed_version_string()
                    );
                }

                Self::LocalPath { executable, .. } => {
                    let executable_parent = executable
                        .parent()
                        .with_context(|| format!("Executable {executable:?} path has no parent"))?;
                    let subprocess_stdout_file = fs::File::create(
                        executable_parent.join("zed_dev.log"),
                    )
                    .with_context(|| format!("Log file creation in {executable_parent:?}"))?;
                    let subprocess_stdin_file =
                        subprocess_stdout_file.try_clone().with_context(|| {
                            format!("Cloning descriptor for file {subprocess_stdout_file:?}")
                        })?;
                    let mut command = std::process::Command::new(executable);
                    command.env(FORCE_CLI_MODE_ENV_VAR_NAME, "");
                    if let Some(dir) = user_data_dir {
                        command.arg("--user-data-dir").arg(dir);
                    }
                    command
                        .stderr(subprocess_stdout_file)
                        .stdout(subprocess_stdin_file)
                        .arg(url);

                    command
                        .spawn()
                        .with_context(|| format!("Spawning {command:?}"))?;
                }
            }

            Ok(())
        }

        fn run_foreground(
            &self,
            ipc_url: String,
            user_data_dir: Option<&str>,
        ) -> io::Result<ExitStatus> {
            let path = match self {
                Bundle::App { app_bundle, .. } => app_bundle.join("Contents/MacOS/zed"),
                Bundle::LocalPath { executable, .. } => executable.clone(),
            };

            let mut cmd = std::process::Command::new(path);
            cmd.arg(ipc_url);
            if let Some(dir) = user_data_dir {
                cmd.arg("--user-data-dir").arg(dir);
            }
            cmd.status()
        }

        fn path(&self) -> PathBuf {
            match self {
                Bundle::App { app_bundle, .. } => app_bundle.join("Contents/MacOS/zed"),
                Bundle::LocalPath { executable, .. } => executable.clone(),
            }
        }
    }

    impl Bundle {
        fn version(&self) -> String {
            match self {
                Self::App { plist, .. } => plist.bundle_short_version_string.clone(),
                Self::LocalPath { .. } => "<development>".to_string(),
            }
        }

        fn path(&self) -> &Path {
            match self {
                Self::App { app_bundle, .. } => app_bundle,
                Self::LocalPath { executable, .. } => executable,
            }
        }
    }

    pub(super) fn spawn_channel_cli(
        channel: release_channel::ReleaseChannel,
        leftover_args: Vec<String>,
    ) -> Result<()> {
        use anyhow::bail;

        let app_path_prompt = format!(
            "POSIX path of (path to application \"{}\")",
            channel.display_name()
        );
        let app_path_output = Command::new("osascript")
            .arg("-e")
            .arg(&app_path_prompt)
            .output()?;
        if !app_path_output.status.success() {
            bail!(
                "Could not determine app path for {}",
                channel.display_name()
            );
        }
        let app_path = String::from_utf8(app_path_output.stdout)?.trim().to_owned();
        let cli_path = format!("{app_path}/Contents/MacOS/cli");
        Command::new(cli_path).args(leftover_args).spawn()?;
        Ok(())
    }
}
