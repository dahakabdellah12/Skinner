#define MyAppName "BrawlCraft"
#define MyAppVersion "2.2.4"
#define MyAppPublisher "dahakabdellah12"
#define MyAppURL "https://github.com/dahakabdellah12/BrawlCraft"
#define MyAppExeName "BrawlCraft.exe"

[Setup]
AppId={{91C68AE2-A23F-433E-BDFA-5BA2D4EF6A51}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
DisableProgramGroupPage=yes
OutputDir=C:\Users\Dahak Abdellah\Downloads\Project\BrawlCraft\release
OutputBaseFilename=BrawlCraft-v2.2.4-Setup
SetupIconFile=C:\Users\Dahak Abdellah\Downloads\Project\BrawlCraft\src\BrawlhallaSkinStudio.App\app.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "C:\Users\Dahak Abdellah\Downloads\Project\BrawlCraft\publish\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
