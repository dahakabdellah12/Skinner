<#
.SYNOPSIS
    Builds the BrawlCraft 3.x release artifacts (portable zip + SHA256SUMS.txt).

.DESCRIPTION
    - Reads the authoritative version from the App project file.
    - Builds the React frontend.
    - Publishes the .NET application to .\publish (win-x64, framework-dependent).
    - Creates .\release\BrawlCraft-v<version>-Windows.zip and SHA256SUMS.txt.
    - Optionally compiles the Inno Setup installer when ISCC.exe is available.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\release.ps1
#>

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $csproj = Join-Path $root 'src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj'
    [xml]$project = Get-Content $csproj
    $version = ($project.Project.PropertyGroup | Where-Object { $_.Version } | Select-Object -First 1).Version
    if (-not $version) { throw "Could not read <Version> from $csproj" }
    Write-Host "BrawlCraft release version: $version"

    $dotnet = 'C:\dotnet_sdk\dotnet.exe'
    if (-not (Test-Path $dotnet)) { $dotnet = 'dotnet' }

    Write-Host '==> Building frontend'
    Push-Location (Join-Path $root 'frontend')
    try {
        if (-not (Test-Path 'node_modules')) { npm install }
        npm run build
    }
    finally { Pop-Location }

    Write-Host '==> Publishing .NET application'
    $publishDir = Join-Path $root 'publish'
    if (Test-Path $publishDir) { Remove-Item $publishDir -Recurse -Force }
    & $dotnet publish (Join-Path $root 'src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj') `
        -c Release -r win-x64 --self-contained false -o $publishDir
    if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed with exit code $LASTEXITCODE" }

    Write-Host '==> Stripping debug symbols'
    Get-ChildItem -Path $publishDir -Recurse -Filter *.pdb -File | Remove-Item -Force

    Write-Host '==> Creating portable archive'
    $releaseDir = Join-Path $root 'release'
    New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
    $zipName = "BrawlCraft-v$version-Windows.zip"
    $zipPath = Join-Path $releaseDir $zipName
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
    Compress-Archive -Path (Join-Path $publishDir '*') -DestinationPath $zipPath -CompressionLevel Optimal

    Write-Host '==> Writing SHA256SUMS.txt'
    $hash = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
    $sumsPath = Join-Path $releaseDir 'SHA256SUMS.txt'
    [System.IO.File]::WriteAllText($sumsPath, "$hash  $zipName`n", (New-Object System.Text.UTF8Encoding($false)))

    $isccCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
    ) | Where-Object { $_ -and (Test-Path $_) }

    if ($isccCandidates.Count -gt 0) {
        Write-Host '==> Compiling installer'
        & $isccCandidates[0] (Join-Path $root 'installer.iss')
        if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }
    }
    else {
        Write-Warning 'Inno Setup (ISCC.exe) not found; skipped installer compilation.'
    }

    Write-Host ''
    Write-Host "Release artifacts written to $releaseDir"
    Get-ChildItem $releaseDir -Filter "*$version*" | Select-Object Name, Length
}
finally {
    Pop-Location
}
