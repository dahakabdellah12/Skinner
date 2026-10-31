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
$utf8 = New-Object System.Text.UTF8Encoding($false)

# Native tools write progress information to stderr. With ErrorActionPreference = Stop
# PowerShell turns that into a terminating error, so native calls are wrapped here.
function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$Exe,
        [string[]]$Arguments = @(),
        [switch]$Quiet
    )
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        if ($Quiet) {
            & $Exe @Arguments 2>&1 | Out-Null
            return $LASTEXITCODE
        }
        $output = & $Exe @Arguments 2>&1
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    if ($code -ne 0) {
        throw ("{0} {1} failed (exit {2})`n{3}" -f $Exe, ($Arguments -join ' '), $code, ($output -join "`n"))
    }
    return $output
}

function Invoke-Git {
    param([string]$RepoPath, [string[]]$Arguments = @())
    return Invoke-Native -Exe 'git' -Arguments (@('-C', $RepoPath) + $Arguments)
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
        $Notes = Get-Content $NotesFile -Raw -Encoding UTF8
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
    $sizeMb = [string]::Format([System.Globalization.CultureInfo]::InvariantCulture, '{0:F1}', $size / 1MB)
    $publishedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $downloadUrl = "https://github.com/$PublicRepo/releases/download/$tag/$assetName"
    Write-Host "    package : $assetName ($size bytes)"
    Write-Host "    sha256  : $sha256"

    # ---- private source repository ------------------------------------------
    if (-not $SkipSourcePush) {
        Write-Host '==> Updating private source repository'
        $dirty = (Invoke-Git -RepoPath $root -Arguments @('status', '--porcelain')) -join "`n"
        if ($dirty.Trim()) {
            Invoke-Git -RepoPath $root -Arguments @('add', '-A') | Out-Null
            Invoke-Git -RepoPath $root -Arguments @('-c', "user.name=$gitName", '-c', "user.email=$gitEmail", 'commit', '-m', "Release $tag") | Out-Null
        }
        $existingTag = (Invoke-Git -RepoPath $root -Arguments @('tag', '--list', $tag)) -join ''
        if (-not $existingTag) {
            Invoke-Git -RepoPath $root -Arguments @('tag', '-a', $tag, '-m', "BrawlCraft $tag") | Out-Null
        }
        Invoke-Git -RepoPath $root -Arguments @('push', 'origin', 'master') | Out-Null
        Invoke-Git -RepoPath $root -Arguments @('push', '-f', 'origin', $tag) | Out-Null
        Write-Host "    pushed master and $tag"
    }

    # ---- public repository ---------------------------------------------------
    Write-Host '==> Preparing public repository'
    if (-not (Test-Path (Join-Path $PublicRepoPath '.git'))) {
        Invoke-Native -Exe 'git' -Arguments @('clone', "https://github.com/$PublicRepo.git", $PublicRepoPath) | Out-Null
    }
    Invoke-Git -RepoPath $PublicRepoPath -Arguments @('fetch', 'origin', 'master') | Out-Null
    Invoke-Git -RepoPath $PublicRepoPath -Arguments @('reset', '--hard', 'origin/master') | Out-Null

    $downloadsDir = Join-Path $PublicRepoPath 'downloads'
    New-Item -ItemType Directory -Force -Path $downloadsDir | Out-Null
    Get-ChildItem $downloadsDir -Filter 'BrawlCraft-*.zip' -File -ErrorAction SilentlyContinue | Remove-Item -Force
    Copy-Item $sumsPath (Join-Path $downloadsDir 'SHA256SUMS.txt') -Force

    $readmePath = Join-Path $PublicRepoPath 'README.md'
    $readme = Get-Content $readmePath -Raw -Encoding UTF8
    $oldVersion = ''
    foreach ($pattern in @('badge/version-(\d+\.\d+\.\d+)', 'Latest version\*\* \| v(\d+\.\d+\.\d+)', 'v(\d+\.\d+\.\d+)')) {
        $match = [regex]::Match($readme, $pattern)
        if ($match.Success) { $oldVersion = $match.Groups[1].Value; break }
    }
    if (-not $oldVersion) { throw 'Could not detect the previous version in the public README.' }
    if ($oldVersion -ne $version) {
        $readme = $readme.Replace($oldVersion, $version)
        Write-Host "    README: $oldVersion -> $version"
    }

    $readme = [regex]::Replace($readme, '\| \*\*Size\*\* \| [^|\r\n]+ \|', "| **Size** | $sizeMb MB |")
    $readme = [regex]::Replace($readme, '\| \*\*SHA-256\*\* \| `[0-9a-fA-F]{64}` \|', "| **SHA-256** | ``$sha256`` |")
    $readme = [regex]::Replace($readme, 'releases/download/v[\d.]+/', "releases/download/$tag/")
    [System.IO.File]::WriteAllText($readmePath, $readme, $utf8)

    # latest.json - primary channel of the built-in updater
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
    if (Test-Path $changelogPath) {
        $changelog = Get-Content $changelogPath -Raw -Encoding UTF8
        if ($changelog -notmatch "(?m)^## $([regex]::Escape($tag))\b") {
            $entry = "## $tag - $((Get-Date).ToString('d MMMM yyyy'))`n`n$Notes`n`n"
            $changelog = [regex]::Replace($changelog, '(?m)^## ', ($entry + '## '), 1)
            [System.IO.File]::WriteAllText($changelogPath, $changelog, $utf8)
        }
    }

    # ---- GitHub Release ------------------------------------------------------
    Write-Host '==> Publishing GitHub Release'
    $notesPath = Join-Path $env:TEMP "brawlcraft-notes-$version.md"
    [System.IO.File]::WriteAllText($notesPath, $Notes, $utf8)

    $exists = (Invoke-Native -Exe 'gh' -Arguments @('release', 'view', $tag, '--repo', $PublicRepo) -Quiet) -eq 0
    if ($exists) {
        Invoke-Native -Exe 'gh' -Arguments @('release', 'edit', $tag, '--repo', $PublicRepo, '--title', $releaseTitle, '--notes-file', $notesPath, '--latest') | Out-Null
        Write-Host "    updated existing release $tag"
    }
    else {
        Invoke-Native -Exe 'gh' -Arguments @('release', 'create', $tag, '--repo', $PublicRepo, '--title', $releaseTitle, '--notes-file', $notesPath, '--target', 'master', '--latest') | Out-Null
        Write-Host "    created release $tag"
    }
    Invoke-Native -Exe 'gh' -Arguments @('release', 'upload', $tag, $zipPath, $sumsPath, '--repo', $PublicRepo, '--clobber') | Out-Null

    # ---- commit public repository -------------------------------------------
    Write-Host '==> Pushing public repository'
    $change = (Invoke-Git -RepoPath $PublicRepoPath -Arguments @('status', '--porcelain')) -join "`n"
    if ($change.Trim()) {
        Invoke-Git -RepoPath $PublicRepoPath -Arguments @('add', '-A') | Out-Null
        Invoke-Git -RepoPath $PublicRepoPath -Arguments @('-c', "user.name=$gitName", '-c', "user.email=$gitEmail", 'commit', '-m', "Release $tag") | Out-Null
        Invoke-Git -RepoPath $PublicRepoPath -Arguments @('push', 'origin', 'master') | Out-Null
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
