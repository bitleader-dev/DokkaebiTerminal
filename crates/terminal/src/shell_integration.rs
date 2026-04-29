//! Shell Integration (OSC 133 / FinalTerm) 바이트 스캐너 + 셸 자동 주입
//!
//! PTY 출력 바이트 스트림에서 OSC 133 시퀀스를 검출해 셸 명령어 단위 메타데이터를 emit 한다.
//! `alacritty_terminal::vte` 0.15.0 의 `osc_dispatch` 가 OSC 133 을 처리하지 않으므로
//! `pty_adapter` 의 read 스레드에서 본 스캐너가 alac 파서와 병행으로 바이트를 검사한다.
//!
//! 표준 시퀀스 (FinalTerm 공개 사양 — 저작권 보호 대상 아님):
//! ```text
//! ESC ] 133 ; A [;params] ST    프롬프트 시작
//! ESC ] 133 ; B [;params] ST    사용자 입력 영역 시작
//! ESC ] 133 ; C [;params] ST    명령 실행 시작
//! ESC ] 133 ; D [;exit_code[;...]] ST  명령 종료 (종료 코드 포함)
//! ```
//! - `ST` = `BEL` (0x07) 또는 `ESC \` (0x1B 0x5C)
//!
//! 또한 셸 종류 자동 감지 시(Auto 정책) PTY spawn 인자/환경에 셸 통합 스크립트를
//! 주입하는 헬퍼 함수를 제공한다. 사용자 rc/profile 파일은 변경하지 않고 임시 디렉터리에
//! 작성한 우리 스크립트를 `--rcfile` (bash) 또는 `-NoExit -Command` (pwsh) 로 로드한다.
//!
//! 라이선스: Warp/Kitty/VTE 코드 미참조. FinalTerm 공식 사양만 참조.
//! 본 파일과 동봉 스크립트는 Dokkaebi 자체 작성.

use std::path::PathBuf;
use std::str;
use std::sync::OnceLock;

use util::shell::ShellKind;

/// OSC 133 매칭 결과로 emit 되는 셸 통합 이벤트.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellIntegrationEvent {
    /// OSC 133;A — 프롬프트(PS1) 시작
    PromptStart,
    /// OSC 133;B — 사용자 입력 영역 시작
    CommandStart,
    /// OSC 133;C — 사용자가 입력을 마치고 명령이 실행되기 시작
    CommandExecuted,
    /// OSC 133;D — 명령 종료. 종료 코드가 포함된 경우 `exit_code` 보유.
    CommandFinished { exit_code: Option<i32> },
}

/// OSC 시퀀스 본문(`\x1b]` 이후 ST 이전)이 보관할 수 있는 최대 바이트.
/// 이를 초과하면 비정상 시퀀스로 간주하고 본문을 폐기한다.
const MAX_OSC_BODY: usize = 4096;

/// OSC 133 스캐너 상태.
///
/// VTE 의 OSC 상태 기계 중 OSC 133 검출에 필요한 최소 상태만 표현한다.
/// 다른 OSC 번호(0, 2, 8 등)는 본문 스킵 후 폐기한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// 일반 상태 — `0x1B` 검색
    Idle,
    /// `0x1B` 감지 — 다음 바이트가 `]`(0x5D) 인지 확인
    SawEsc,
    /// `0x1B ]` 감지 — 본문 누적 중. ST(BEL 또는 `0x1B \`) 까지 수집
    InOscBody,
    /// 본문 수집 중 `0x1B` 감지 — 다음이 `\\`(0x5C) 면 ST 종료, 아니면 본문 폐기
    SawEscInBody,
}

/// 바이트 스트림에서 OSC 133 시퀀스를 검출하는 상태 기계.
///
/// `feed()` 를 통해 청크 단위로 바이트를 공급하면 매칭된 이벤트 목록을 반환한다.
/// 시퀀스가 read 버퍼 경계에 걸쳐 split 되어도 내부 상태가 유지되므로 호출자는
/// 동일 인스턴스에 연속해서 `feed` 하면 된다.
pub struct Osc133Scanner {
    state: State,
    body: Vec<u8>,
}

