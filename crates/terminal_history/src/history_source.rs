// 셸 히스토리 파일을 읽어서 명령어 목록을 반환하는 모듈

use std::collections::HashSet;
use std::path::PathBuf;
use util::shell::Shell;

/// 히스토리 항목 최대 개수
const MAX_HISTORY_ENTRIES: usize = 5000;

/// 단일 히스토리 항목
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub command: String,
}

/// 셸 프로그램 이름을 기반으로 히스토리 파일 경로를 결정한다.
pub fn history_file_path(shell: &Shell) -> Option<PathBuf> {
    let program = shell.program();
    let program_lower = program.to_lowercase();

    // 실행 파일 이름에서 경로/확장자 제거
    let name = std::path::Path::new(&program_lower)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&program_lower);

    match name {
        "bash" | "sh" => {
            // $HISTFILE 환경변수 우선, 없으면 ~/.bash_history
            if let Ok(histfile) = std::env::var("HISTFILE") {
                if !histfile.is_empty() {
                    return Some(PathBuf::from(histfile));
                }
            }
            dirs::home_dir().map(|h| h.join(".bash_history"))
        }
        "zsh" => {
            if let Ok(histfile) = std::env::var("HISTFILE") {
                if !histfile.is_empty() {
                    return Some(PathBuf::from(histfile));
                }
            }
            dirs::home_dir().map(|h| h.join(".zsh_history"))
        }
        "fish" => {
            dirs::data_local_dir().map(|d| d.join("fish").join("fish_history"))
        }
        "pwsh" | "powershell" => powershell_history_path(),
        "nu" | "nushell" => {
            dirs::config_dir().map(|c| c.join("nushell").join("history.txt"))
        }
        // cmd.exe 등은 영구 히스토리 파일이 없음
        _ => None,
    }
}

/// PowerShell/pwsh 히스토리 파일 경로 (PSReadLine)
fn powershell_history_path() -> Option<PathBuf> {
    if cfg!(windows) {
        // Windows: %APPDATA%\Microsoft\Windows\PowerShell\PSReadline\ConsoleHost_history.txt
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("PowerShell")
                .join("PSReadline")
                .join("ConsoleHost_history.txt")
        })
    } else {
        // Linux/macOS: ~/.local/share/powershell/PSReadLine/ConsoleHost_history.txt
        dirs::data_local_dir().map(|d| {
            d.join("powershell")
                .join("PSReadLine")
                .join("ConsoleHost_history.txt")
        })
    }
}

/// 히스토리 파일을 읽어서 중복 제거된 명령어 목록을 반환한다.
/// 최신 명령어가 먼저 오도록 역순 정렬된다.
pub fn load_history(shell: &Shell) -> Vec<HistoryEntry> {
    let path = match history_file_path(shell) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let content = match std::fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => return Vec::new(),
    };

    let program = shell.program();
    let name = std::path::Path::new(&program)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let commands: Vec<String> = match name.as_str() {
        "zsh" => parse_zsh_history(&content),
        "fish" => parse_fish_history(&content),
        _ => parse_plain_history(&content),
    };

    // 중복 제거 (최신 항목 유지): 역순 순회하여 처음 나타나는 것만 남김
    let mut seen = HashSet::new();
    let mut entries: Vec<HistoryEntry> = Vec::new();

    for cmd in commands.into_iter().rev() {
        let trimmed = cmd.trim().to_string();
        if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
            entries.push(HistoryEntry { command: trimmed });
            if entries.len() >= MAX_HISTORY_ENTRIES {
                break;
            }
        }
    }

    // entries는 이미 최신 순 (역순 순회했으므로)
    entries
}

/// 일반 히스토리 (bash, PowerShell 등): 한 줄에 한 명령어
fn parse_plain_history(content: &str) -> Vec<String> {
    content.lines().map(|l| l.to_string()).collect()
}

/// zsh 히스토리: `: 타임스탬프:0;명령어` 형식 또는 일반 문자열
fn parse_zsh_history(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| {
            // ": 1234567890:0;실제 명령어" 형식 처리
            if line.starts_with(": ") {
                if let Some(pos) = line.find(';') {
                    return line[pos + 1..].to_string();
                }
            }
            line.to_string()
        })
        .collect()
}

/// fish 히스토리: YAML 유사 형식에서 `- cmd:` 행 추출
fn parse_fish_history(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("- cmd:") {
                Some(trimmed.strip_prefix("- cmd:")?.trim().to_string())
            } else {
                None
            }
        })
        .collect()
}
