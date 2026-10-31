<#
.SYNOPSIS
    Builds and publishes a BrawlCraft release end to end.

.DESCRIPTION
    1. Bumps the version across the project when -Version is supplied, then reads the
       version from the App project file.
    2. Builds the portable package via scripts/release.ps1 (frontend + .NET + ZIP + SHA256SUMS).
    3. Commits the source change and pushes the private source repository with a v<version> tag.
    4. Creates a GitHub Release on the public repository and uploads the portable ZIP plus
       SHA256SUMS.txt (this is what users download).
    5. Refreshes public/downloads/latest.json so older clients still find the same package.
    6. Updates version, size and checksum references in the public README and prepends a
       CHANGELOG entry.
    7. Commits and pushes the public repository (documentation + latest.json only).

    The public repository contains no application source. GitHub still attaches a
    "Source code" archive to every Release; that archive is only README/license/changelog.

.PARAMETER Notes
    Release notes in Markdown, used for the CHANGELOG and downloads/latest.json.

.PARAMETER NotesFile
    Path to a Markdown file with the release notes. Takes precedence over -Notes.

.PARAMETER Version
    New version in 3.1.4 form. Updates the App .csproj (Version, AssemblyVersion and
    FileVersion), frontend/package.json, frontend/package-lock.json, installer.iss and the
    private README before building.

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

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\publish.ps1 -Version 3.1.3 -NotesFile .\notes.md
#>

param(
    [string]$Notes = '',
    [string]$NotesFile = '',
    [string]$Version = '',
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
    # ---- optional version bump ----------------------------------------------
    $csproj = Join-Path $root 'src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj'
    if ($Version) {
        if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw '-Version must use the 3.1.4 format.' }
        Write-Host "==> Bumping version to $Version" -ForegroundColor Cyan

        $text = Get-Content $csproj -Raw -Encoding UTF8
        $text = [regex]::Replace($text, '<Version>[^<]*</Version>', "<Version>$Version</Version>")
        $text = [regex]::Replace($text, '<AssemblyVersion>[^<]*</AssemblyVersion>', "<AssemblyVersion>$Version.0</AssemblyVersion>")
        $text = [regex]::Replace($text, '<FileVersion>[^<]*</FileVersion>', "<FileVersion>$Version.0</FileVersion>")
        [System.IO.File]::WriteAllText($csproj, $text, $utf8)

        $packageJson = Join-Path $root 'frontend\package.json'
        $pkg = Get-Content $packageJson -Raw -Encoding UTF8
        $pkg = ([regex]'"version"\s*:\s*"[^"]*"').Replace($pkg, "`"version`": `"$Version`"", 1)
        [System.IO.File]::WriteAllText($packageJson, $pkg, $utf8)

        $packageLock = Join-Path $root 'frontend\package-lock.json'
        $lock = Get-Content $packageLock -Raw -Encoding UTF8
        $lock = ([regex]'"version"\s*:\s*"[^"]*"').Replace($lock, "`"version`": `"$Version`"", 2)
        [System.IO.File]::WriteAllText($packageLock, $lock, $utf8)

        $installer = Join-Path $root 'installer.iss'
        $iss = Get-Content $installer -Raw -Encoding UTF8
        $iss = [regex]::Replace($iss, '#define MyAppVersion "[^"]*"', "#define MyAppVersion `"$Version`"")
        [System.IO.File]::WriteAllText($installer, $iss, $utf8)

        $devReadme = Join-Path $root 'README.md'
        $devText = Get-Content $devReadme -Raw -Encoding UTF8
        $devText = [regex]::Replace($devText, 'Current release: \*\*v[0-9.]+\*\*', "Current release: **v$Version**")
        [System.IO.File]::WriteAllText($devReadme, $devText, $utf8)
    }

    # ---- version -------------------------------------------------------------
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
    Write-Host "    binaries stay on GitHub Releases; latest.json points at $tag"

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
    $sumsUrl = "https://github.com/$PublicRepo/releases/download/$tag/SHA256SUMS.txt"
    $readme = [regex]::Replace($readme, 'https://(?:raw\.githubusercontent\.com|github\.com)/' + [regex]::Escape($PublicRepo) + '[^\s\)"]*(?:downloads/|releases/download/[^/]+/)BrawlCraft-v[\d.]+-Windows\.zip', $downloadUrl)
    $readme = [regex]::Replace($readme, 'https://(?:raw\.githubusercontent\.com|github\.com)/' + [regex]::Escape($PublicRepo) + '[^\s\)"]*(?:downloads/|releases/download/[^/]+/)SHA256SUMS\.txt', $sumsUrl)
    $readme = [regex]::Replace($readme, '\(`?downloads/SHA256SUMS\.txt`?\)', "($sumsUrl)")
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
            # The release notes usually start with their own title heading. Drop leading
            # headings and rules so the CHANGELOG entry does not repeat the version title.
            $changelogNotes = $Notes.TrimStart()
            while ($true) {
                $lead = [regex]::Match($changelogNotes, '^(?:#{1,6}[^\r\n]*(?:\r?\n)+|-{3,}[ \t]*(?:\r?\n)+)')
                if (-not $lead.Success) { break }
                $changelogNotes = $changelogNotes.Substring($lead.Length).TrimStart()
            }
            if (-not $changelogNotes) { $changelogNotes = $Notes }
            $dash = [string][char]0x2014
            $date = (Get-Date).ToString('d MMMM yyyy', [System.Globalization.CultureInfo]::InvariantCulture)
            $entry = "## $tag $dash $date`n`n$changelogNotes`n`n"
            $changelog = [regex]::Replace($changelog, '(?m)^## ', ($entry + '## '), 1)
            [System.IO.File]::WriteAllText($changelogPath, $changelog, $utf8)
        }
    }

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

    Write-Host "==> Publishing GitHub Release $tag"
    $existingReleaseCode = Invoke-Native -Exe 'gh' -Arguments @('release', 'view', $tag, '-R', $PublicRepo, '--json', 'tagName') -Quiet
    if ($existingReleaseCode -eq 0) {
        Invoke-Native -Exe 'gh' -Arguments @('release', 'delete', $tag, '-R', $PublicRepo, '--yes') | Out-Null
    }
    $notesPath = Join-Path ([System.IO.Path]::GetTempPath()) ("brawlcraft-release-notes-" + [guid]::NewGuid().ToString('N') + '.md')
    try {
        [System.IO.File]::WriteAllText($notesPath, $Notes, $utf8)
        Invoke-Native -Exe 'gh' -Arguments @(
            'release', 'create', $tag,
            $zipPath,
            $sumsPath,
            '-R', $PublicRepo,
            '--title', $releaseTitle,
            '--notes-file', $notesPath,
            '--latest'
        ) | Out-Null
    }
    finally {
        if (Test-Path $notesPath) { Remove-Item $notesPath -Force }
    }
    Write-Host "    https://github.com/$PublicRepo/releases/tag/$tag"

    Write-Host ''
    Write-Host "Published $releaseTitle" -ForegroundColor Green
    Write-Host "  page    : https://github.com/$PublicRepo/releases/latest"
    Write-Host "  package : $downloadUrl"
    Write-Host "  sha256  : $sha256"
}
finally {
    Pop-Location
}