impl Default for Osc133Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Osc133Scanner {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            body: Vec::new(),
        }
    }

    /// 바이트 청크를 공급한다. 매칭된 OSC 133 이벤트가 있으면 반환한다.
    /// 매칭되지 않은 다른 OSC 시퀀스는 내부에서 무해하게 폐기된다.
    /// 본 함수는 입력 바이트를 변경하지 않는다 — 호출자는 동일 바이트를
    /// alac VTE 파서에도 그대로 전달해야 한다.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<ShellIntegrationEvent> {
        let mut events = Vec::new();
        for &byte in bytes {
            self.feed_one(byte, &mut events);
        }
        events
    }

    fn feed_one(&mut self, byte: u8, events: &mut Vec<ShellIntegrationEvent>) {
        match self.state {
            State::Idle => {
                if byte == 0x1B {
                    self.state = State::SawEsc;
                }
            }
            State::SawEsc => {
                if byte == b']' {
                    // OSC 시퀀스 시작 — 본문 누적 시작
                    self.body.clear();
                    self.state = State::InOscBody;
                } else if byte == 0x1B {
                    // 연속된 ESC — 다음 바이트로 평가 지속
                    self.state = State::SawEsc;
                } else {
                    // ESC 뒤 ']' 가 아니면 OSC 가 아니므로 idle 복귀
                    self.state = State::Idle;
                }
            }
            State::InOscBody => match byte {
                // BEL — ST 종료
                0x07 => {
                    self.dispatch_body(events);
                    self.body.clear();
                    self.state = State::Idle;
                }
                // ESC — `\\` 가 따라오면 ST(`ESC \`), 아니면 본문 폐기
                0x1B => {
                    self.state = State::SawEscInBody;
                }
                _ => {
                    if self.body.len() >= MAX_OSC_BODY {
                        // 비정상 시퀀스 — 본문 폐기 후 idle
                        self.body.clear();
                        self.state = State::Idle;
                    } else {
                        self.body.push(byte);
                    }
                }
            },
            State::SawEscInBody => {
                if byte == b'\\' {
                    // ESC \ 형태의 ST 완료
                    self.dispatch_body(events);
                    self.body.clear();
                    self.state = State::Idle;
                } else if byte == 0x1B {
                    // 다시 ESC — 본문 폐기 후 SawEsc 로 재해석
                    self.body.clear();
                    self.state = State::SawEsc;
                } else {
                    // 잘못된 시퀀스 — 본문 폐기 후 idle
                    self.body.clear();
                    self.state = State::Idle;
                }
            }
        }
    }

    /// 본문 누적이 끝났을 때 OSC 133 인지 판정해 이벤트를 emit 한다.
    fn dispatch_body(&self, events: &mut Vec<ShellIntegrationEvent>) {
        // 본문이 "133;" 으로 시작하는지 확인
        let prefix = b"133;";
        if !self.body.starts_with(prefix) {
            return;
        }
        let rest = &self.body[prefix.len()..];
        if rest.is_empty() {
            return;
        }
        // 첫 바이트가 마크 (A/B/C/D)
        let mark = rest[0];
        let after_mark = &rest[1..];
        // 마크 다음은 `;` 또는 본문 끝(없음). 그 외는 무시(ill-formed)
        if !after_mark.is_empty() && after_mark[0] != b';' {
            return;
        }
        // 마크 뒤 첫 ';' 다음부터를 파라미터 영역으로 본다
        let params_raw: &[u8] = if after_mark.is_empty() {
            &[]
        } else {
            &after_mark[1..]
        };

        match mark {
            b'A' => events.push(ShellIntegrationEvent::PromptStart),
            b'B' => events.push(ShellIntegrationEvent::CommandStart),
            b'C' => events.push(ShellIntegrationEvent::CommandExecuted),
            b'D' => {
                let exit_code = parse_exit_code(params_raw);
                events.push(ShellIntegrationEvent::CommandFinished { exit_code });
            }
            _ => {}
        }
    }
}

