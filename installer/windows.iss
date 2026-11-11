#define AppVersion "0.5.0"

[Setup]
AppId={{7C1F4E2A-9B3D-4C6E-A8F1-CA12E7D5B9E0}
AppName=Carnet de notes
AppVersion={#AppVersion}
AppVerName=Carnet de notes {#AppVersion}
AppPublisher=jeremdevgit
AppPublisherURL=https://github.com/jeremdevgit/carnet-rust
AppSupportURL=https://github.com/jeremdevgit/carnet-rust/issues
DefaultDirName={localappdata}\Programs\Carnet de notes
DefaultGroupName=Carnet de notes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=CarnetDeNotes-{#AppVersion}-Windows-x64-Setup
SetupIconFile=..\assets\carnet-de-notes.ico
UninstallDisplayIcon={app}\CarnetDeNotes.exe
UninstallDisplayName=Carnet de notes
Compression=lzma2
SolidCompression=yes
LicenseFile=..\LICENSE
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
; Inno Setup ships Chinese only as an unofficial translation, so include it when installed.
#if FileExists(AddBackslash(CompilerPath) + "Languages\ChineseSimplified.isl")
Name: "chinese"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
#endif
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\CarnetDeNotes.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.fr.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\SECURITY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\COMPATIBILITY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\OFL-Noto.txt"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\NOTICE.txt"; DestDir: "{app}\assets"; Flags: ignoreversion

[Icons]
Name: "{group}\Carnet de notes"; Filename: "{app}\CarnetDeNotes.exe"; WorkingDir: "{app}"
Name: "{group}\{cm:UninstallProgram,Carnet de notes}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Carnet de notes"; Filename: "{app}\CarnetDeNotes.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\CarnetDeNotes.exe"; Description: "{cm:LaunchProgram,Carnet de notes}"; Flags: nowait postinstall skipifsilent

; The encrypted notebook profile lives in the user's AppData and is never removed by the uninstaller.
