$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Compilation failed.' }
    $version = '0.3.0'
    $stage = Join-Path 'dist' "CarnetRust-$version-Windows-x64"
    if (Test-Path $stage) { throw 'Choose a clean output directory; existing packages are protected.' }
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Item 'target/release/CarnetRust.exe' $stage
    Copy-Item 'LICENSE','README.md','README.fr.md','README.es.md','README.de.md','README.it.md','README.pt.md','README.zh.md','README.ja.md','SECURITY.md','COMPATIBILITY.md','THIRD-PARTY-NOTICES.txt','VALIDATION.md','CHANGELOG.md','dependency-audit.json' $stage
    New-Item -ItemType Directory -Path (Join-Path $stage 'assets') | Out-Null
    Copy-Item 'assets/OFL-Noto.txt','assets/NOTICE.txt' (Join-Path $stage 'assets')
    $archive = "$stage.zip"
    if (Test-Path $archive) { throw 'Existing archive is protected.' }
    Compress-Archive -Path $stage -DestinationPath $archive
    $hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLower()
    "$hash  $(Split-Path -Leaf $archive)" | Set-Content -Encoding ascii (Join-Path 'dist' 'SHA256SUMS.txt')
} finally {
    Pop-Location
}
