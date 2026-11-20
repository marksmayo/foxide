#define AppName "Foxide"
#ifndef AppVersion
  #error "AppVersion must be supplied by the release workflow."
#endif
#ifndef PayloadDir
  #error "PayloadDir must be supplied by the release workflow."
#endif

[Setup]
AppId={{8C5BFEB4-4B63-4B42-9C51-5C6076C6C8A3}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Foxide
DefaultDirName={localappdata}\Programs\Foxide
DefaultGroupName=Foxide
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Uninstallable=yes
SetupIconFile=Foxide.ico
UninstallDisplayIcon={app}\Foxide.ico
OutputDir=..\dist
OutputBaseFilename=Foxide-{#AppVersion}-windows-x64-setup
SetupLogging=yes
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "Foxide.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Foxide"; Filename: "{app}\Foxide.exe"; IconFilename: "{app}\Foxide.ico"
Name: "{group}\Install playback runtime"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\Install-PlaybackRuntime.ps1"""; WorkingDir: "{app}"
Name: "{autodesktop}\Foxide"; Filename: "{app}\Foxide.exe"; IconFilename: "{app}\Foxide.ico"; Tasks: desktopicon
