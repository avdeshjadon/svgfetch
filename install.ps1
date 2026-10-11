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
#
# Presentation only: the animated progress is cosmetic and is disabled when
# stderr is not a TTY, when NO_COLOR is set, or when TERM=dumb. The install
# directory is still printed as the final line.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repo    = 'avdeshjadon/svgfetch'
$bin     = 'svgfetch'
$version = if ($env:SVGFETCH_VERSION) { $env:SVGFETCH_VERSION } elseif ($env:GET_SVG_VERSION) { $env:GET_SVG_VERSION } else { 'latest' }

# --- glyphs (built from code points so the file stays ASCII / PS 5.1 safe) --
$ESC    = [char]27
$G_CHK  = [char]0x2713   # check mark
$G_DASH = [char]0x2013   # en dash
$G_DOT  = [char]0x25CF   # bullet
$G_TREE = [char]0x23BF   # branch
$G_ELL  = [char]0x2026   # ellipsis
$G_TL   = [char]0x250C
$G_TR   = [char]0x2510
$G_BL   = [char]0x2514
$G_BR   = [char]0x2518
$G_HB   = [char]0x2500
$G_VB   = [char]0x2502

# --- terminal capability detection -----------------------------------------
# All progress goes to stderr, so that is what we probe.
$script:UseColor = $false
$script:UseAnim  = $false
try {
    $errTty = -not [Console]::IsErrorRedirected
    if ($errTty -and -not $env:NO_COLOR -and $env:TERM -ne 'dumb') {
        $script:UseColor = $true
        $script:UseAnim  = $true
    }
} catch {}

# Enable ANSI (virtual terminal) processing on the legacy Windows console.
function Enable-VT {
    if (-not $script:UseColor) { return }
    try {
        if (-not ('SvgFetch.VT' -as [type])) {
            Add-Type -Namespace SvgFetch -Name VT -MemberDefinition @'
[DllImport("kernel32.dll", SetLastError=true)]
public static extern IntPtr GetStdHandle(int nStdHandle);
[DllImport("kernel32.dll", SetLastError=true)]
public static extern bool GetConsoleMode(IntPtr hConsoleHandle, out uint lpMode);
[DllImport("kernel32.dll", SetLastError=true)]
public static extern bool SetConsoleMode(IntPtr hConsoleHandle, uint dwMode);
'@
        }
        foreach ($std in @(-12, -11)) {
            $h = [SvgFetch.VT]::GetStdHandle($std)
            [uint32]$mode = 0
            if ([SvgFetch.VT]::GetConsoleMode($h, [ref]$mode)) {
                [void][SvgFetch.VT]::SetConsoleMode($h, $mode -bor 0x0004)
            }
        }
    } catch {}
}

# --- colour helpers --------------------------------------------------------
function Write-Err([string]$s) {
    try { [Console]::Error.Write($s) } catch {}
}
function Paint([string]$sgr, [string]$text) {
    if ($script:UseColor) { return "${ESC}[${sgr}m${text}${ESC}[0m" }
    return $text
}
function Accent([string]$t) { Paint '1;38;5;209' $t }
function Green([string]$t)  { Paint '32' $t }
function Yellow([string]$t) { Paint '33' $t }
function Red([string]$t)    { Paint '31' $t }
function Dim([string]$t)    { Paint '2' $t }

# --- cursor ----------------------------------------------------------------
$script:CursorHidden = $false
function Hide-Cursor {
    if (-not $script:UseAnim -or $script:CursorHidden) { return }
    $script:CursorHidden = $true
    Write-Err "$ESC[?25l"
}
function Show-Cursor {
    if (-not $script:CursorHidden) { return }
    $script:CursorHidden = $false
    Write-Err "`r$ESC[2K$ESC[?25h"
}

# --- animation -------------------------------------------------------------
$TICK_MS = 70
$MIN_STEP_MS = 450

$script:SpinSW = $null
$script:SpinRS = $null
$script:SpinPS = $null

