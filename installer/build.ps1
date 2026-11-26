#  ███╗   ███╗██╗███████╗████████╗██╗     ██╗   ██╗
#  ████╗ ████║██║██╔════╝╚══██╔══╝██║     ╚██╗ ██╔╝
#  ██╔████╔██║██║███████╗   ██║   ██║      ╚████╔╝
#  ██║╚██╔╝██║██║╚════██║   ██║   ██║       ╚██╔╝
#  ██║ ╚═╝ ██║██║███████║   ██║   ███████╗   ██║
#  ╚═╝     ╚═╝╚═╝╚══════╝   ╚═╝   ╚══════╝   ╚═╝
#
#  Mistly Launcher · https://mistly.cc
#  Code : https://github.com/Mistly-MC/Mistly-Launcher
#  Discord : https://discord.gg/rdQJUgvUh3
#  Misty License 1.0 : free to use, modify and share, not to sell (LICENSE.txt)
#
<#
.SYNOPSIS
    Prépare une release de Mistly Launcher : exe, zip (portable + mise à jour automatique), installateur.

.DESCRIPTION
    Produit dans publish\ :
      - Misty-<version>\            l'application (un seul fichier : Misty.exe)
      - Misty-<version>.zip         version portable, à joindre aussi à la release GitHub
                                    (c'est ce zip que télécharge la mise à jour automatique)
      - Mistly-Setup-<version>.exe  l'installateur (crates\misty-setup, en Rust), qui contient
                                    Misty.exe compressé

    La version est lue dans Cargo.toml à la racine ([workspace.package], seule source de la version).

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File installer\build.ps1
#>
param(
    # Ne génère que le dossier et le zip
    [switch]$SansInstallateur
)

$ErrorActionPreference = "Stop"
$racine = Split-Path -Parent $PSScriptRoot
# Workspace Cargo à la racine du repo (crates/misty-app produit Misty.exe)
$projet = $racine

# ── Version ──
# Cargo.toml est la seule source de la version (app_info.rs la lit avec env!("CARGO_PKG_VERSION"))
$cargo = Get-Content (Join-Path $projet "Cargo.toml") -Raw
if ($cargo -notmatch '(?m)^version = "([^"]+)"') { throw "Version introuvable dans Cargo.toml" }
$version = $Matches[1]
Write-Host "Mistly Launcher $version" -ForegroundColor Cyan

$publish = Join-Path $racine "publish"
$dossier = Join-Path $publish "Misty-$version"
$zip = Join-Path $publish "Misty-$version.zip"

# Mistly Client (le mod en jeu) n'est plus dans l'exe : le launcher le télécharge depuis les
# releases GitHub de Misty-Client. Publier une release du mod là-bas avant celle du launcher.

# ── Compilation (release : optimisée, sans console) ──
Push-Location $projet
try {
    cargo build --release -p misty
    if ($LASTEXITCODE -ne 0) { throw "cargo build a échoué" }
} finally {
    Pop-Location
}

# ── Signature : certificat de signature de code « CN=ENDERR.WIN » du magasin de l'utilisateur ──
# (auto-signé : Windows affiche l'éditeur sur les PC qui font confiance au certificat ; ailleurs,
# SmartScreen reste prudent tant que l'exe n'est pas signé par une autorité reconnue)
$certificat = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert -ErrorAction SilentlyContinue |
    Where-Object { $_.Subject -eq "CN=ENDERR.WIN" -and $_.NotAfter -gt (Get-Date) } |
    Sort-Object NotAfter -Descending | Select-Object -First 1
function Signer([string]$Fichier) {
    if (-not $certificat) { Write-Host "Pas de certificat CN=ENDERR.WIN : $Fichier n'est pas signé" -ForegroundColor Yellow; return }
    $null = Set-AuthenticodeSignature -FilePath $Fichier -Certificate $certificat -HashAlgorithm SHA256 -TimestampServer "http://timestamp.digicert.com"
    # un certificat auto-signé donne le statut « UnknownError » (racine non approuvée) : on vérifie
    # seulement que la signature est bien posée par ce certificat
    $v = Get-AuthenticodeSignature -FilePath $Fichier
    if ($v.SignerCertificate.Thumbprint -ne $certificat.Thumbprint) { throw "Signature de $Fichier impossible : $($v.StatusMessage)" }
    Write-Host "Signé : $Fichier (horodaté : $([bool]$v.TimeStamperCertificate))" -ForegroundColor Green
}
Signer (Join-Path $projet "target\release\Misty.exe")

# ── Dossier de l'application : tout est intégré à l'exe (polices, traductions, images) ──
if (Test-Path $dossier) { Remove-Item $dossier -Recurse -Force }
New-Item -ItemType Directory -Force $dossier | Out-Null
Copy-Item (Join-Path $projet "target\release\Misty.exe") $dossier
Copy-Item (Join-Path $racine "LICENSE.txt") $dossier

# ── Zip (le dossier Misty-<version> doit être à la racine du zip, voir services\update.rs) ──
if (Test-Path $zip) { Remove-Item $zip -Force }
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($dossier, $zip, [System.IO.Compression.CompressionLevel]::Optimal, $true)
Write-Host "Zip : $zip" -ForegroundColor Green

if ($SansInstallateur) { return }

# ── Installateur (crates\misty-setup, en Rust) : Misty.exe y est compressé ──
$env:MISTY_SETUP_EXE = Join-Path $projet "target\release\Misty.exe"
Push-Location $projet
try {
    cargo build --release -p misty-setup
    if ($LASTEXITCODE -ne 0) { throw "cargo build (misty-setup) a échoué" }
} finally {
    Pop-Location
    Remove-Item Env:\MISTY_SETUP_EXE
}
$setup = Join-Path $publish "Mistly-Setup-$version.exe"
Copy-Item (Join-Path $projet "target\release\Mistly-Setup.exe") $setup -Force
Signer $setup
Write-Host "Installateur : $setup" -ForegroundColor Green
