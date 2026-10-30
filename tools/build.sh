#!/usr/bin/env bash
# Matrix - Kod Yağmuru: macOS, Linux, GNOME, Android ve iPhone çıktılarını üretir.
#
# Kullanım:
#   ./tools/build.sh            Bu sistemin çıktısı
#                               macOS: .saver ekran koruyucusu -> build/macos
#                               Linux: xscreensaver programı   -> build/linux
#   ./tools/build.sh gnome      GNOME Shell eklentisi: GNOME 46–50 ve
#                               GNOME 42–44 paketi (Linux'ta)   -> build/gnome
#   ./tools/build.sh android    Android uygulaması             -> build/android
#   ./tools/build.sh ios        iPhone uygulaması: simülatör paketi ve
#                               telefon için imzasız .ipa (macOS'ta) -> build/ios
#
# Kurulum yapmaz, sistem ayarlarını değiştirmez. Ara dosyalar src/target ve
# build/gradle altında kalır.
set -euo pipefail
# Betik tools/ altındadır; bütün yollar depo kökünden verilir.
cd "$(dirname "$0")/.."

app_name="Matrix - Kod Yağmuru"
manifest=src/Cargo.toml
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$manifest" | head -n 1)"

# Hata mesajını yazar ve betiği durdurur.
fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

# macOS ekran koruyucusu: Rust kitaplığı Apple ve Intel işlemciler için derlenir,
# Objective-C bağlantısıyla tek bir .saver paketine konur.
build_macos() {
  local targets=(aarch64-apple-darwin x86_64-apple-darwin)
  local work=src/target/macos
  local bundle="build/macos/$app_name.saver"
  local libs=()
  export MACOSX_DEPLOYMENT_TARGET=11.0

  rustup target add "${targets[@]}"
  for target in "${targets[@]}"; do
    cargo build --manifest-path "$manifest" --release --locked --lib --no-default-features --features ffi --target "$target"
    libs+=("src/target/$target/release/libmatrix_core.a")
  done
  mkdir -p "$work"
  lipo -create -output "$work/libmatrix_core.a" "${libs[@]}"

  rm -rf "$bundle"
  mkdir -p "$bundle/Contents/MacOS"
  # Paketten yalnız sistemin yüklediği sınıf dışarı açılır. Böylece Rust
  # kitaplığının kullanılmayan kodu atılır; paket yarıdan fazla küçülür.
  clang -bundle -fobjc-arc -O2 -Wall \
    -arch arm64 -arch x86_64 -mmacosx-version-min="$MACOSX_DEPLOYMENT_TARGET" \
    -I src src/macos/MatrixSaverView.m "$work/libmatrix_core.a" \
    -framework ScreenSaver -framework Cocoa -framework QuartzCore -framework IOSurface \
    -Wl,-dead_strip -Wl,-x -Wl,-exported_symbol,'_OBJC_CLASS_$_MatrixKodYagmuruView' \
    -o "$bundle/Contents/MacOS/MatrixKodYagmuru"
  sed "s/@VERSION@/$version/g" src/macos/Info.plist >"$bundle/Contents/Info.plist"
  # Apple işlemcili Mac'ler imzasız kodu yüklemez; yerel imza bu Mac için yeterlidir.
  codesign --force --sign - "$bundle"
  printf 'Hazır: %s\n' "$PWD/$bundle"
}

# Linux programının isteyebileceği en yeni glibc: Ubuntu 22.04'teki 2.35.
# Program, derlendiği sistemin glibc sürümlerini alır. Daha yeni bir sistemde
# derlenirse eski sistemlerde açılmaz; ör. Ubuntu 26.04'te derlenen program
# Ubuntu 24.04'te glibc 2.43 ister. Bu yüzden Ubuntu 24.04 veya 22.04'te
# derlenir.
max_glibc=2.35

# Linux masaüstü programını derler; xscreensaver çıktısı ve GNOME eklentisi bunu kullanır.
build_linux_program() {
  cargo build --manifest-path "$manifest" --release --locked --bin matrixcoderain
  check_glibc src/target/release/matrixcoderain
}

# Programın istediği glibc sürümü sınırı aşıyorsa betiği durdurur. Sürümler,
# glibc'nin program açılırken denetlediği sürüm listesinden (readelf -V)
# okunur. Sembolün zayıf bağlı olması yetmez: listede "WEAK" işareti
# olmayan bir sürüm glibc'de yoksa program açılmaz.
check_glibc() {
  local need
  need="$(readelf -V "$1" | grep 'Name: GLIBC_[0-9]' | grep -v 'Flags: WEAK' | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' | sort -uV | tail -n 1)"
  if [ "$(printf '%s\n%s\n' "$need" "$max_glibc" | sort -V | tail -n 1)" != "$max_glibc" ]; then
    fail "Program glibc $need istiyor; en çok $max_glibc olmalı (Ubuntu 22.04). Daha eski bir sistemde derleyin."
  fi
}

# Linux programı: xscreensaver'ın verdiği pencereye çizer.
build_linux() {
  build_linux_program
  mkdir -p build/linux
  cp src/target/release/matrixcoderain build/linux/matrixcoderain
  printf 'Hazır: %s\n' "$PWD/build/linux"
}

