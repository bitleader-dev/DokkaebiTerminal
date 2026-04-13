[Setup]
AppId={#AppId}
AppName={#AppName}
AppVerName={#AppDisplayName}
AppPublisher=Dokkaebi
AppPublisherURL=https://github.com/bitleader-dev/DokkaebiTerminal
AppSupportURL=https://github.com/bitleader-dev/DokkaebiTerminal
AppUpdatesURL=https://github.com/bitleader-dev/DokkaebiTerminal
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableReadyPage=yes
AllowNoIcons=yes
OutputDir={#OutputDir}
OutputBaseFilename={#AppSetupName}
Compression=lzma
SolidCompression=yes
AppMutex={code:GetAppMutex}
SetupMutex={#AppMutex}Setup
; WizardImageFile="{#ResourcesDir}\inno-100.bmp,{#ResourcesDir}\inno-125.bmp,{#ResourcesDir}\inno-150.bmp,{#ResourcesDir}\inno-175.bmp,{#ResourcesDir}\inno-200.bmp,{#ResourcesDir}\inno-225.bmp,{#ResourcesDir}\inno-250.bmp"
; WizardSmallImageFile="{#ResourcesDir}\inno-small-100.bmp,{#ResourcesDir}\inno-small-125.bmp,{#ResourcesDir}\inno-small-150.bmp,{#ResourcesDir}\inno-small-175.bmp,{#ResourcesDir}\inno-small-200.bmp,{#ResourcesDir}\inno-small-225.bmp,{#ResourcesDir}\inno-small-250.bmp"
SetupIconFile={#ResourcesDir}\{#AppIconName}.ico
UninstallDisplayIcon={app}\{#AppExeName}.exe
MinVersion=10.0.16299
SourceDir={#SourceDir}
AppVersion={#Version}
VersionInfoVersion={#Version}
ShowLanguageDialog=auto
WizardStyle=modern

CloseApplications=force

#if GetEnv("CI") != ""
SignTool=Defaultsign
#endif

DefaultDirName={autopf}\{#AppName}
PrivilegesRequired=lowest

ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"; LicenseFile: "script\terms\terms.rtf"
Name: "english"; MessagesFile: "compiler:Default.isl,{#ResourcesDir}\messages\en.isl"; LicenseFile: "script\terms\terms.rtf"
Name: "simplifiedChinese"; MessagesFile: "{#ResourcesDir}\messages\Default.zh-cn.isl,{#ResourcesDir}\messages\zh-cn.isl"; LicenseFile: "script\terms\terms.rtf"

[UninstallDelete]
; Delete logs
Type: filesandordirs; Name: "{app}\tools"
Type: filesandordirs; Name: "{app}\updates"
; Delete newer files which may not have been added by the initial installation
Type: filesandordirs; Name: "{app}\x64"
Type: filesandordirs; Name: "{app}\arm64"


[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Dirs]
Name: "{app}"; AfterInstall: DisableAppDirInheritance

[Files]
Source: "{#ResourcesDir}\Zed.exe"; DestDir: "{code:GetInstallDir}"; Flags: ignoreversion
Source: "{#ResourcesDir}\bin\*"; DestDir: "{code:GetInstallDir}\bin"; Flags: ignoreversion
Source: "{#ResourcesDir}\tools\*"; DestDir: "{app}\tools"; Flags: ignoreversion
#ifexist ResourcesDir + "\amd_ags_x64.dll"
Source: "{#ResourcesDir}\amd_ags_x64.dll"; DestDir: "{app}"; Flags: ignoreversion
#endif
#ifexist ResourcesDir + "\x64\OpenConsole.exe"
Source: "{#ResourcesDir}\x64\OpenConsole.exe"; DestDir: "{code:GetInstallDir}\x64"; Flags: ignoreversion
#endif
#ifexist ResourcesDir + "\arm64\OpenConsole.exe"
Source: "{#ResourcesDir}\arm64\OpenConsole.exe"; DestDir: "{code:GetInstallDir}\arm64"; Flags: ignoreversion
#endif
Source: "{#ResourcesDir}\conpty.dll"; DestDir: "{code:GetInstallDir}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; AppUserModelID: "{#AppUserId}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; Tasks: desktopicon; AppUserModelID: "{#AppUserId}"

[Run]
Filename: "{app}\{#AppExeName}.exe"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall; Check: WizardNotSilent

[UninstallRun]

[Registry]



[Code]
function WizardNotSilent(): Boolean;
begin
  Result := not WizardSilent();
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

function SwitchHasValue(Name: string; Value: string): Boolean;
begin
  Result := CompareText(ExpandConstant('{param:' + Name + '}'), Value) = 0;
end;

function IsUpdating(): Boolean;
begin
  Result := SwitchHasValue('update', 'true') and WizardSilent();
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if IsUpdating() then
    begin
      SaveStringToFile(ExpandConstant('{app}\updates\versions.txt'), '{#Version}' + #13#10, True);
    end
  end;
end;

function GetAppMutex(Param: string): string;
begin
  if IsUpdating() then
    Result := ''
  else
    Result := '{#AppMutex}';
end;

function GetInstallDir(Param: string): string;
begin
  if IsUpdating() then
    Result := ExpandConstant('{app}\install')
  else
    Result := ExpandConstant('{app}');
end;
