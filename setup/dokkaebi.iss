; AppGuid는 escape 없는 raw GUID. Pascal [Code]에서 안전하게 사용 가능.
; AppId는 [Setup] AppId= 평가 시 한 글자 escape("{{")가 한 개 "{"로 환원되도록 "{" + AppGuid 조합.
#define AppGuid "{B8F4E2A1-7C3D-4E5F-9A1B-6D8E0F2C3A4B}"
#define AppId "{" + AppGuid
#define AppName "Dokkaebi"
#define Version "0.4.1"
#define AppSetupName "Dokkaebi-Setup-v" + Version
#define AppMutex "Dokkaebi-Instance-Mutex"
#define AppIconName "app-icon-dokkaebi"
#define AppExeName "dokkaebi"
#define AppUserId "BitLeader.Dokkaebi"
#define ResourcesDir "."
#define SourceDir "."
#define OutputDir "output"

[Setup]
AppId={#AppId}
AppName={#AppName}
; AppVerName 미지정 시 Inno Setup이 자동으로 "AppName AppVersion" 형태를 기본값으로 사용한다.
; 마법사·Add/Remove Programs 모두 버전 없는 "Dokkaebi"만 표시하도록 명시적으로 지정.
AppVerName={#AppName}
UninstallDisplayName={#AppName}
AppPublisher=Dokkaebi
AppCopyright=Copyright (c) 2026 Dokkaebi. Based on Zed (c) 2022-2025 Zed Industries, Inc.
AppPublisherURL=https://github.com/bitleader-dev/DokkaebiTerminal
AppSupportURL=https://github.com/bitleader-dev/DokkaebiTerminal
AppUpdatesURL=https://github.com/bitleader-dev/DokkaebiTerminal
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableReadyPage=yes
DisableDirPage=yes
AllowNoIcons=yes
OutputDir={#OutputDir}
OutputBaseFilename={#AppSetupName}
Compression=lzma
SolidCompression=yes
AppMutex={#AppMutex}
SetupMutex={#AppMutex}Setup
SetupIconFile={#ResourcesDir}\{#AppIconName}.ico
UninstallDisplayIcon={app}\{#AppExeName}.exe
MinVersion=10.0.16299
SourceDir={#SourceDir}
AppVersion={#Version}
VersionInfoVersion={#Version}
ShowLanguageDialog=yes
WizardStyle=modern
CloseApplications=force
DisableWelcomePage=no
WizardImageFile=welcome-icon.png
WizardImageAlphaFormat=defined

#if GetEnv("CI") != ""
SignTool=Defaultsign
#endif

DefaultDirName={localappdata}\Programs\{#AppName}
PrivilegesRequired=lowest

ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Messages]
korean.WelcomeLabel1=Dokkaebi에 오신 것을 환영합니다
korean.WelcomeLabel2=Zed를 기반으로 개발된 AI 코딩 에이전트를 위한 Windows 중심 터미널 작업 공간입니다.
english.WelcomeLabel1=Welcome to Dokkaebi
english.WelcomeLabel2=A Windows-focused terminal workspace for AI coding agents, built on Zed.

[CustomMessages]
; 다운그레이드 경고: %1 = 기존 설치 버전, %2 = 설치하려는 버전
english.DowngradeWarningText=A newer version (%1) of Dokkaebi is already installed.%nIf you continue, it will be downgraded to version %2.%n%nDo you want to continue?
korean.DowngradeWarningText=이미 설치된 버전(%1)이 설치하려는 버전(%2)보다 높습니다.%n계속 진행하면 이전 버전으로 되돌립니다.%n%n계속하시겠습니까?

[Registry]
; 앱이 런타임에 기록하는 자동 실행 레지스트리 값(HKCU\...\Run\Dokkaebi)을 언인스톨 시 함께 제거.
; 설치·업그레이드 시에는 값을 건드리지 않고(ValueType: none) 사용자가 앱에서 토글한 상태를 보존한다.
; 값이 없을 때도 uninsdeletevalue 는 조용히 통과한다.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Dokkaebi"; Flags: uninsdeletevalue

[UninstallDelete]
Type: filesandordirs; Name: "{app}\x64"
Type: filesandordirs; Name: "{app}\arm64"
Type: filesandordirs; Name: "{app}\config"
Type: filesandordirs; Name: "{app}\logs"
Type: filesandordirs; Name: "{app}\db"
Type: filesandordirs; Name: "{app}\extensions"
Type: filesandordirs; Name: "{app}\plugins"
Type: filesandordirs; Name: "{app}\state"
; data_dir() == {app} 구조라 런타임에 생성되는 모든 서브폴더(temp, hang_traces, server_state,
; remote_extensions, conversations, prompts, prompt_overrides, embeddings, languages,
; debug_adapters, external_agents, copilot, prettier, remote_servers, devcontainer 등)와
; 잔여 파일까지 설치 폴더 전체를 마지막에 일괄 제거
Type: filesandordirs; Name: "{app}"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Dirs]
Name: "{app}"; AfterInstall: DisableAppDirInheritance

[Files]
; GPL §4·§5 의무 — 라이선스 사본·NOTICE·변경 고지를 수령자에게 함께 전달.
; setup/dokkaebi.iss 위치 기준 상대경로(..\LICENSE-GPL 등)로 리포 루트의 원본을 참조.
; .txt 확장자를 부여해 메모장 더블클릭으로 열람 가능하게 한다.
; 이들 파일이 없으면 GPL 의무 위반이므로 #ifexist 가드 없이 컴파일 에러로 즉시 발견되도록 한다.
Source: "..\LICENSE-GPL"; DestDir: "{app}\licenses"; DestName: "LICENSE-GPL.txt"; Flags: ignoreversion
Source: "..\LICENSE-APACHE"; DestDir: "{app}\licenses"; DestName: "LICENSE-APACHE.txt"; Flags: ignoreversion
Source: "..\NOTICE"; DestDir: "{app}\licenses"; DestName: "NOTICE.txt"; Flags: ignoreversion
Source: "{#ResourcesDir}\{#AppExeName}.exe"; DestDir: "{app}"; Flags: ignoreversion
#ifexist ResourcesDir + "\tools"
Source: "{#ResourcesDir}\tools\*"; DestDir: "{app}\tools"; Flags: ignoreversion
#endif
#ifexist ResourcesDir + "\amd_ags_x64.dll"
Source: "{#ResourcesDir}\amd_ags_x64.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
Source: "{#ResourcesDir}\OpenConsole.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#ResourcesDir}\conpty.dll"; DestDir: "{app}"; Flags: ignoreversion
; cli 바이너리 — Dokkaebi 본체에 IPC 메시지를 전달하는 작은 클라이언트.
; Claude Code 플러그인이 dokkaebi-cli.exe를 호출해 작업 알림을 본체로 보낸다.
; dokkaebi-cli.exe는 {app}\dokkaebi.exe를 자동 탐지(./dokkaebi.exe 폴백)하여 Named Pipe로 IPC URL 전달.
; cargo bin name이 "dokkaebi-cli"로 빌드되므로 별도 리네임 불필요.
#ifexist ResourcesDir + "\dokkaebi-cli.exe"
Source: "{#ResourcesDir}\dokkaebi-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
#endif
; jq.exe — Claude Code 훅 스크립트(dispatch.sh)가 JSON payload 파싱에 사용.
; 사용자 환경에 jq 설치 여부와 무관하게 동작하도록 번들. dispatch.sh는 사용자 PATH
; 의 jq를 먼저 찾고 없으면 {app}\jq.exe를 폴백으로 사용한다.
; 라이선스: MIT (jqlang/jq). 출처: https://github.com/jqlang/jq/releases
#ifexist ResourcesDir + "\jq.exe"
Source: "{#ResourcesDir}\jq.exe"; DestDir: "{app}"; Flags: ignoreversion
#endif
; Claude Code 작업 알림 브리지 플러그인 (로컬 마켓플레이스 구조).
; Claude Code의 directory source는 `.claude-plugin/marketplace.json` 카탈로그가 있는
; 마켓플레이스 루트 디렉터리를 요구하므로 {app}\plugins\ 에 marketplace.json과
; 플러그인 서브디렉터리를 함께 배치한다.
;   {app}\plugins\.claude-plugin\marketplace.json
;   {app}\plugins\dokkaebi-notify-bridge\.claude-plugin\plugin.json
;   {app}\plugins\dokkaebi-notify-bridge\hooks\hooks.json
;   {app}\plugins\dokkaebi-notify-bridge\scripts\dispatch.sh
; 1순위: 인스톨러 작업 디렉터리(setup/)에 plugins/ 마켓플레이스 루트가 미리 복사되어 있는 경우.
; 2순위: 저장소 루트의 assets/claude-plugins/ (수동/로컬 빌드 경로).
; 설치된 후 사용자가 [설정 → 알림 → Claude Code → 플러그인 설치] 클릭 시 활성화됨.
#ifexist ResourcesDir + "\plugins\.claude-plugin\marketplace.json"
Source: "{#ResourcesDir}\plugins\*"; DestDir: "{app}\plugins"; Flags: ignoreversion recursesubdirs createallsubdirs
#else
#ifexist ResourcesDir + "\..\assets\claude-plugins\.claude-plugin\marketplace.json"
Source: "{#ResourcesDir}\..\assets\claude-plugins\*"; DestDir: "{app}\plugins"; Flags: ignoreversion recursesubdirs createallsubdirs
#endif
#endif

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; AppUserModelID: "{#AppUserId}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; Tasks: desktopicon; AppUserModelID: "{#AppUserId}"

[Run]
; 인스톨러가 PostInstall 단계에서 dokkaebi 를 직접 spawn 하면 자식 chain
; (dokkaebi → 터미널 → claude → bash → dispatch.sh → jq) 이 인스톨러 process
; 의 spawn 컨텍스트(환경변수 PATH 중복 등)를 상속해 일부 user-owned reparse
; point(예: %LOCALAPPDATA%\Microsoft\WinGet\Links\jq symlink) spawn 이 차단된다.
; 결과적으로 Claude Code Task 호출 시 jq 가 실패 → SubagentStart/Stop IPC 가
; 본체에 도달하지 않아 서브에이전트 뷰 탭 자동 생성이 안 된다 (사용자 보고:
; "설치 후 자동 실행만 100% 재현, 종료 후 시작메뉴 직접 실행은 정상").
;
; explorer.exe 를 거쳐 spawn 하면 dokkaebi 의 부모가 인스톨러가 아닌
; explorer 가 되어 시작메뉴 더블클릭과 동등한 컨텍스트로 시작되고 자식 chain
; 도 정상 환경을 상속한다. 인스톨러 자체는 비-elevated(`PrivilegesRequired=lowest`)
; 라 RunAs/권한 변경과는 무관 — 순수 spawn 컨텍스트 격리 fix.
Filename: "{win}\explorer.exe"; Parameters: """{app}\{#AppExeName}.exe"""; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall; Check: WizardNotSilent
; silent 업데이트(앱 내부 자동 업데이트) 후 앱을 자동 실행. 같은 explorer 우회
; 적용. explorer.exe 가 추가 인자(`--updated`) 를 자식에 전달하지 않으므로
; "방금 업데이트됐음" 시그널 기반 릴리즈 노트 1회 자동 표시는 본 fix 와 함께
; 일시적으로 비활성된다. 자동 표시는 별도 follow-up 으로 본체 측 메커니즘
; (예: 직전 버전 비교) 으로 재구현 예정. 우선순위는 핵심 기능(서브에이전트 탭
; 자동 생성) 정상화이므로 release notes 자동 표시는 임시 trade-off.
Filename: "{win}\explorer.exe"; Parameters: """{app}\{#AppExeName}.exe"""; Flags: nowait; Check: WizardSilent

[Code]
function WizardNotSilent(): Boolean;
begin
  Result := not WizardSilent();
end;

// 다운그레이드 차단/경고
// HKCU\...\{AppId}_is1\DisplayVersion 으로 기존 설치 버전을 읽어 신규 버전과 비교한다.
// - 신규 설치(키 없음) / 동일 / 업그레이드: 통과
// - 다운그레이드(기존 > 신규):
//   * silent (앱 자동 업데이트 경로): 메시지 없이 즉시 차단
//   * interactive: 한/영 메시지로 Yes/No 확인. 기본 버튼은 No(안전).
function InitializeSetup(): Boolean;
var
  RegKey: String;
  InstalledVer: String;
  InstalledPacked: Int64;
  NewPacked: Int64;
  Msg: String;
begin
  Result := True;
  // AppGuid는 raw GUID(중괄호 1개)이므로 Pascal string에서 그대로 안전하게 결합 가능.
  RegKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#AppGuid}_is1';

  if not RegQueryStringValue(HKCU, RegKey, 'DisplayVersion', InstalledVer) then
    Exit;

  // 버전 문자열 파싱 실패 시 안전하게 통과(레거시/손상된 레지스트리 보호)
  if not StrToVersion(InstalledVer, InstalledPacked) then
    Exit;
  if not StrToVersion('{#Version}', NewPacked) then
    Exit;

  if ComparePackedVersion(InstalledPacked, NewPacked) <= 0 then
    Exit;

  if WizardSilent() then
  begin
    Result := False;
    Exit;
  end;

  Msg := FmtMessage(CustomMessage('DowngradeWarningText'), [InstalledVer, '{#Version}']);
  if MsgBox(Msg, mbConfirmation, MB_YESNO or MB_DEFBUTTON2) <> IDYES then
    Result := False;
end;

// 제거 시작 시점(AppMutex 체크 이전)에 설치 경로와 일치하는 Dokkaebi 프로세스만 강제 종료
// PowerShell Get-Process에 -Name 매칭 후 Where-Object로 Path 엄격 비교 → 다른 위치 동명 프로세스는 보호
// -ErrorAction SilentlyContinue: 프로세스가 없어도 오류 없이 통과
// Sleep(500)으로 OS가 파일 핸들을 해제할 시간 확보 후 제거 절차 계속 진행
function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
  AppExePath: String;
  PSCommand: String;
begin
  AppExePath := ExpandConstant('{app}\{#AppExeName}.exe');
  PSCommand := '-NoProfile -Command "Get-Process -Name {#AppExeName} -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq ''' + AppExePath + ''' } | Stop-Process -Force"';
  Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), PSCommand, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(500);
  Result := True;
end;

// 제거 usUninstall 단계(파일 삭제 직전)에서 Claude Code 훅을 정리한다.
// dokkaebi-cli.exe --uninstall-claude-plugin 이 사용자 프로필의
// ~/.claude/settings.json 에서 Dokkaebi 플러그인 등록 항목만 안전하게 제거한다.
// silent 언인스톨(앱 자동 업데이트의 재설치 경로)에서는 skip 해야 업데이트 후
// 사용자가 [설치] 버튼을 다시 눌러야 하는 UX 회귀를 피한다.
// cli.exe 가 없거나 실행에 실패해도 언인스톨 자체는 계속 진행한다.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
  CliPath: String;
begin
  if CurUninstallStep <> usUninstall then
    Exit;
  if UninstallSilent() then
    Exit;
  CliPath := ExpandConstant('{app}\dokkaebi-cli.exe');
  if not FileExists(CliPath) then
    Exit;
  Exec(CliPath, '--uninstall-claude-plugin', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/icacls
// https://docs.microsoft.com/en-US/windows/security/identity-protection/access-control/security-identifiers
procedure DisableAppDirInheritance();
var
  ResultCode: Integer;
  Permissions: string;
begin
  Permissions := '/grant:r "*S-1-5-18:(OI)(CI)F" /grant:r "*S-1-5-32-544:(OI)(CI)F" /grant:r "*S-1-5-11:(OI)(CI)RX" /grant:r "*S-1-5-32-545:(OI)(CI)RX"';

  Permissions := Permissions + Format(' /grant:r "*S-1-3-0:(OI)(CI)F" /grant:r "%s:(OI)(CI)F"', [GetUserNameString()]);

  Exec(ExpandConstant('{sys}\icacls.exe'), ExpandConstant('"{app}" /inheritancelevel:r ') + Permissions, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;