/// `D` 마크의 파라미터에서 종료 코드를 추출한다.
/// 형식 예: `;0`, `;1`, `0`, `0;aid=...`, `;0;aid=...` 등 다양한 변형 허용.
fn parse_exit_code(params: &[u8]) -> Option<i32> {
    // ';' 로 분리해 첫 정수 토큰을 찾는다.
    let s = str::from_utf8(params).ok()?;
    for token in s.split(';') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        // key=value 형태(예: aid=...)는 스킵
        if trimmed.contains('=') {
            continue;
        }
        if let Ok(code) = trimmed.parse::<i32>() {
            return Some(code);
        }
        // 첫 비-key 토큰이 정수가 아니면 더 이상 시도 안 함
        break;
    }
    None
}

// =============================================================================
// 셸 통합 스크립트 자동 주입
// =============================================================================

/// 컴파일 타임 임베드 — bash 용
const BASH_SCRIPT: &str = include_str!("../assets/shell_integration/dokkaebi.bash");
/// 컴파일 타임 임베드 — pwsh / Windows PowerShell 용
const PWSH_SCRIPT: &str = include_str!("../assets/shell_integration/dokkaebi.ps1");

/// 셸 통합 주입 결과. UI/로깅 용도로 추적 가능.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectOutcome {
    /// 주입 성공 — 셸 종류와 사용한 스크립트 경로.
    Applied {
        shell: &'static str,
        script_path: PathBuf,
    },
    /// 주입하지 않음 — 사유 포함.
    Skipped { reason: &'static str },
}

/// 셸 종류·사용자 args·환경변수를 보고 OSC 133 통합 스크립트를 자동 주입한다.
///
/// 주입 정책 (Auto):
/// - `DOKKAEBI_SHELL_INTEGRATION=off` 환경변수 설정 시 항상 skip (escape hatch).
/// - 사용자가 셸 args 를 직접 지정한 경우 skip (덮어쓰기 회피).
/// - 지원 셸: `Posix`(bash 계열), `Pwsh`(PowerShell 7), `PowerShell`(레거시).
///   다른 셸(zsh/cmd/fish/nushell 등) 은 본 plan 범위 밖 → skip.
///
/// 호출자는 `args` 와 `env` 를 spawn 직전에 전달해 본 함수가 in-place 로 수정하게 한다.
/// rc 파일이나 사용자 dotfiles 는 절대 수정하지 않는다 — 우리 스크립트가 사용자
/// 원본을 명시적으로 source 한다.
pub fn inject_shell_integration(
    shell_kind: ShellKind,
    args: &mut Vec<String>,
    env: &mut Vec<(String, String)>,
) -> InjectOutcome {
    // escape hatch: 사용자 환경변수로 강제 비활성
    if env
        .iter()
        .any(|(k, v)| k == "DOKKAEBI_SHELL_INTEGRATION" && v.eq_ignore_ascii_case("off"))
    {
        return InjectOutcome::Skipped {
            reason: "DOKKAEBI_SHELL_INTEGRATION=off 환경변수로 비활성",
        };
    }

    // 사용자가 args 를 명시했으면(예: `bash -l`, `pwsh -File ...`) 안전을 위해 skip
    if !args.is_empty() {
        return InjectOutcome::Skipped {
            reason: "사용자 지정 셸 args 가 있어 자동 주입 보류",
        };
    }

    match shell_kind {
        ShellKind::Posix => match write_script_once("dokkaebi.bash", BASH_SCRIPT) {
            Ok(path) => {
                args.push("--rcfile".into());
                args.push(path.to_string_lossy().into_owned());
                args.push("-i".into());
                InjectOutcome::Applied {
                    shell: "bash",
                    script_path: path,
                }
            }
            Err(e) => {
                log::warn!("Dokkaebi shell integration: bash 스크립트 작성 실패 — {e}");
                InjectOutcome::Skipped {
                    reason: "bash 스크립트 작성 실패",
                }
            }
        },
        ShellKind::Pwsh | ShellKind::PowerShell => {
            match write_script_once("dokkaebi.ps1", PWSH_SCRIPT) {
                Ok(path) => {
                    args.push("-NoExit".into());
                    args.push("-Command".into());
                    // 단일 따옴표 escape: PS literal 문자열에서 '' 가 ' 한 글자
                    let escaped = path.to_string_lossy().replace('\'', "''");
                    args.push(format!("& '{}'", escaped));
                    InjectOutcome::Applied {
                        shell: if matches!(shell_kind, ShellKind::Pwsh) {
                            "pwsh"
                        } else {
                            "powershell"
                        },
                        script_path: path,
                    }
                }
                Err(e) => {
                    log::warn!("Dokkaebi shell integration: pwsh 스크립트 작성 실패 — {e}");
                    InjectOutcome::Skipped {
                        reason: "pwsh 스크립트 작성 실패",
                    }
                }
            }
        }
        _ => InjectOutcome::Skipped {
            reason: "지원하지 않는 셸 종류 (Auto 정책 외)",
        },
    }
}

