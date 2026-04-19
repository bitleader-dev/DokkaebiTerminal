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
# tasklist는 Windows 명령. MSYS/Git Bash 환경에서 호출 가능.
if command -v tasklist >/dev/null 2>&1; then
  if ! tasklist /FI "IMAGENAME eq dokkaebi.exe" 2>/dev/null | grep -qi "dokkaebi.exe"; then
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

# cli 호출 (실패해도 무시). cli는 IPC handshake 후 종료하므로 background 불필요하지만,
# Claude Code hook 응답 지연을 막기 위해 background로 실행.
if [ -n "$CWD" ]; then
  "$CLI" --notify-kind "$KIND" --notify-title "$TITLE" --notify-message "$MESSAGE" --notify-cwd "$CWD" >/dev/null 2>&1 &
else
  "$CLI" --notify-kind "$KIND" --notify-title "$TITLE" --notify-message "$MESSAGE" >/dev/null 2>&1 &
fi

exit 0
