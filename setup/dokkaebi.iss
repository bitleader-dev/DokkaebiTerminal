; AppGuid는 escape 없는 raw GUID. Pascal [Code]에서 안전하게 사용 가능.
; AppId는 [Setup] AppId= 평가 시 한 글자 escape("{{")가 한 개 "{"로 환원되도록 "{" + AppGuid 조합.
#define AppGuid "{B8F4E2A1-7C3D-4E5F-9A1B-6D8E0F2C3A4B}"
#define AppId "{" + AppGuid
#define AppName "Dokkaebi"
#define Version "0.2.0"
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

[UninstallDelete]
Type: filesandordirs; Name: "{app}\x64"
Type: filesandordirs; Name: "{app}\arm64"
Type: filesandordirs; Name: "{app}\config"
Type: filesandordirs; Name: "{app}\logs"
Type: filesandordirs; Name: "{app}\db"
Type: filesandordirs; Name: "{app}\extensions"
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
Source: "{#ResourcesDir}\{#AppExeName}.exe"; DestDir: "{app}"; Flags: ignoreversion
#ifexist ResourcesDir + "\tools"
Source: "{#ResourcesDir}\tools\*"; DestDir: "{app}\tools"; Flags: ignoreversion
#endif
#ifexist ResourcesDir + "\amd_ags_x64.dll"
Source: "{#ResourcesDir}\amd_ags_x64.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
Source: "{#ResourcesDir}\OpenConsole.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#ResourcesDir}\conpty.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; AppUserModelID: "{#AppUserId}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; Tasks: desktopicon; AppUserModelID: "{#AppUserId}"

[Run]
Filename: "{app}\{#AppExeName}.exe"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall; Check: WizardNotSilent
; silent 업데이트(앱 내부 자동 업데이트) 후 앱을 자동 실행하며, --updated 파라미터로
; "직전 설치가 업데이트였음" 시그널을 앱에 전달한다(릴리즈 노트 1회 자동 표시 트리거).
Filename: "{app}\{#AppExeName}.exe"; Parameters: "--updated"; Flags: nowait; Check: WizardSilent

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
