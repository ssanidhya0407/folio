<#
.SYNOPSIS
  Build folio's Microsoft Store package (.msix).

.DESCRIPTION
  Tauri's bundler has no MSIX target, so this script assembles one from the
  release build: it stages folio.exe plus the native libraries next to it,
  applies your Partner Center identity to the manifest, indexes the scaled
  tile assets into a resources.pri, and packs the result with makeappx.

  Do NOT sign the package you upload to the Store — Microsoft signs it with
  your publisher certificate during ingestion, and a pre-signed package is
  rejected. Use -SelfSign only to sideload and test locally.

.PARAMETER SelfSign
  Also produce a locally signed copy for sideload testing, creating a
  self-signed certificate if one does not already exist.

.PARAMETER SkipBuild
  Package whatever is already in target/release instead of rebuilding.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File msix\build-msix.ps1
  powershell -ExecutionPolicy Bypass -File msix\build-msix.ps1 -SelfSign
#>

[CmdletBinding()]
param(
    [switch]$SelfSign,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$MsixDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = Split-Path -Parent $MsixDir
$Release = Join-Path $Root 'target\release'
$Runtime = Join-Path $Root 'src-tauri\runtime'
$OutDir = Join-Path $Root 'target\msix'
$Stage = Join-Path $OutDir 'stage'

function Find-SdkTool([string]$Name) {
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "$env:ProgramFiles\Windows Kits\10\bin"
    )
    foreach ($r in $roots) {
        if (-not (Test-Path $r)) { continue }
        $hit = Get-ChildItem -Path $r -Filter $Name -Recurse -ErrorAction SilentlyContinue |
               Where-Object { $_.FullName -match '\\x64\\' } |
               Sort-Object FullName -Descending |
               Select-Object -First 1
        if ($hit) { return $hit.FullName }
    }
    return $null
}

# ---------------------------------------------------------------- identity ---

# Partner Center assigns these once you reserve the app name. Keeping them in
# an untracked file means the manifest in git stays free of account details.
$IdentityFile = Join-Path $MsixDir 'identity.json'
$identity = $null
if (Test-Path $IdentityFile) {
    $identity = Get-Content $IdentityFile -Raw | ConvertFrom-Json
    Write-Host "Identity: $($identity.name) / $($identity.publisher)" -ForegroundColor Cyan
} else {
    Write-Warning @"
No msix\identity.json found, so the manifest keeps its REPLACE-ME placeholders.
The package will build and sideload, but the Store will reject it.

Create msix\identity.json from the values in
Partner Center -> your app -> Product identity:

{
  "name": "12345Publisher.folio",
  "publisher": "CN=ABCDEF12-3456-7890-ABCD-EF1234567890",
  "publisherDisplayName": "Your Publisher Name",
  "version": "1.0.0.0"
}
"@
}

# ------------------------------------------------------------------- build ---

if (-not $SkipBuild) {
    Write-Host "Building release binary…" -ForegroundColor Cyan
    Push-Location $Root
    try {
        cargo build --release -p pdf-gui
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    } finally {
        Pop-Location
    }
}

$exe = Join-Path $Release 'folio.exe'
if (-not (Test-Path $exe)) { throw "folio.exe not found at $exe" }

# ------------------------------------------------------------------- stage ---

Write-Host "Staging package payload…" -ForegroundColor Cyan
if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
New-Item -ItemType Directory -Path $Stage -Force | Out-Null

Copy-Item $exe (Join-Path $Stage 'folio.exe')

if (-not (Test-Path $Runtime)) {
    throw "src-tauri\runtime is missing. Run: python assets\fetch-runtime.py"
}
# The engines load from beside the executable, so they stay flat.
Get-ChildItem $Runtime -File | Where-Object { $_.Extension -in '.dll', '.exe' } |
    ForEach-Object { Copy-Item $_.FullName (Join-Path $Stage $_.Name) }

$tessSrc = Join-Path $Runtime 'tessdata'
if (Test-Path $tessSrc) {
    Copy-Item $tessSrc (Join-Path $Stage 'tessdata') -Recurse
}
if (-not (Test-Path (Join-Path $Stage 'tesseract.exe'))) {
    Write-Warning "tesseract.exe is not in the package - OCR will report itself as unavailable. See README."
}

Copy-Item (Join-Path $MsixDir 'Assets') (Join-Path $Stage 'Assets') -Recurse

