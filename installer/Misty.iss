;  ███╗   ███╗██╗███████╗████████╗██╗     ██╗   ██╗
;  ████╗ ████║██║██╔════╝╚══██╔══╝██║     ╚██╗ ██╔╝
;  ██╔████╔██║██║███████╗   ██║   ██║      ╚████╔╝
;  ██║╚██╔╝██║██║╚════██║   ██║   ██║       ╚██╔╝
;  ██║ ╚═╝ ██║██║███████║   ██║   ███████╗   ██║
;  ╚═╝     ╚═╝╚═╝╚══════╝   ╚═╝   ╚══════╝   ╚═╝
;
;  Mistly Launcher · https://mistly.cc
;  Code : https://github.com/Mistly-MC/Mistly-Launcher
;  Discord : https://discord.gg/rdQJUgvUh3
;  Misty License 1.0 : free to use, modify and share, not to sell (LICENSE.txt)
;
; Installateur Windows de Mistly Launcher (Inno Setup 6.3+).
; La fenêtre que voit le joueur est Mistly-Setup.exe (crates\misty-setup) : elle contient cet
; installateur et le lance sans fenêtre (/VERYSILENT /DIR=…). Lancé seul, il garde ses pages.
; Ne pas compiler à la main : lancer installer\build.ps1, qui compile l'app puis appelle ISCC avec la bonne version.
;
; Structure installée (compatible avec la mise à jour automatique, voir crates\misty-core\src\systeme\update.rs) :
;   %LOCALAPPDATA%\Programs\Misty\Misty-<version>\Misty.exe
;   %LOCALAPPDATA%\Programs\Misty\Misty.exe : lanceur fixe (le même exe), visé par les raccourcis. Son chemin
;   ne change jamais, contrairement au dossier de la version : une épingle de la barre des tâches survit
;   aux mises à jour. Il démarre la version la plus récente (crates\misty-core\src\systeme\lanceur.rs).
; Pas de droits administrateur nécessaires. Les données (versions, mondes, profils) restent dans %LOCALAPPDATA%\Misty.
; Plus besoin de .NET : Misty.exe contient tout.

#ifndef Version
  #error "Version manquante : lancer installer\build.ps1"
#endif
#ifndef Source
  #define Source "..\publish\Misty-" + Version
#endif
#ifndef Sortie
  #define Sortie "..\publish"
#endif

#define Dossier "Misty-" + Version

; version numérique des propriétés du fichier (sans suffixe de pré-release : "0.3.8-beta.1" -> "0.3.8")
#ifndef NumVersion
  #define NumVersion Version
#endif

[Setup]
; Ne jamais changer cet ID : c'est lui qui relie les versions entre elles (et update.rs : CLE_DESINSTALLATION)
AppId={{8F4C2A61-3B7E-4D19-9A55-6C1E0F2B7D43}
AppName=Mistly Launcher
AppVersion={#Version}
AppVerName=Mistly Launcher {#Version}
AppPublisher=Mistly
AppPublisherURL=https://mistly.cc
AppSupportURL=https://github.com/Mistly-MC/Mistly-Launcher/issues
AppUpdatesURL=https://github.com/Mistly-MC/Mistly-Launcher/releases
VersionInfoVersion={#NumVersion}
DefaultDirName={localappdata}\Programs\Misty
DisableDirPage=auto
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#Sortie}
OutputBaseFilename=Mistly-Setup-Inno-{#Version}
SetupIconFile=..\crates\misty-app\assets\icone.ico
UninstallDisplayIcon={app}\{#Dossier}\Misty.exe
UninstallDisplayName=Mistly Launcher
UninstallFilesDir={app}\uninstall
LicenseFile=..\LICENSE.txt
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
fr.SupprimerDonnees=Supprimer aussi les données de Mistly Launcher (versions de Minecraft, profils, mondes) ?%n%nChoisis « Non » pour les garder en cas de réinstallation.
fr.Lancer=Lancer Mistly Launcher
en.SupprimerDonnees=Also delete Mistly Launcher's data (Minecraft versions, profiles, worlds)?%n%nChoose "No" to keep them for a later reinstall.
en.Lancer=Launch Mistly Launcher

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[InstallDelete]
; Versions précédentes (installées par le setup ou par la mise à jour automatique)
Type: filesandordirs; Name: "{app}\Misty-*"
; Raccourcis d'avant le changement de nom (Misty → Mistly)
Type: files; Name: "{autoprograms}\Misty.lnk"
Type: files; Name: "{autodesktop}\Misty.lnk"

[Files]
Source: "{#Source}\*"; DestDir: "{app}\{#Dossier}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Source}\Misty.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; AppUserModelID : le même que Misty (lanceur.rs : AUMID), pour un seul bouton dans la barre des tâches
Name: "{autoprograms}\Mistly"; Filename: "{app}\Misty.exe"; WorkingDir: "{app}"; AppUserModelID: "MistyMC.Misty"
Name: "{autodesktop}\Mistly"; Filename: "{app}\Misty.exe"; WorkingDir: "{app}"; AppUserModelID: "MistyMC.Misty"; Tasks: desktopicon

[Run]
Filename: "{app}\{#Dossier}\Misty.exe"; Description: "{cm:Lancer}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Les mises à jour automatiques créent des dossiers Misty-<version> inconnus de l'installateur
Type: filesandordirs; Name: "{app}"

[Code]
{ Désinstallation : les données de jeu ne sont supprimées que si l'utilisateur le demande }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usPostUninstall) and not UninstallSilent then
    if MsgBox(CustomMessage('SupprimerDonnees'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
      DelTree(ExpandConstant('{localappdata}\Misty'), True, True, True);
end;
