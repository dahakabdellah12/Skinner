<#
.SYNOPSIS
    Builds and publishes a BrawlCraft release end to end.

.DESCRIPTION
    1. Reads the version from the App project file.
    2. Builds the portable package via scripts/release.ps1 (frontend + .NET + ZIP + SHA256SUMS).
    3. Commits the source change and pushes the private source repository with a v<version> tag.
    4. Uploads the package and checksums to a GitHub Release on the public repository.
    5. Refreshes public/downloads/latest.json (used by the built-in updater).
    6. Updates version, size and checksum references in the public README and prepends a
       CHANGELOG entry.
    7. Commits and pushes the public repository.

    The public repository intentionally contains no application source code. Release
    packages live on the GitHub Release (and are referenced by latest.json), so the
    auto-generated "Source code" archives of the public repository only ever contain
    documentation and the logo.

.PARAMETER Notes
    Release notes in Markdown, used for the GitHub Release and the CHANGELOG.

.PARAMETER NotesFile
    Path to a Markdown file with the release notes. Takes precedence over -Notes.

.PARAMETER PublicRepoPath
    Local working copy of the public repository (cloned automatically when missing).

.PARAMETER PublicRepo
    Public repository in owner/name form.

.PARAMETER SkipBuild
    Reuse the artifacts already present in .\release.

.PARAMETER SkipSourcePush
    Do not commit or push the private source repository.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\publish.ps1 -NotesFile .\notes.md

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\publish.ps1 -Notes "Fixes the weapon skin picker."
#>