# --------------------------------------------------------------- manifest ---

# Edit the XML rather than substituting placeholder strings. String matching
# silently does nothing once a placeholder is renamed, and the result is a
# package that builds fine and is then rejected on upload.
$doc = New-Object System.Xml.XmlDocument
$doc.PreserveWhitespace = $true
$doc.Load((Join-Path $MsixDir 'AppxManifest.xml'))

if ($identity) {
    $doc.Package.Identity.Name = [string]$identity.name
    $doc.Package.Identity.Publisher = [string]$identity.publisher
    $doc.Package.Identity.Version = [string]$identity.version
    $doc.Package.Properties.PublisherDisplayName = [string]$identity.publisherDisplayName
}
$manifest = $doc.OuterXml

# Fail loudly rather than shipping a package Partner Center will bounce.
if ($identity -and $manifest -match 'REPLACE-WITH|PUBLISHER-ID\.') {
    throw "identity substitution failed - the manifest still contains placeholders"
}
$manifestPath = Join-Path $Stage 'AppxManifest.xml'
[System.IO.File]::WriteAllText($manifestPath, $manifest, (New-Object System.Text.UTF8Encoding $false))

# ------------------------------------------------------- resource indexing ---

# resources.pri is what lets Windows pick the right tile for the user's DPI.
$makepri = Find-SdkTool 'makepri.exe'
if ($makepri) {
    Write-Host "Indexing assets (makepri)…" -ForegroundColor Cyan
    $priConfig = Join-Path $OutDir 'priconfig.xml'
    & $makepri createconfig /cf $priConfig /dq en-US /o | Out-Null
    & $makepri new /pr $Stage /cf $priConfig /of (Join-Path $Stage 'resources.pri') /o | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "makepri failed" }
} else {
    Write-Warning "makepri.exe not found; packing without resources.pri (tiles fall back to the unscaled assets)."
}

# -------------------------------------------------------------------- pack ---

$makeappx = Find-SdkTool 'makeappx.exe'
if (-not $makeappx) {
    throw "makeappx.exe not found. Install the Windows 10/11 SDK (Windows App Development tools)."
}

$version = if ($identity) { $identity.version } else { '1.0.0.0' }
$package = Join-Path $OutDir "folio-$version.msix"
if (Test-Path $package) { Remove-Item $package -Force }

Write-Host "Packing $([System.IO.Path]::GetFileName($package))…" -ForegroundColor Cyan
& $makeappx pack /d $Stage /p $package /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }

$sizeMb = [math]::Round((Get-Item $package).Length / 1MB, 1)
Write-Host "`nBuilt $package ($sizeMb MB)" -ForegroundColor Green

# --------------------------------------------------------- optional signing ---

if ($SelfSign) {
    if (-not $identity) { throw "-SelfSign needs msix\identity.json: the certificate subject must equal the manifest Publisher." }

    $signtool = Find-SdkTool 'signtool.exe'
    if (-not $signtool) { throw "signtool.exe not found. Install the Windows SDK." }

    $cert = Get-ChildItem Cert:\CurrentUser\My |
            Where-Object { $_.Subject -eq $identity.publisher } |
            Select-Object -First 1
    if (-not $cert) {
        Write-Host "Creating a self-signed test certificate…" -ForegroundColor Cyan
        $cert = New-SelfSignedCertificate -Type Custom -Subject $identity.publisher `
            -KeyUsage DigitalSignature -FriendlyName 'folio sideload test' `
            -CertStoreLocation 'Cert:\CurrentUser\My' `
            -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
    }

    $signed = Join-Path $OutDir "folio-$version-selfsigned.msix"
    Copy-Item $package $signed -Force
    & $signtool sign /fd SHA256 /sha1 $cert.Thumbprint $signed
    if ($LASTEXITCODE -ne 0) { throw "signtool failed" }

    Write-Host "Signed test package: $signed" -ForegroundColor Green
    Write-Host "To sideload, trust the certificate once (elevated):" -ForegroundColor Yellow
    Write-Host "  Export-Certificate -Cert Cert:\CurrentUser\My\$($cert.Thumbprint) -FilePath folio-test.cer"
    Write-Host "  Import-Certificate -FilePath folio-test.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
}

Write-Host "`nUpload the UNSIGNED package to Partner Center. See msix\SUBMISSION.md." -ForegroundColor Cyan
