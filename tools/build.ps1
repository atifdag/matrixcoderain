# Matrix - Kod Yağmuru: Windows ve Android çıktılarını üretir.
#
# Kullanım:
#   ./tools/build.ps1            Windows ekran koruyucusu  -> build\windows
#   ./tools/build.ps1 android    Android uygulaması        -> build\android
#
# Kurulum yapmaz, sistem ayarlarını değiştirmez. Ara dosyalar src\target ve
# build\gradle altında kalır.
param(
    [ValidateSet('windows', 'android')]
    [string]$Target = 'windows'
)
$ErrorActionPreference = 'Stop'

# Betik tools\ altındadır; bütün yollar depo kökünden verilir.
$root = Split-Path -Parent $PSScriptRoot

# Dış komutu çalıştırır; komut başarısız olursa betiği verilen mesajla durdurur.
function Invoke-Native([string]$FailureMessage, [scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw $FailureMessage }
}

# Windows ekran koruyucusu: masaüstü programı .scr uzantısıyla kopyalanır.
function Build-Windows {
    Invoke-Native 'Rust derlemesi başarısız.' { cargo build --manifest-path 'src\Cargo.toml' --release --locked --bin matrixcoderain }
    $output = Join-Path $root 'build\windows'
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    $saver = Join-Path $output 'Matrix - Kod Yağmuru.scr'
    Copy-Item -LiteralPath 'src\target\release\matrixcoderain.exe' -Destination $saver -Force
    Get-FileHash -LiteralPath $saver -Algorithm SHA256 | Format-List Algorithm, Hash
    Write-Host "Hazır: $output"
}

# Android uygulaması: Rust kitaplığını ve APK'yı Gradle derler. Gradle'ın
# kendisi ilk çalıştırmada sarmalayıcı (gradlew) tarafından indirilir.
function Build-Android {
    if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
        throw 'cargo-ndk bulunamadı. Kurulum: cargo install cargo-ndk'
    }
    Invoke-Native 'Rust Android hedefleri eklenemedi.' {
        rustup target add aarch64-linux-android x86_64-linux-android
    }
    $project = Join-Path $root 'src\android'
    Invoke-Native 'Android derlemesi başarısız.' { & (Join-Path $project 'gradlew.bat') -p $project deliverApk }
    Write-Host "Hazır: $(Join-Path $root 'build\android')"
}

Push-Location $root
try {
    switch ($Target) {
        'windows' { Build-Windows }
        'android' { Build-Android }
    }
} finally {
    Pop-Location
}
