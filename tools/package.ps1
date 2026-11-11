# Builds the Windows installer (Inno Setup 6) and SHA256SUMS.txt in dist/.
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Compilation failed.' }
    $compiler = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $compiler) { throw 'Inno Setup 6 compiler not found.' }
    & $compiler installer\windows.iss
    if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
    $setup = Get-ChildItem dist -Filter 'CarnetDeNotes-*-Windows-x64-Setup.exe' | Select-Object -First 1
    $hash = (Get-FileHash -Algorithm SHA256 $setup.FullName).Hash.ToLower()
    [System.IO.File]::WriteAllText((Join-Path (Get-Location) 'dist/SHA256SUMS.txt'), "$hash  $($setup.Name)`n", [System.Text.UTF8Encoding]::new($false))
} finally {
    Pop-Location
}
