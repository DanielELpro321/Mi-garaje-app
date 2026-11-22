# Compila la biblioteca nativa de Android y arma el APK firmado.
# Uso:  powershell -ExecutionPolicy Bypass -File android\build_all.ps1

$ErrorActionPreference = "Stop"

$sdk = $env:ANDROID_HOME
if (-not $sdk) { $sdk = Join-Path $env:LOCALAPPDATA "Android\Sdk" }

$ndkRoot = Join-Path $sdk "ndk"
if (-not (Test-Path $ndkRoot)) { throw "No encontré el NDK en $ndkRoot" }
$ndk = Get-ChildItem $ndkRoot -Directory |
    Sort-Object { [version]($_.Name -replace '[^0-9.]', '') } -Descending |
    Select-Object -First 1
$env:ANDROID_NDK_HOME = $ndk.FullName

# El host de Rust es windows-gnu y necesita dlltool de MinGW.
$mingw = "C:\Users\Daniel\AppData\Local\craft-tools\mingw64\bin"
if (Test-Path $mingw) { $env:PATH = "$mingw;$env:PATH" }

Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo ndk -t arm64-v8a -t x86_64 -o android\app\src\main\jniLibs build --release --lib
    if ($LASTEXITCODE -ne 0) { throw "Falló cargo-ndk" }

    python android\build_apk.py
    if ($LASTEXITCODE -ne 0) { throw "Falló el empaquetado del APK" }
}
finally {
    Pop-Location
}