# GNOME Shell eklentisi: Linux programı eklentinin içine konur; eklenti onu
# bilgisayar boşta kalınca başlatır. GNOME 45'te eklenti biçimi değiştiği için
# iki paket çıkar:
# - GNOME 46–50 (ör. Ubuntu 24.04 ve 26.04): src/gnome
# - GNOME 42–44 (ör. Ubuntu 22.04, Debian 12): src/gnome/legacy
build_gnome() {
  command -v zip >/dev/null || fail "zip bulunamadı."
  command -v glib-compile-schemas >/dev/null || fail "glib-compile-schemas bulunamadı (Debian ve Ubuntu'da libglib2.0-bin)."
  build_linux_program
  mkdir -p build/gnome
  pack_gnome src/gnome matrixcoderain@atifdag.github.io.shell-extension.zip
  # GNOME 44 öncesi ayar şemasını kendisi derlemez; derlenmiş şema pakete konur.
  pack_gnome src/gnome/legacy matrixcoderain@atifdag.github.io.gnome-42-44.shell-extension.zip compile
  printf 'Hazır: %s\n' "$PWD/build/gnome"
}

# Bir eklenti paketini build/gnome altına yazar. Girdiler: eklentinin kaynak
# klasörü, paketin adı ve ayar şemasının derlenip derlenmeyeceği.
pack_gnome() {
  local source=$1 name=$2 compile=${3:-} work
  # Paket geçici bir Linux klasöründe hazırlanır; programın çalıştırılabilir
  # olması gibi dosya izinleri Windows diskinde (WSL) de doğru kalır.
  work="$(mktemp -d)"
  mkdir -p "$work/schemas"
  cp "$source/extension.js" "$source/prefs.js" src/target/release/matrixcoderain "$work/"
  cp src/gnome/schemas/*.gschema.xml "$work/schemas/"
  sed "s/@VERSION@/$version/g" "$source/metadata.json" >"$work/metadata.json"
  [ -z "$compile" ] || glib-compile-schemas "$work/schemas"
  chmod 644 "$work"/*.js "$work/metadata.json" "$work"/schemas/*
  chmod 755 "$work/matrixcoderain" "$work/schemas"
  rm -f "build/gnome/$name"
  (cd "$work" && zip -qrX "$OLDPWD/build/gnome/$name" .)
  rm -rf "$work"
}

# Android uygulaması: Rust kitaplığını ve APK'yı Gradle derler. Gradle'ın
# kendisi ilk çalıştırmada sarmalayıcı (gradlew) tarafından indirilir.
build_android() {
  command -v cargo-ndk >/dev/null || fail "cargo-ndk bulunamadı. Kurulum: cargo install cargo-ndk"
  rustup target add aarch64-linux-android x86_64-linux-android
  sh src/android/gradlew -p src/android deliverApk
  printf 'Hazır: %s\n' "$PWD/build/android"
}

# iPhone uygulaması: Rust kitaplığı simülatör ve telefon için ayrı derlenir,
# Objective-C bağlantısıyla .app paketine konur.
# - Simülatör paketi Apple işlemcili bir Mac'teki iPhone simülatöründe çalışır.
# - Telefon paketi (.ipa) Apple imzası taşımaz; kullanıcı onu kendi Apple
#   hesabıyla imzalayan bir kurulum aracıyla telefona kurar.
build_ios() {
  [ "$(uname -s)" = Darwin ] || fail "iPhone uygulaması yalnız macOS'ta derlenir."
  local payload=src/target/ios/Payload
  export IPHONEOS_DEPLOYMENT_TARGET=15.0

  rustup target add aarch64-apple-ios-sim aarch64-apple-ios
  build_ios_app aarch64-apple-ios-sim iphonesimulator -simulator iPhoneSimulator "build/ios/$app_name.app"

  # .ipa içindeki klasör adı Türkçe harf taşımaz; Windows'taki kurulum
  # araçları böyle adları bozabilir. Telefonda görünen ad Info.plist'tedir.
  rm -rf "$payload"
  build_ios_app aarch64-apple-ios iphoneos "" iPhoneOS "$payload/MatrixKodYagmuru.app"
  rm -f build/ios/matrixcoderain.ipa
  (cd "$payload/.." && zip -qrX "$OLDPWD/build/ios/matrixcoderain.ipa" Payload)
  printf 'Hazır: %s\n' "$PWD/build/ios"
}

# Bir iPhone .app paketi derler. Girdiler: Rust hedefi, Apple SDK'sı, clang
# hedefinin eki, Info.plist'teki platform adı ve paketin yolu.
build_ios_app() {
  local target=$1 sdk=$2 suffix=$3 platform=$4 app=$5

  cargo build --manifest-path "$manifest" --release --locked --lib --no-default-features --features ffi --target "$target"
  rm -rf "$app"
  mkdir -p "$app"
  xcrun --sdk "$sdk" clang -fobjc-arc -O2 -Wall \
    -target "arm64-apple-ios$IPHONEOS_DEPLOYMENT_TARGET$suffix" \
    -I src src/ios/MatrixApp.m "src/target/$target/release/libmatrix_core.a" \
    -framework UIKit -framework CoreGraphics -framework QuartzCore -framework IOSurface \
    -Wl,-dead_strip -o "$app/MatrixKodYagmuru"
  sed -e "s/@VERSION@/$version/g" -e "s/@MINIMUM_OS@/$IPHONEOS_DEPLOYMENT_TARGET/g" \
    -e "s/@PLATFORM@/$platform/g" src/ios/Info.plist >"$app/Info.plist"
  # Simülatör yerel imzayla yetinir. Telefonda kurulum aracı bu imzayı
  # kullanıcının Apple hesabıyla yapılan imzayla değiştirir.
  codesign --force --sign - "$app"
}

case "${1:-}" in
  android) build_android ;;
  ios) build_ios ;;
  gnome) build_gnome ;;
  "")
    case "$(uname -s)" in
      Darwin) build_macos ;;
      Linux) build_linux ;;
      *) fail "Bu sistemde tools/build.ps1 kullanın." ;;
    esac
    ;;
  *) fail "Bilinmeyen hedef: $1 (geçerli: android, ios, gnome)" ;;
esac