param(
    [string]$Notes = '',
    [string]$NotesFile = '',
    [string]$PublicRepoPath = (Join-Path $env:TEMP 'opencode\brawlcraft-public'),
    [string]$PublicRepo = 'dahakabdellah12/BrawlCraft',
    [switch]$SkipBuild,
    [switch]$SkipSourcePush
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$gitName = 'dahakabdellah12'
$gitEmail = 'dahakabdellah12@users.noreply.github.com'

function Invoke-Git {
    param([string]$RepoPath, [string[]]$Arguments)
    $output = & git -C $RepoPath @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed in $RepoPath`n$output"
    }
    return $output
}

Push-Location $root
try {
    # ---- version -------------------------------------------------------------
    $csproj = Join-Path $root 'src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj'
    [xml]$project = Get-Content $csproj
    $version = ($project.Project.PropertyGroup | Where-Object { $_.Version } | Select-Object -First 1).Version
    if (-not $version) { throw "Could not read <Version> from $csproj" }

    $tag = "v$version"
    $assetName = "BrawlCraft-v$version-Windows.zip"
    $releaseTitle = "BrawlCraft v$version"
    Write-Host "==> Publishing BrawlCraft $version" -ForegroundColor Cyan

    if ($NotesFile) {
        if (-not (Test-Path $NotesFile)) { throw "Notes file not found: $NotesFile" }
        $Notes = Get-Content $NotesFile -Raw
    }
    if ([string]::IsNullOrWhiteSpace($Notes)) { $Notes = "BrawlCraft v$version." }
    $Notes = $Notes.Trim()

    # ---- build ---------------------------------------------------------------
    if (-not $SkipBuild) {
        & (Join-Path $PSScriptRoot 'release.ps1')
        if ($LASTEXITCODE -ne 0) { throw "Release build failed with exit code $LASTEXITCODE" }
    }

    $releaseDir = Join-Path $root 'release'
    $zipPath = Join-Path $releaseDir $assetName
    $sumsPath = Join-Path $releaseDir 'SHA256SUMS.txt'
    if (-not (Test-Path $zipPath)) { throw "Missing artifact: $zipPath" }

    $sha256 = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
    $size = (Get-Item $zipPath).Length
    $sizeMb = '{0:F1}' -f ($size / 1MB)
    $publishedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $downloadUrl = "https://github.com/$PublicRepo/releases/download/$tag/$assetName"
    Write-Host "    package : $assetName ($size bytes)"
    Write-Host "    sha256  : $sha256"

    # ---- private source repository ------------------------------------------
    if (-not $SkipSourcePush) {
        Write-Host '==> Updating private source repository'
        $dirty = (& git -C $root status --porcelain) -join "`n"
        if ($dirty.Trim()) {
            & git -C $root add -A
            & git -C $root -c "user.name=$gitName" -c "user.email=$gitEmail" commit -m "Release $tag" | Out-Null
        }
        $existingTag = (& git -C $root tag --list $tag) -join ''
        if (-not $existingTag) {
            & git -C $root tag -a $tag -m "BrawlCraft $tag"
        }
        & git -C $root push origin master | Out-Null
        & git -C $root push -f origin $tag | Out-Null
        Write-Host "    pushed master and $tag"
    }

    # ---- public repository ---------------------------------------------------
    Write-Host '==> Preparing public repository'
    if (-not (Test-Path (Join-Path $PublicRepoPath '.git'))) {
        & git clone "https://github.com/$PublicRepo.git" $PublicRepoPath | Out-Null
    }
    & git -C $PublicRepoPath fetch origin master | Out-Null
    & git -C $PublicRepoPath reset --hard origin/master | Out-Null

    $downloadsDir = Join-Path $PublicRepoPath 'downloads'
    New-Item -ItemType Directory -Force -Path $downloadsDir | Out-Null
    Get-ChildItem $downloadsDir -Filter 'BrawlCraft-*.zip' -File -ErrorAction SilentlyContinue | Remove-Item -Force
    Copy-Item $sumsPath (Join-Path $downloadsDir 'SHA256SUMS.txt') -Force

    $readmePath = Join-Path $PublicRepoPath 'README.md'
    $readme = Get-Content $readmePath -Raw
    $oldVersion = ''
    foreach ($pattern in @('badge/version-(\d+\.\d+\.\d+)', 'Latest version\*\* \| v(\d+\.\d+\.\d+)', 'v(\d+\.\d+\.\d+)')) {
        $match = [regex]::Match($readme, $pattern)
        if ($match.Success) { $oldVersion = $match.Groups[1].Value; break }
    }
    if (-not $oldVersion) { throw 'Could not detect the previous version in the public README.' }
    if ($oldVersion -eq $version) {
        Write-Host "    public README already references $version"
    }
    else {
        $readme = $readme.Replace($oldVersion, $version)
        Write-Host "    README: $oldVersion -> $version"
    }

    # Refresh size, checksum and download link
    $readme = [regex]::Replace($readme, '\| \*\*Size\*\* \| [^|\r\n]+ \|', "| **Size** | $sizeMb MB |")
    $readme = [regex]::Replace($readme, '\| \*\*SHA-256\*\* \| `[0-9a-fA-F]{64}` \|', "| **SHA-256** | ``$sha256`` |")
    $readme = [regex]::Replace($readme, 'releases/download/v[\d.]+/', "releases/download/$tag/")
    $readme = [regex]::Replace($readme, 'raw\.githubusercontent\.com/' + [regex]::Escape($PublicRepo) + '/master/downloads/BrawlCraft-v[\d.]+-Windows\.zip', $downloadUrl)
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($readmePath, $readme, $utf8)

    # latest.json — primary channel of the built-in updater
    $manifest = [ordered]@{
        version     = $version
        title       = $releaseTitle
        notes       = $Notes
        assetName   = $assetName
        downloadUrl = $downloadUrl
        size        = $size
        sha256      = $sha256
        publishedAt = $publishedAt
    }
    $manifestJson = ($manifest | ConvertTo-Json -Depth 4)
    [System.IO.File]::WriteAllText((Join-Path $downloadsDir 'latest.json'), $manifestJson + "`n", $utf8)

    # CHANGELOG entry
    $changelogPath = Join-Path $PublicRepoPath 'CHANGELOG.md'
    $entry = "## $tag — $((Get-Date).ToString('d MMMM yyyy'))`n`n$Notes`n`n"
    if (Test-Path $changelogPath) {
        $changelog = Get-Content $changelogPath -Raw
        if ($changelog -notmatch "(?m)^## $([regex]::Escape($tag))\b") {
            $changelog = [regex]::Replace($changelog, '(?m)^## ', ($entry + '## '), 1)
            [System.IO.File]::WriteAllText($changelogPath, $changelog, $utf8)
        }
    }

    # ---- GitHub Release ------------------------------------------------------
    Write-Host '==> Publishing GitHub Release'
    $notesFile = Join-Path $env:TEMP "brawlcraft-notes-$version.md"
    [System.IO.File]::WriteAllText($notesFile, $Notes, $utf8)

    $releaseExists = $false
    & gh release view $tag --repo $PublicRepo 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { $releaseExists = $true }

    if ($releaseExists) {
        & gh release edit $tag --repo $PublicRepo --title $releaseTitle --notes-file $notesFile --latest | Out-Null
        Write-Host "    updated existing release $tag"
    }
    else {
        & gh release create $tag --repo $PublicRepo --title $releaseTitle --notes-file $notesFile --target master --latest | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "gh release create failed" }
        Write-Host "    created release $tag"
    }
    & gh release upload $tag $zipPath $sumsPath --repo $PublicRepo --clobber | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "gh release upload failed" }

    # ---- commit public repository -------------------------------------------
    Write-Host '==> Pushing public repository'
    $change = (& git -C $PublicRepoPath status --porcelain) -join "`n"
    if ($change.Trim()) {
        & git -C $PublicRepoPath add -A
        & git -C $PublicRepoPath -c "user.name=$gitName" -c "user.email=$gitEmail" commit -m "Release $tag" | Out-Null
        & git -C $PublicRepoPath push origin master | Out-Null
        Write-Host '    committed and pushed'
    }
    else {
        Write-Host '    nothing to commit'
    }

    Write-Host ''
    Write-Host "Published $releaseTitle" -ForegroundColor Green
    Write-Host "  release : https://github.com/$PublicRepo/releases/tag/$tag"
    Write-Host "  package : $assetName ($sizeMb MB)"
    Write-Host "  sha256  : $sha256"
}
finally {
    Pop-Location
}
