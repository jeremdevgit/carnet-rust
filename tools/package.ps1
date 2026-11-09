$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Compilation failed.' }
    $version = '0.4.1'
    $stage = Join-Path 'dist' "CarnetDeNotes-$version-Windows-x64"
    if (Test-Path $stage) { throw 'Choose a clean output directory; existing packages are protected.' }
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Item 'target/release/CarnetDeNotes.exe' $stage
    Copy-Item 'LICENSE','README.md','README.fr.md','README.es.md','README.de.md','README.it.md','README.pt.md','README.zh.md','README.ja.md','EDITEUR.md','SECURITY.md','COMPATIBILITY.md','THIRD-PARTY-NOTICES.txt','VALIDATION.md','CHANGELOG.md','dependency-audit.json' $stage
    New-Item -ItemType Directory -Path (Join-Path $stage 'assets') | Out-Null
    Copy-Item 'assets/OFL-Noto.txt','assets/NOTICE.txt','assets/carnet-de-notes.svg','assets/carnet-de-notes.png','assets/carnet-de-notes.ico' (Join-Path $stage 'assets')
    $archive = "$stage.zip"
    if (Test-Path $archive) { throw 'Existing archive is protected.' }
    Compress-Archive -Path $stage -DestinationPath $archive
    $hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLower()
    $checksum = "$hash  $(Split-Path -Leaf $archive)`n"
    [System.IO.File]::WriteAllText((Join-Path (Get-Location) 'dist/SHA256SUMS.txt'), $checksum, [System.Text.UTF8Encoding]::new($false))
} finally {
    Pop-Location
}