function Start-Spin([string]$Label) {
    if (-not $script:UseAnim) { return }
    Hide-Cursor
    $script:SpinSW = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $rs = [runspacefactory]::CreateRunspace()
        $rs.ApartmentState = 'MTA'
        $rs.ThreadOptions = 'ReuseThread'
        $rs.Open()
        $ps = [powershell]::Create()
        $ps.Runspace = $rs
        [void]$ps.AddScript({
            param($Label, $Color, $Esc, $Ell)
            function P([string]$sgr, [string]$t) {
                if ($Color) { return "${Esc}[${sgr}m${t}${Esc}[0m" }
                return $t
            }
            $glyphs = @(
                [char]0x00B7, [char]0x2722, [char]0x2733, [char]0x2736, [char]0x273B,
                [char]0x273D, [char]0x273B, [char]0x2736, [char]0x2733, [char]0x2722
            )
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            $i = 0
            while ($true) {
                $gi = [int](($i / 2) % $glyphs.Length)
                $g = $glyphs[$gi]
                $p = $i % 18
                if ($p -lt 10) { $pos = $p } else { $pos = 18 - $p }
                $sc = ''
                for ($c = 0; $c -lt 10; $c++) {
                    if ($c -eq $pos) { $sgr = '1;38;5;209'; $cell = [char]0x25A0 }
                    elseif ($c -eq ($pos - 1)) { $sgr = '38;5;173'; $cell = [char]0x25AA }
                    elseif ($c -eq ($pos - 2)) { $sgr = '38;5;130'; $cell = [char]0x25AA }
                    else { $sgr = '38;5;238'; $cell = [char]0x25AA }
                    $sc += (P $sgr $cell)
                }
                $secs = '{0:0.0}s' -f $sw.Elapsed.TotalSeconds
                try {
                    [Console]::Error.Write("`r${Esc}[2K  " + (P '1;38;5;209' $g) + ' ' + (P '1;38;5;209' $Label) + $Ell + '  ' + $sc + '  ' + (P '2' $secs))
                } catch {}
                Start-Sleep -Milliseconds 70
                $i++
            }
        }).AddArgument($Label).AddArgument($script:UseColor).AddArgument($ESC).AddArgument($G_ELL)
        $script:SpinHandle = $ps.BeginInvoke()
        $script:SpinRS = $rs
        $script:SpinPS = $ps
    } catch {
        $script:UseAnim = $false
        if ($script:SpinSW) { $script:SpinSW.Stop(); $script:SpinSW = $null }
        Show-Cursor
    }
}

function Stop-Spin {
    $elapsed = 0.0
    if ($script:SpinSW) {
        $script:SpinSW.Stop()
        $elapsed = $script:SpinSW.Elapsed.TotalSeconds
        $script:SpinSW = $null
    }
    if ($script:SpinPS) {
        try { $script:SpinPS.Stop() } catch {}
        try { $script:SpinPS.Dispose() } catch {}
        $script:SpinPS = $null
    }
    if ($script:SpinRS) {
        try { $script:SpinRS.Close() } catch {}
        try { $script:SpinRS.Dispose() } catch {}
        $script:SpinRS = $null
    }
    $script:SpinHandle = $null
    if ($script:UseAnim) { Write-Err "`r$ESC[2K" }
    return $elapsed
}

# --- step tree -------------------------------------------------------------
$script:StepTotal   = 4
$script:StepN       = 0
$script:TallyDone   = 0
$script:TallySkip   = 0
$script:TallyWarn   = 0

function Step-Begin([string]$Label) {
    $script:StepLabel = $Label
    $script:StepChildren = @()
    $script:StepWarn = $false
    Start-Spin $Label
}

function Step-Child([string]$kind, [string]$tag, [string]$text) {
    switch ($kind) {
        'ok'    { $sym = $G_CHK;  $sgr = '32'; $script:TallyDone++ }
        'skip'  { $sym = $G_DASH; $sgr = '2';  $script:TallySkip++ }
        'warn'  { $sym = '!';     $sgr = '33'; $script:StepWarn = $true; $script:TallyWarn++ }
        default { $sym = '!';     $sgr = '31'; $script:StepWarn = $true; $script:TallyWarn++ }
    }
    $pad = $tag.PadRight(9)
    if ($script:StepChildren.Count -eq 0) { $branch = "    $G_TREE " } else { $branch = '      ' }
    $script:StepChildren += ($branch + (Paint $sgr "$sym $pad") + ' ' + $text)
}

function Step-End {
    # Hold the spinner on screen for at least MIN_STEP_MS so fast steps do not
    # flash by, then stop it and report the true elapsed time.
    if ($script:SpinSW) {
        $elapsed = $script:SpinSW.Elapsed.TotalSeconds
        if ($script:UseAnim -and $elapsed -lt ($MIN_STEP_MS / 1000.0)) {
            $wait = [int](($MIN_STEP_MS / 1000.0 - $elapsed) * 1000)
            if ($wait -gt 0) { Start-Sleep -Milliseconds $wait }
        }
    }
    $elapsed = Stop-Spin
    $script:StepN++
    if ($script:StepWarn) { $dot = Yellow $G_DOT } else { $dot = Green $G_DOT }
    if ($script:UseAnim) {
        Write-Err ("  $dot $($script:StepLabel) [$($script:StepN)/$($script:StepTotal)]  " + (Dim ('{0:0.0}s' -f $elapsed)) + "`n")
    } else {
        Write-Err "  $dot $($script:StepLabel) [$($script:StepN)/$($script:StepTotal)]`n"
    }
    foreach ($child in $script:StepChildren) { Write-Err "$child`n" }
    Write-Err "`n"
}

function Print-Summary {
    $warn = if ($script:TallyWarn -gt 0) { Yellow "! $($script:TallyWarn) warning(s)" } else { Dim "! $($script:TallyWarn) warnings" }
    Write-Err ("  " + (Green "$G_CHK $($script:TallyDone) done") + "  " + (Dim "$G_DASH $($script:TallySkip) skipped") + "  " + $warn + "`n`n")
}