/// 임시 디렉터리에 스크립트를 한 번만 작성하고 경로를 반환한다.
/// 동일 파일이 이미 존재하고 내용이 같으면 재작성하지 않는다.
fn write_script_once(filename: &str, contents: &str) -> std::io::Result<PathBuf> {
    static SCRIPT_DIR: OnceLock<PathBuf> = OnceLock::new();
    let dir = SCRIPT_DIR.get_or_init(|| {
        let dir = paths::temp_dir().join("shell_integration");
        let _ = std::fs::create_dir_all(&dir);
        dir
    });
    let path = dir.join(filename);
    let need_write = match std::fs::read_to_string(&path) {
        Ok(existing) => existing != contents,
        Err(_) => true,
    };
    if need_write {
        std::fs::write(&path, contents)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(bytes: &[u8]) -> Vec<ShellIntegrationEvent> {
        let mut scanner = Osc133Scanner::new();
        scanner.feed(bytes)
    }

    #[test]
    fn prompt_start_bel_terminated() {
        let events = run(b"\x1b]133;A\x07");
        assert_eq!(events, vec![ShellIntegrationEvent::PromptStart]);
    }

    #[test]
    fn prompt_start_st_terminated() {
        let events = run(b"\x1b]133;A\x1b\\");
        assert_eq!(events, vec![ShellIntegrationEvent::PromptStart]);
    }

    #[test]
    fn command_start_bel() {
        let events = run(b"\x1b]133;B\x07");
        assert_eq!(events, vec![ShellIntegrationEvent::CommandStart]);
    }

    #[test]
    fn command_executed_bel() {
        let events = run(b"\x1b]133;C\x07");
        assert_eq!(events, vec![ShellIntegrationEvent::CommandExecuted]);
    }

    #[test]
    fn command_finished_with_exit_zero() {
        let events = run(b"\x1b]133;D;0\x07");
        assert_eq!(
            events,
            vec![ShellIntegrationEvent::CommandFinished { exit_code: Some(0) }]
        );
    }

    #[test]
    fn command_finished_with_exit_one() {
        let events = run(b"\x1b]133;D;1\x07");
        assert_eq!(
            events,
            vec![ShellIntegrationEvent::CommandFinished { exit_code: Some(1) }]
        );
    }

    #[test]
    fn command_finished_without_exit_code() {
        let events = run(b"\x1b]133;D\x07");
        assert_eq!(
            events,
            vec![ShellIntegrationEvent::CommandFinished { exit_code: None }]
        );
    }

    #[test]
    fn command_finished_with_exit_and_aid() {
        let events = run(b"\x1b]133;D;127;aid=42\x07");
        assert_eq!(
            events,
            vec![ShellIntegrationEvent::CommandFinished {
                exit_code: Some(127)
            }]
        );
    }

    #[test]
    fn command_finished_aid_only() {
        let events = run(b"\x1b]133;D;aid=42\x07");
        assert_eq!(
            events,
            vec![ShellIntegrationEvent::CommandFinished { exit_code: None }]
        );
    }

    #[test]
    fn full_command_cycle() {
        let stream = b"\x1b]133;A\x07\x1b]133;B\x07echo hi\r\n\x1b]133;C\x07hi\r\n\x1b]133;D;0\x07";
        let events = run(stream);
        assert_eq!(
            events,
            vec![
                ShellIntegrationEvent::PromptStart,
                ShellIntegrationEvent::CommandStart,
                ShellIntegrationEvent::CommandExecuted,
                ShellIntegrationEvent::CommandFinished { exit_code: Some(0) },
            ]
        );
    }

    #[test]
    fn split_across_chunks() {
        let mut scanner = Osc133Scanner::new();
        // ESC ] 1 / 33;A BEL — read 버퍼 경계에 걸친 케이스
        let mut events = scanner.feed(b"\x1b]1");
        events.extend(scanner.feed(b"33;A\x07"));
        assert_eq!(events, vec![ShellIntegrationEvent::PromptStart]);
    }

    #[test]
    fn split_after_esc() {
        let mut scanner = Osc133Scanner::new();
        let mut events = scanner.feed(b"\x1b");
        events.extend(scanner.feed(b"]133;C\x07"));
        assert_eq!(events, vec![ShellIntegrationEvent::CommandExecuted]);
    }

    #[test]
    fn other_osc_ignored() {
        // OSC 0 (set window title) 는 무시되어야 한다
        let events = run(b"\x1b]0;hello\x07");
        assert_eq!(events, Vec::<ShellIntegrationEvent>::new());
    }

    #[test]
    fn osc_8_hyperlink_ignored() {
        let events = run(b"\x1b]8;;https://example.com\x07link\x1b]8;;\x07");
        assert_eq!(events, Vec::<ShellIntegrationEvent>::new());
    }

    #[test]
    fn plain_text_no_event() {
        let events = run(b"hello world\r\n");
        assert_eq!(events, Vec::<ShellIntegrationEvent>::new());
    }

    #[test]
    fn lone_esc_no_event() {
        let events = run(b"\x1b\x1b\x1b]");
        assert_eq!(events, Vec::<ShellIntegrationEvent>::new());
    }

    #[test]
    fn malformed_133_no_event() {
        // "133" 뒤 ';' 가 없으면 무시
        let events = run(b"\x1b]133A\x07");
        assert_eq!(events, Vec::<ShellIntegrationEvent>::new());
    }

    #[test]
    fn unknown_mark_ignored() {
        let events = run(b"\x1b]133;Z\x07");
        assert_eq!(events, Vec::<ShellIntegrationEvent>::new());
    }

    #[test]
    fn body_overflow_recovers() {
        // 매우 긴 OSC 본문은 폐기되고 이후 정상 시퀀스를 다시 인식한다
        let mut payload = vec![0x1B, b']'];
        payload.extend(std::iter::repeat(b'X').take(MAX_OSC_BODY + 100));
        payload.extend_from_slice(b"\x07\x1b]133;A\x07");
        let events = run(&payload);
        assert_eq!(events, vec![ShellIntegrationEvent::PromptStart]);
    }

    #[test]
    fn negative_exit_code() {
        let events = run(b"\x1b]133;D;-1\x07");
        assert_eq!(
            events,
            vec![ShellIntegrationEvent::CommandFinished {
                exit_code: Some(-1)
            }]
        );
    }
}
