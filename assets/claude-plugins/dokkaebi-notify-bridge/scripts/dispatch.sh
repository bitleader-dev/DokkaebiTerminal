#!/usr/bin/env bash
# Claude Code hook → dokkaebi-cli.exe --notify-* IPC 브리지.
# 인자: 알림 종류 ("stop" | "idle" | "permission")
# stdin: Claude Code hook payload (JSON)
# 동작:
#   1. cli 바이너리(dokkaebi-cli.exe) 탐색. DOKKAEBI_CLI 환경변수 → PATH → 기본 설치 경로.
#      cli.exe는 Dokkaebi 본체에 Named Pipe로 IPC URL을 전달하는 별도 클라이언트.
#      본체에 직접 --notify-kind를 보내면 본체의 clap이 unknown argument로 거부함.
#   2. Dokkaebi 본체 인스턴스 실행 중인지 확인 (꺼져 있으면 cli가 새 본체를 spawn하므로 skip).
#   3. stdin JSON payload 파싱:
#      - 공통: cwd
#      - stop: stop_hook_active (true면 재귀 호출이므로 중복 방지 skip),
#               transcript_path 에서 마지막 user 프롬프트 + 마지막 assistant 응답 추출
#      - idle: payload.message (Claude 가 띄운 원본 메시지)
#      - permission: tool_name + tool_input 의 command/file_path preview
#   4. dokkaebi-cli.exe --notify-kind <kind> [--notify-cwd ...] --notify-pid ...
#      [--notify-prompt ...] [--notify-response ...]
#      [--notify-tool-name ...] [--notify-tool-preview ...]
#      [--notify-idle-summary ...]
# 알림 제목/본문 문자열은 본체가 i18n 으로 UI 언어에 맞춰 생성하므로
# dispatch.sh 는 동적 내용만 전달한다.
# 실패해도 Claude Code UX 영향 없도록 항상 0 반환.

set +e

KIND="${1:-stop}"

# 긴 문자열을 지정 길이로 자르고 말줄임표를 붙인다.
# 사용: RESULT=$(truncate "$TEXT" 200)
truncate() {
  local s="$1" n="$2"
  if [ ${#s} -gt "$n" ]; then
    printf '%s...' "${s:0:$((n - 3))}"
  else
    printf '%s' "$s"
  fi
}

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

# stdin payload 읽기. 이후 여러 번 파싱에 재사용한다(stdin 은 한 번만 읽을 수 있음).
PAYLOAD=$(cat 2>/dev/null || true)

# jq 존재 여부를 1회만 조회해 캐싱. Windows Git Bash 는 프로세스 spawn 비용이
# 커서 command -v jq 반복 호출 자체도 수십 ms 단위로 비용이 누적된다.
HAVE_JQ=0
if command -v jq >/dev/null 2>&1; then
  HAVE_JQ=1
fi

# 공통: cwd 추출 (jq 우선, 없으면 grep fallback)
CWD=""
if [ -n "$PAYLOAD" ]; then
  if [ "$HAVE_JQ" = "1" ]; then
    CWD=$(printf '%s' "$PAYLOAD" | jq -r '.cwd // empty' 2>/dev/null)
  else
    CWD=$(printf '%s' "$PAYLOAD" | grep -oE '"cwd"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"cwd"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
  fi
fi

# KIND별 동적 필드 파싱. jq 없으면 동적 필드 전부 생략되고 본체가 default_body 로 폴백.
DYN_ARGS=()
case "$KIND" in
  stop)
    # stop_hook_active=true 면 재귀 호출이므로 중복 알림 방지를 위해 즉시 종료.
    # 동시에 transcript_path 도 같은 jq 호출에서 뽑아 fork 회수를 줄인다.
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      STOP_FIELDS=$(printf '%s' "$PAYLOAD" | jq -r '[(.stop_hook_active // false), (.transcript_path // "")] | @tsv' 2>/dev/null)
      IFS=$'\t' read -r STOP_HOOK_ACTIVE TRANSCRIPT_PATH <<< "$STOP_FIELDS"
      if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
        exit 0
      fi
      if [ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ]; then
        # Stop hook 은 transcript flush 보다 먼저 발화하므로 마지막 user/assistant
        # 블록이 파일에 쓰일 때까지 잠시 대기한다. 읽을 transcript 가 있을 때만
        # 지연하므로 jq 미설치/ transcript 부재 환경은 즉시 진행한다.
        sleep 0.3
        # 마지막 user 프롬프트 — tool_result 블록만 있는 user 메시지는 제외하고
        # 실제 text 블록(또는 plain string content)을 가진 메시지만 고른다.
        PROMPT=$(jq -rs '
          [
            .[] | select(.type == "user") |
            if .message.content | type == "string" then .
            elif [.message.content[] | select(.type == "text")] | length > 0 then .
            else empty
            end
          ] | last |
          if .message.content | type == "array"
          then [.message.content[] | select(.type == "text") | .text] | join(" ")
          else .message.content // empty
          end
        ' "$TRANSCRIPT_PATH" 2>/dev/null)
        # 마지막 assistant 응답 text 블록 전체를 한 줄로 합친다.
        RESPONSE=$(jq -rs '
          [.[] | select(.type == "assistant" and .message.content)] | last |
          [.message.content[] | select(.type == "text") | .text] | join(" ")
        ' "$TRANSCRIPT_PATH" 2>/dev/null)
        PROMPT=$(truncate "$PROMPT" 200)
        RESPONSE=$(truncate "$RESPONSE" 200)
        [ -n "$PROMPT" ] && DYN_ARGS+=("--notify-prompt" "$PROMPT")
        [ -n "$RESPONSE" ] && DYN_ARGS+=("--notify-response" "$RESPONSE")
      fi
    fi
    ;;
  idle)
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      IDLE_MSG=$(printf '%s' "$PAYLOAD" | jq -r '.message // empty' 2>/dev/null)
      [ -n "$IDLE_MSG" ] && DYN_ARGS+=("--notify-idle-summary" "$IDLE_MSG")
    fi
    ;;
  permission)
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      # tool_name 과 tool_input preview 를 한 번의 jq 호출로 TSV 추출.
      # tool_input 은 command(Bash) / file_path(Edit/Read/Write) 우선, 둘 다 없으면
      # tool_input 전체를 80자로 잘라 preview 로 사용한다.
      PERM_FIELDS=$(printf '%s' "$PAYLOAD" | jq -r '
        [
          (.tool_name // ""),
          ((.tool_input | if .command then .command
                          elif .file_path then .file_path
                          else (tostring | .[0:80]) end) // "")
        ] | @tsv
      ' 2>/dev/null)
      IFS=$'\t' read -r TOOL_NAME TOOL_PREVIEW <<< "$PERM_FIELDS"
      TOOL_PREVIEW=$(truncate "$TOOL_PREVIEW" 120)
      [ -n "$TOOL_NAME" ] && DYN_ARGS+=("--notify-tool-name" "$TOOL_NAME")
      [ -n "$TOOL_PREVIEW" ] && DYN_ARGS+=("--notify-tool-preview" "$TOOL_PREVIEW")
    fi
    ;;
esac

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
  "$CLI" --notify-kind "$KIND" --notify-cwd "$CWD" --notify-pid "$CLAUDE_PID" "${DYN_ARGS[@]}" >/dev/null 2>&1
else
  "$CLI" --notify-kind "$KIND" --notify-pid "$CLAUDE_PID" "${DYN_ARGS[@]}" >/dev/null 2>&1
fi

exit 0