function Print-Box([string[]]$lines) {
    $w = 0
    foreach ($l in $lines) { if ($l.Length -gt $w) { $w = $l.Length } }
    $bar = ([string]$G_HB) * ($w + 4)
    Write-Err ("  " + (Accent $G_TL) + (Accent $bar) + (Accent $G_TR) + "`n")
    foreach ($l in $lines) {
        $pad = ' ' * ($w - $l.Length)
        Write-Err ("  " + (Accent $G_VB) + "  $l$pad  " + (Accent $G_VB) + "`n")
    }
    Write-Err ("  " + (Accent $G_BL) + (Accent $bar) + (Accent $G_BR) + "`n")
}

# --- platform --------------------------------------------------------------
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne 'AMD64') {
    throw "unsupported Windows CPU architecture: $arch (only x86_64 builds are published)"
}
$target = 'x86_64-pc-windows-msvc'
$ext = 'zip'
$destDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE '.local\bin' }
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "svgfetch.$([guid]::NewGuid().ToString('N'))"

Enable-VT
New-Item -ItemType Directory -Path $tmp -Force | Out-Null

try {
    # --- step 1: resolve latest release tag -------------------------------
    Step-Begin "Resolving release"
    if ($version -eq 'latest') {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
        $version = $release.tag_name
        if (-not $version) {
            Stop-Spin | Out-Null
            throw 'could not determine the latest release tag'
        }
    }
    Step-Child 'ok' 'resolved' "$version"
    Step-End

    $baseUrl  = "https://github.com/$repo/releases/download/$version"
    $artifact = "$bin-$target.$ext"

    # --- step 2: download artifact (+ checksum file) ----------------------
    Step-Begin "Downloading $artifact"
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact"        -OutFile (Join-Path $tmp $artifact)
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact.sha256" -OutFile (Join-Path $tmp "$artifact.sha256")
    } catch {
        $artifact = "get-svg-$target.$ext"
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact"        -OutFile (Join-Path $tmp $artifact)
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$artifact.sha256" -OutFile (Join-Path $tmp "$artifact.sha256")
    }
    Step-Child 'ok' 'download' $artifact
    Step-End

    # --- step 3: verify SHA-256 -------------------------------------------
    Step-Begin "Verifying checksum"
    $expected = (Get-Content -Raw (Join-Path $tmp "$artifact.sha256") -ErrorAction Stop).Trim().Split(' ')[0]
    $hash = Get-FileHash -Algorithm SHA256 -Path (Join-Path $tmp $artifact)
    if ($hash.Hash -ne $expected.ToUpperInvariant()) {
        Stop-Spin | Out-Null
        throw "checksum mismatch for $artifact (expected $expected, got $($hash.Hash))"
    }
    Step-Child 'ok' 'verified' $hash.Hash.ToLower()
    Step-End

    # --- step 4: extract + install ----------------------------------------
    Step-Begin "Installing"
    $extract = Join-Path $tmp 'extracted'
    Expand-Archive -Path (Join-Path $tmp $artifact) -DestinationPath $extract -Force

    $primary = Get-ChildItem -Path $extract -Recurse -Filter "*.exe" | Select-Object -First 1
    if (-not $primary) {
        Stop-Spin | Out-Null
        throw "archive did not contain any executable binary"
    }

    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    foreach ($exe in @("$bin.exe", 'svg-fetch.exe')) {
        $binary = Get-ChildItem -Path $extract -Recurse -Filter $exe | Select-Object -First 1
        if (-not $binary) {
            Copy-Item -Path $primary.FullName -Destination (Join-Path $destDir $exe) -Force
        } else {
            Copy-Item -Path $binary.FullName  -Destination (Join-Path $destDir $exe) -Force
        }
        Step-Child 'ok' 'installed' (Join-Path $destDir $exe)
    }
    Step-End

    Print-Summary
} finally {
    Stop-Spin | Out-Null
    Show-Cursor
    if (Test-Path $tmp) { Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue }
}

# --- version read-back -----------------------------------------------------
$binExe = Join-Path $destDir "$bin.exe"
if (Test-Path $binExe) {
    try {
        $verOut = & $binExe --version 2>$null
        if ($verOut) { Write-Err ("  " + (Green $G_CHK) + " " + (Dim "$verOut") + "`n") }
    } catch {}
}

# --- PATH hint -------------------------------------------------------------
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$destDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$destDir", 'User')
    Write-Err ("  " + (Green $G_CHK) + " " + (Dim "Added $destDir to your PATH") + "`n")
} else {
    Write-Err ("  " + (Green $G_CHK) + " " + (Dim "$destDir is already on your PATH") + "`n")
}
Write-Err "`n"

Print-Box @(
    "svgfetch $version is ready!",
    "",
    "Instant Vector Engine & Curated Icon Registry",
    "",
    "Quick Start:",
    "  svgfetch                Launch interactive vector search",
    "  svgfetch amazon         Download Amazon vector directly",
    "  svgfetch --help         View documentation and options"
)

Write-Host ''
Write-Host '> done. Restart your terminal, then try:  svgfetch --help'
Write-Host $destDir
