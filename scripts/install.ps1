param(
    [string]$Version = "latest",
    [string]$BinDir = (Join-Path $env:USERPROFILE ".local/bin"),
    [switch]$PrintTarget
)
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") { throw "This installer is for Windows." }
$architecture = $env:PROCESSOR_ARCHITEW6432
if (-not $architecture) { $architecture = $env:PROCESSOR_ARCHITECTURE }
if ($PSVersionTable.PSVersion.Major -ge 6) {
    $architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
}
switch ($architecture.ToUpperInvariant()) {
    "X64" { $target = "x86_64-pc-windows-msvc" }
    "AMD64" { $target = "x86_64-pc-windows-msvc" }
    "ARM64" { $target = "aarch64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $architecture" }
}
if ($PrintTarget) { Write-Output $target; exit 0 }
if ($Version -ne "latest") {
    $Version = $Version -replace "^v", ""
    if ($Version -notmatch "^\d+\.\d+\.\d+$") { throw "Invalid version." }
}
if ([string]::IsNullOrWhiteSpace($BinDir)) { throw "BinDir must not be empty." }
Get-Command git -ErrorAction Stop | Out-Null
Get-Command tar -ErrorAction Stop | Out-Null
$base = $env:ALREADY_TESTED_DOWNLOAD_BASE_URL
if ([string]::IsNullOrWhiteSpace($base)) {
    if ($Version -eq "latest") {
        $base = "https://github.com/dblock/already-tested/releases/latest/download"
    } else {
        $base = "https://github.com/dblock/already-tested/releases/download/v$Version"
    }
}
function Get-Asset([string]$Url, [string]$Destination) {
    $uri = [Uri]$Url
    if ($uri.Scheme -eq "file") {
        Copy-Item -LiteralPath $uri.LocalPath -Destination $Destination
    } elseif ($uri.Scheme -eq "https") {
        Invoke-WebRequest -UseBasicParsing -Uri $uri -OutFile $Destination
    } else {
        throw "Download URL must use HTTPS (or file:// for local archives)."
    }
}
$temporary = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
$stage = $null
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    $archive = "already-tested-$target.tar.gz"
    Get-Asset "$($base.TrimEnd('/'))/$archive" (Join-Path $temporary "archive.tar.gz")
    Get-Asset "$($base.TrimEnd('/'))/$archive.sha256" (Join-Path $temporary "checksum")
    $checksum = [IO.File]::ReadAllText((Join-Path $temporary "checksum"))
    if ($checksum -cnotmatch "\A([0-9a-f]{64})  $([regex]::Escape($archive))\r?\n\z") {
        throw "Malformed checksum."
    }
    $expected = $Matches[1]
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $temporary "archive.tar.gz")).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "Archive checksum mismatch." }
    & tar -xzf (Join-Path $temporary "archive.tar.gz") -C $temporary -- already-tested.exe
    if ($LASTEXITCODE -ne 0) { throw "Could not extract executable." }
    $executable = Join-Path $temporary "already-tested.exe"
    $item = Get-Item -LiteralPath $executable
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Archive must contain a regular executable."
    }
    $installedVersion = & $executable --version
    if ($LASTEXITCODE -ne 0 -or $installedVersion -notmatch "^\d+\.\d+\.\d+$") { throw "Invalid executable." }
    if ($Version -ne "latest" -and $installedVersion -ne $Version) { throw "Executable version mismatch." }
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $stage = Join-Path $BinDir (".already-tested-" + [Guid]::NewGuid().ToString() + ".exe")
    Copy-Item -LiteralPath $executable -Destination $stage
    $destination = Join-Path $BinDir "already-tested.exe"
    if (Test-Path -LiteralPath $destination) {
        [IO.File]::Replace($stage, $destination, [NullString]::Value)
    } else {
        [IO.File]::Move($stage, $destination)
    }
    $stage = $null
    Write-Output "Installed already-tested $installedVersion to $destination"
} finally {
    if ($stage -and (Test-Path -LiteralPath $stage)) { Remove-Item -LiteralPath $stage -Force }
    foreach ($name in @("archive.tar.gz", "checksum", "already-tested.exe")) {
        $path = Join-Path $temporary $name
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
    Remove-Item -LiteralPath $temporary
}
