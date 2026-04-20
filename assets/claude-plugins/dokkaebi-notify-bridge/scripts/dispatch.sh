#!/usr/bin/env bash
# Claude Code hook → dokkaebi-cli.exe --notify-* IPC 브리지.
# 인자: 알림 종류 ("stop" | "idle" | "permission")
# stdin: Claude Code hook payload (JSON)
# 동작:
#   1. cli 바이너리(dokkaebi-cli.exe) 탐색. DOKKAEBI_CLI 환경변수 → PATH → 기본 설치 경로.
#      cli.exe는 Dokkaebi 본체에 Named Pipe로 IPC URL을 전달하는 별도 클라이언트.
#      본체에 직접 --notify-kind를 보내면 본체의 clap이 unknown argument로 거부함.
#   2. Dokkaebi 본체 인스턴스 실행 중인지 확인 (꺼져 있으면 cli가 새 본체를 spawn하므로 skip).
#   3. stdin JSON에서 cwd 추출 (라우팅 힌트로 전달).
#   4. dokkaebi-cli.exe --notify-kind <kind> --notify-title "..." --notify-message "..." [--notify-cwd ...]
# 실패해도 Claude Code UX 영향 없도록 항상 0 반환.

set +e

KIND="${1:-stop}"

# 알림 종류별 제목/본문 (i18n 미적용 — 사용자 가시 문구. 추후 환경변수 등으로 확장 가능)
case "$KIND" in
  stop)
    TITLE="Claude Code"
    MESSAGE="작업이 완료되었습니다."
    ;;
  idle)
    TITLE="Claude Code"
    MESSAGE="사용자 입력을 기다리고 있습니다."
    ;;
  permission)
    TITLE="Claude Code"
    MESSAGE="도구 사용 권한 승인이 필요합니다."
    ;;
  *)
    TITLE="Claude Code"
    MESSAGE="$KIND"
    ;;
esac

# cli 실행 파일 탐색 — 본체(dokkaebi.exe)가 아닌 별도 cli 바이너리를 사용해야 한다.
# 본체는 paths_or_urls 위주 인자만 파싱하므로 --notify-kind 전달 시 clap 에러 발생.
find_cli() {
  if [ -n "${DOKKAEBI_CLI:-}" ] && [ -x "$DOKKAEBI_CLI" ]; then
    echo "$DOKKAEBI_CLI"
    return 0
  fi
  if command -v dokkaebi-cli.exe >/dev/null 2>&1; then
    command -v dokkaebi-cli.exe
    return 0
  fi
  # Windows 기본 설치 경로 (LOCALAPPDATA) — 인스톨러가 본체와 같은 디렉터리에 배포
  local default_path="${LOCALAPPDATA:-$HOME/AppData/Local}/Programs/Dokkaebi/dokkaebi-cli.exe"
  if [ -x "$default_path" ]; then
    echo "$default_path"
    return 0
  fi
  return 1
}

CLI=$(find_cli)
if [ -z "$CLI" ]; then
  exit 0
fi

# Dokkaebi 본체가 실행 중인지 확인 (없으면 cli가 새 본체를 spawn하므로 skip).
# tasklist는 Windows 명령. Git Bash/MSYS는 `/FI` 같은 단일 슬래시 인자를
# Windows 경로(`C:/Program Files/Git/FI`)로 변환해 tasklist가 "잘못된 인수"로
# 거부한다. POSIX 스타일 대시 플래그(-FI)는 경로 변환을 우회하면서 cmd.exe
# 환경에서도 동일하게 동작한다.
if command -v tasklist >/dev/null 2>&1; then
  if ! tasklist -FI "IMAGENAME eq dokkaebi.exe" 2>/dev/null | grep -qi "dokkaebi.exe"; then
    exit 0
  fi
fi

# stdin에서 cwd 추출 (jq 우선, 없으면 grep fallback)
CWD=""
PAYLOAD=$(cat 2>/dev/null || true)
if [ -n "$PAYLOAD" ]; then
  if command -v jq >/dev/null 2>&1; then
    CWD=$(printf '%s' "$PAYLOAD" | jq -r '.cwd // empty' 2>/dev/null)
  else
    CWD=$(printf '%s' "$PAYLOAD" | grep -oE '"cwd"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"cwd"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
  fi
fi

# Claude Code 프로세스 PID = 이 bash script의 부모. Dokkaebi 본체가 이 PID의
# parent chain을 따라가며 각 터미널의 shell PID와 일치하는 터미널을 정확히
# 찾아 dot 인디케이터/그룹 배지를 해당 탭 한 곳에만 표시한다. 같은 이름의
# 터미널이 여러 개이거나 같은 cwd에 여러 탭이 열린 환경에서도 정확한 탭을
# 식별할 수 있다.
#
# MSYS/Git Bash 주의: 일반 `$PPID` 는 MSYS 가상 PID이며 Windows 시스템 프로세스
# 테이블과 매칭되지 않는다. `/proc/PID/winpid` 가상 파일이 해당 프로세스의
# Win32 PID를 제공하므로 이를 우선 사용하고, 파일이 없으면 `$PPID` 로 폴백
# (네이티브 bash 등 MSYS가 아닌 환경 대응).
CLAUDE_PID=""
if [ -r "/proc/${PPID}/winpid" ]; then
  CLAUDE_PID=$(cat "/proc/${PPID}/winpid" 2>/dev/null)
fi
if [ -z "$CLAUDE_PID" ]; then
  CLAUDE_PID="${PPID:-0}"
fi

# cli 호출을 **동기 실행**한다(`&` 없음). 이유: Dokkaebi 본체는 cli가 전송한
# Toolhelp snapshot으로 parent chain을 따라 Claude가 실행 중인 터미널을 식별
# 하는데, background 실행 시 bash/dispatch.sh가 cli와 동시에 종료하면서
# 그 조상(Claude의 wrapper 등)이 snapshot 시점에 이미 exit해 chain이 중간에
# 끊긴다. 동기 실행하면 bash가 cli 종료까지 살아있어 전체 chain을 캡처할 수
# 있다. cli는 IPC handshake 후 즉시 종료하므로 Claude Code hook 응답 지연은
# 수십 ms 수준으로 무시 가능.
if [ -n "$CWD" ]; then
  "$CLI" --notify-kind "$KIND" --notify-title "$TITLE" --notify-message "$MESSAGE" --notify-cwd "$CWD" --notify-pid "$CLAUDE_PID" >/dev/null 2>&1
else
  "$CLI" --notify-kind "$KIND" --notify-title "$TITLE" --notify-message "$MESSAGE" --notify-pid "$CLAUDE_PID" >/dev/null 2>&1
fi

exit 0
