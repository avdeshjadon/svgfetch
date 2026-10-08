#Requires -Version 5.1
#
# svgfetch — one-line installer for native Windows (PowerShell).
#
#   Set-ExecutionPolicy -Scope Process Bypass
#   irm https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.ps1 | iex
#
#   Pin a specific version:
#     $env:SVGFETCH_VERSION = 'v0.2.3'
#     irm .../install.ps1 | iex
#
# Downloads the official release binary (SHA-256 verified) and installs it
# to $env:USERPROFILE\.local\bin, then prints the PATH to add.

$ErrorActionPreference = 'Stop'
$repo   = 'avdeshjadon/svgfetch'
$bin    = 'svgfetch'
$version = if ($env:SVGFETCH_VERSION) { $env:SVGFETCH_VERSION } elseif ($env:GET_SVG_VERSION) { $env:GET_SVG_VERSION } else { 'latest' }

# --- platform --------------------------------------------------------------
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne 'AMD64') {
    throw "unsupported Windows CPU architecture: $arch (only x86_64 builds are published)"
}
$target = 'x86_64-pc-windows-msvc'
$ext = 'zip'

# --- resolve version --------------------------------------------------------
if ($version -eq 'latest') {
    Write-Host '> resolving latest release ...'
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
    $version = $release.tag_name
    if (-not $version) { throw 'could not determine the latest release tag' }
}

$baseUrl   = "https://github.com/$repo/releases/download/$version"
$artifact  = "$bin-$target.$ext"
$destDir   = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE '.local\bin' }

Write-Host "> downloading $artifact ($version) ..."
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "svgfetch.$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact"         -OutFile (Join-Path $tmp $artifact)
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact.sha256"  -OutFile (Join-Path $tmp "$artifact.sha256")
    } catch {
        $artifact = "get-svg-$target.$ext"
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact"         -OutFile (Join-Path $tmp $artifact)
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact.sha256"  -OutFile (Join-Path $tmp "$artifact.sha256")
    }

    $expected = (Get-Content -Raw (Join-Path $tmp "$artifact.sha256") -ErrorAction Stop).Trim().Split(' ')[0]
    $hash = Get-FileHash -Algorithm SHA256 -Path (Join-Path $tmp $artifact)
    if ($hash.Hash -ne $expected.ToUpperInvariant()) {
        throw "checksum mismatch for $artifact (expected $expected, got $($hash.Hash))"
    }
    Write-Host "> checksum verified"

    $extract = Join-Path $tmp 'extracted'
    Expand-Archive -Path (Join-Path $tmp $artifact) -DestinationPath $extract -Force

    $primary = Get-ChildItem -Path $extract -Recurse -Filter "*.exe" | Select-Object -First 1
    if (-not $primary) { throw "archive did not contain any executable binary" }

    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    foreach ($exe in @("$bin.exe", 'svg-fetch.exe')) {
        $binary = Get-ChildItem -Path $extract -Recurse -Filter $exe | Select-Object -First 1
        if (-not $binary) {
            Copy-Item -Path $primary.FullName -Destination (Join-Path $destDir $exe) -Force
        } else {
            Copy-Item -Path $binary.FullName  -Destination (Join-Path $destDir $exe) -Force
        }
        Write-Host "> installed $(Join-Path $destDir $exe) ($version)"
    }
} finally {
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$destDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$destDir", 'User')
    Write-Host "> added $destDir to your User PATH"
}

Write-Host "> done. Restart your terminal, then try:  svgfetch --help"
Write-Host $destDir