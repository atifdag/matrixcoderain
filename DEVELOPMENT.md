# Matrix - Kod Yağmuru: Geliştirme

Bu belge derleme, test, kodun düzeni ve cihazda deneme içindir. Kurulum ve kullanım [README.md](README.md) dosyasındadır.

## Derleme

Derleme betikleri `tools/` klasöründedir; hangi klasörden çalıştırılırsa çalıştırılsın depo kökünü kendisi bulur. Son çıktılar `build/<platform>/` altına yazılır ve depoya eklenir; bunlar sürüm (release) derlemeleridir ve README kullanıcıyı bu hazır paketlere yönlendirir. Çıktı klasörlerine lisans dosyası konmaz; lisanslar yalnız kökteki `LICENSE` ve `licenses/` altındadır. Bir platformun çıktısı, kod değiştiğinde o platformda yeniden üretilip depoya eklenir. Ara dosyalar `src/target/` (Cargo) ve `build/gradle/` (Android) altında kalır ve Git'e eklenmez.

### Windows

Rust 1.98 veya sonrası gerekir.

```powershell
./tools/build.ps1
```

Windows, ekranın açık veya kapalı olduğunu tek tek ekranlar için değil oturumun bütünü için bildirir. Bu yüzden ekranlardan yalnız biri kapatılırsa o ekranın çizimi de sürer; bütün ekranlar kapanınca çizim durur.

### macOS

Xcode komut satırı araçları ve Rust gerekir.

```bash
./tools/build.sh
```

Rust kitaplığı Apple ve Intel işlemciler için ayrı derlenip `lipo` ile birleştirilir. Paket şu komutlarla denetlenir:

```bash
plutil -lint "build/macos/Matrix - Kod Yağmuru.saver/Contents/Info.plist"
lipo -info "build/macos/Matrix - Kod Yağmuru.saver/Contents/MacOS/MatrixKodYagmuru"
codesign --verify --strict --verbose "build/macos/Matrix - Kod Yağmuru.saver"
```

Paket yerel olarak imzalanır; yalnız onu derleyen Mac'te sorunsuz açılır. Başka bir Mac'e dağıtmak için Apple geliştirici imzası gerekir.

Kurulu paketi güncellemeden önce Sistem Ayarları kapatılmalı ve eski kod bellekten atılmalıdır:

```bash
killall legacyScreenSaver
```

Seçili paket bir an yerinde olmazsa (ör. silinip yeniden kurulurken) sistem kaydında "arch mismatch" hatası görülür; bu, paketin mimarisiyle ilgili değildir.

macOS 27'de Sistem Ayarları'ndaki önizleme, önizleme olarak işaretli küçük bir görünüm değil, ekran boyutunda çizilip küçültülen tam ekran bir görünümdür. Bu yüzden önizleme açıkken işlemci kullanımı gerçek ekran koruyucuyla aynıdır.

### Linux

Rust 1.98 veya sonrası ve bir C bağlayıcısı gerekir (Debian ve Ubuntu'da `build-essential`). Wayland, X11 ve xkbcommon kitaplıkları program çalışırken yüklenir; derleme için bunların geliştirme paketleri gerekmez.

```bash
./tools/build.sh
```

Program glibc'ye bağlıdır ve derlendiği sistemin glibc sürümlerini alır. Ubuntu 26.04'te (glibc 2.43) derlenen program Ubuntu 24.04'te açılmaz. Bu yüzden Linux çıktıları Ubuntu 24.04 veya 22.04'te derlenir; Windows'ta WSL'deki Ubuntu 24.04 yeterlidir. Böyle derlenen program glibc 2.34 ister ve Ubuntu 22.04'te de çalışır.

Betik derlemeden sonra programın istediği glibc sürümlerini `readelf -V` ile okur; sürüm 2.35'i (Ubuntu 22.04) aşarsa durur. glibc, program açılırken bu listedeki her sürümü arar; sembolün zayıf bağlı olması yetmez. Örneğin Rust'ın standart kitaplığı alt süreç başlatan kodda `pidfd_spawnp` ve `pidfd_getpid` sembollerini zayıf bağlar, yine de Ubuntu 24.04'te derlenen programın listesine zorunlu `GLIBC_2.39` girer. Programın kendisi alt süreç başlatmaz. Bu kodu getiren tek bağımlılık winit'in Adwaita görünümlü Wayland pencere çerçevesiydi (`wayland-csd-adwaita`; temayı okumak için `dbus-send` çalıştırır), bu yüzden o özellik `Cargo.toml`'da kapalıdır. Ekran koruyucu pencereleri çerçevesizdir; yalnız Wayland'deki deneme penceresi (`/w`) düz yedek çerçeveyle açılır. Yeni bir bağımlılık alt süreç başlatırsa betik derlemeyi durdurur.

### GNOME

Linux'ta, GNOME kurulu olmasa da (ör. WSL); `zip` ve `glib-compile-schemas` (`libglib2.0-bin`) gerekir:

```bash
./tools/build.sh gnome
```

Linux programı derlenir ve eklentiyle birlikte `build/gnome` altındaki iki pakete konur. GNOME 45'te eklenti biçimi değiştiği (ESM) için tek paket bütün sürümlerde yüklenmez:

| Paket | GNOME | Kaynak |
|---|---|---|
| `matrixcoderain@atifdag.github.io.shell-extension.zip` | 46–50 | `src/gnome/` |
| `matrixcoderain@atifdag.github.io.gnome-42-44.shell-extension.zip` | 42–44 | `src/gnome/legacy/` |

Eklentinin kaynakları: `extension.js` boşta kalmayı izler ve programı yönetir, `prefs.js` ayar penceresidir, `src/gnome/schemas/` iki paketin ortak ayar şemasıdır. Sürüm numarası `metadata.json` dosyasına derlemede yazılır. `legacy/` klasörü GNOME 45 öncesinin `imports` biçimiyle yazılmış kopyadır; davranış değişikliği iki klasöre birlikte yapılır. GNOME 44 ve sonrası ayar şemasını kurulumda kendisi derler; GNOME 42–44 paketine derlenmiş şema (`gschemas.compiled`) da konur.

Eklenti programı oturuma ve GNOME sürümüne göre başlatır:

| Oturum | GNOME | Başlatma | Pencerenin tanınması |
|---|---|---|---|
| Wayland | 49–50 | `Meta.WaylandClient.new_subprocess` | `owns_window`; pencere listesinden `window.hide_from_window_list()` ile gizlenir |
| Wayland | 42–48 | `Meta.WaylandClient.new` ve `spawnv` (GNOME 44 öncesi `global.context` almaz) | `owns_window`; `client.hide_from_window_list(window)` ile gizlenir |
| X11 | 42–49 | Sıradan alt süreç (`Gio.SubprocessLauncher`) | Süreç numarası (`window.get_pid()`); X11'de pencere listesinden gizleme yoktur |

Wayland'de program, GNOME Shell'in açtığı özel bir bağlantıyla başlar; böylece pencereleri kesin tanınır. Her yolda pencere en üste alınır ve etkinleştirilir. GNOME 50 X11 desteğini kaldırdığı için `Meta.is_wayland_compositor` işlevi orada yoktur; eklenti bu durumda oturumu Wayland sayar. Çizim GNOME Shell'in içinde yapılmaz: WebAssembly ile denendiğinde 2560 × 1440'lık her karenin GNOME Shell'e aktarılması 7–10 ms sürdü ve masaüstünü yavaşlatacağı için bu yol bırakıldı.

Eklenti GNOME 50'de denenmiştir. Diğer sürümlerdeki yollar GNOME ve mutter kaynaklarındaki API'ye göre yazılmıştır; her sürüm cihazda denendikçe aşağıdaki deneme listesiyle doğrulanır. GNOME'un her yeni ana sürümü denenip `src/gnome/metadata.json` içindeki `shell-version` listesine eklenir.

### Android

**Gerekenler:**
- Android SDK: platform 36, NDK 29.0.14206865 ve platform-tools.
- JDK 17 veya sonrası.
- `cargo install cargo-ndk`.

SDK'nın yeri `ANDROID_HOME` ortam değişkeniyle ya da `src/android/local.properties` dosyasında `sdk.dir=...` satırıyla bildirilir. Bu dosya bilgisayara özeldir ve Git'e eklenmez. Gradle'ın kurulu olması gerekmez; depodaki `gradlew` ilk çalıştırmada doğru sürümü indirir ve sağlama değeriyle doğrular.

```powershell
./tools/build.ps1 android
```

```bash
./tools/build.sh android
```

Proje `src/android/` klasöründedir; Android Studio ile doğrudan da açılabilir. APK hem telefon (64 bit ARM) hem de bilgisayardaki öykünücü (64 bit x86) için kitaplık içerir.

Görüntü her iki eksende yarı boyutta çizilir ve çizim yüzeyi de ekranın yarı boyutunda istenir; ekrana büyütmeyi ekran donanımı yapar. Hazır görüntü yüzeye ekran kartıyla konur; Galaxy S25 Ultra'da bu yol, işlemciyle kopyalamaya göre işlemciyi üçte bir daha az kullandı.

**İmza:** APK bir yayın sürümüdür (hata ayıklanamaz; kullanılmayan kod R8 ile atılır). Kişisel yayın anahtarıyla imzalanır. Anahtarın bilgileri depo dışında, kullanıcı klasöründeki `.android/matrixcoderain-release.properties` dosyasında tutulur:

```text
storeFile=C:/Users/<kullanıcı>/.android/matrixcoderain-release.jks
storePassword=...
keyAlias=matrixcoderain
keyPassword=...
```

Anahtar bir kez JDK'daki `keytool` ile oluşturulur:

```bash
keytool -genkeypair -keystore matrixcoderain-release.jks -storetype PKCS12 -alias matrixcoderain -keyalg RSA -keysize 4096 -validity 36500
```

Bu iki dosya yedeklenmelidir. Kaybolursa telefondaki uygulama güncellenemez; kaldırılıp yeniden kurulması gerekir. Dosya yoksa APK uyarıyla birlikte hata ayıklama anahtarıyla imzalanır. Farklı anahtarla imzalanmış bir APK, kurulu sürümün üzerine kurulamaz.

### iPhone

iOS üçüncü taraf ekran koruyucusuna ve kilit ekranında sürekli çizime izin vermez. iPhone uygulaması bu yüzden açıkken yağmuru ve saati tam ekran gösteren, ekranın kendiliğinden kararmasını engelleyen bir uygulamadır. Arka plana geçince veya telefon kilitlenince çizim durur ve motor bırakılır. Kontrol Merkezi açılınca veya arama gelince yağmur bekler ve ekranın kararması yeniden sisteme bırakılır. Betik `build/ios/` altına iki paket yazar:

- `Matrix - Kod Yağmuru.app`: Apple işlemcili bir Mac'teki iPhone simülatörü için.
- `matrixcoderain.ipa`: Telefon için. Apple imzası taşımaz; kullanıcı onu Sideloadly veya AltStore gibi bir araçla kendi Apple hesabıyla imzalayıp kurar (adımlar README'dedir). İçindeki klasör adı (`MatrixKodYagmuru.app`) Türkçe harf taşımaz; Windows'taki kurulum araçları böyle adları bozabilir.

Mac'te Xcode ile doğrudan telefona kurmak için bir Xcode projesi yoktur.

Xcode (yalnız komut satırı araçları yetmez), iOS simülatörü ve Rust gerekir. Betik Rust'ın `aarch64-apple-ios-sim` ve `aarch64-apple-ios` hedeflerini kendisi ekler. Simülatörde deneme:

```bash
./tools/build.sh ios
xcrun simctl boot "iPhone 17 Pro"
open -a Simulator
xcrun simctl install booted "build/ios/Matrix - Kod Yağmuru.app"
xcrun simctl launch booted com.atifdag.matrixcoderain
```

Uygulama iOS 15 ve sonrası içindir (saniyede 25 kare isteği bu sürümle gelir). Pencere, iOS 26'dan sonra zorunlu olan sahne (scene) yapısıyla kurulur. Görüntü macOS'taki gibi iki IOSurface üzerinden doğrudan ekrana verilir. Saniyede 60 kez tazelenen ekranda sistem 25 kare isteğini 30'a yuvarlar; yağmur yine saniyede 25 adım ilerler. Simülatörde Mac'in işlemcisi ve ekran kartı kullanıldığı için ölçümler telefonun başarımını göstermez.

## Komut satırı seçenekleri

Windows `.scr` dosyası şu seçenekleri tanır:

| Seçenek | Anlamı |
|---|---|
| `/s` | Ekran koruyucuyu başlatır. Tuş, tıklama veya fare hareketiyle kapanır. |
| `/p <pencere>` | Ayarlar penceresindeki küçük önizleme. Windows kendisi verir. |
| `/c` | Ayarlar düğmesi; bilgi mesajı gösterir. |
| `/w` | Deneme penceresi; Esc ile kapanır. |
| `--font <dosya>` | Yağmur için başka bir yazı tipi. |

Linux'ta program şu seçenekleri tanır; seçenekler tek ve iki tireyle yazılabilir:

| Seçenek | Anlamı |
|---|---|
| `-root` | xscreensaver'ın `XSCREENSAVER_WINDOW` değişkeniyle verdiği pencereye çizer. xscreensaver ekran koruyucuyu `--root` ile başlatır. |
| `-window-id <kimlik>` | Verilen X11 penceresine çizer. xscreensaver ayarlarındaki önizleme programı `--root --window-id <kimlik>` ile başlatır. |
| (seçeneksiz) | Boyutu değiştirilebilen deneme penceresi; Esc ile kapanır. |
| `/s` | Bütün ekranları kaplar; tuş, tıklama veya fare hareketiyle kapanır. GNOME eklentisi programı bu seçenekle başlatır. Kilit değildir. |

## Android'de deneme

Android ekran koruyucuyu yalnız telefon şarj olurken başlatır. Pildeyken ekran zaman aşımında her zaman açık ekran (AOD) gelir; kayıtta Matrix yerine `DozeService` başlar. En güvenilir deneme, telefon şarjdayken ekran zaman aşımının dolmasını beklemektir; kısa bir zaman aşımı (ör. 30 saniye) denemeyi hızlandırır. Çalışan ekran koruyucu `adb shell dumpsys dreams` çıktısındaki `mCurrentDream` satırında görülür.

Ekran koruyucunun pilde çalışıp çalışmayacağına telefon üreticisi karar verir; uygulama veya `adb` ayarı bunu değiştiremez. Telefondaki ayarlar şöyle okunur:

```bash
adb shell "dumpsys power | grep -E 'mDreamsEnabledOnBatteryConfig|mDreamsActivateOn'"
```

`mDreamsEnabledOnBatteryConfig=false` ise ekran koruyucu pilde hiç başlamaz; Galaxy S25 Ultra'da böyledir. `mDreamsActivateOnSleepSetting=true` şarjdayken, `mDreamsActivateOnDockSetting=true` yuvadayken başlatmayı açar; bu ikisi telefonun ekran koruyucu ayarından değiştirilir.

Seçili ekran koruyucu bilgisayardan da başlatılabilir:

```bash
adb shell am start -n com.android.systemui/.Somnambulator
```

Galaxy S25 Ultra'da (One UI 9.0) bu komut, uygulama yeniden kurulduktan sonra her zaman çalışmadı; o durumda zaman aşımı beklenir.

**Öykünücüde deneme:** Android Studio'nun cihaz yöneticisinden ya da `avdmanager` ile Android 16 (API 36, x86_64) bir sanal cihaz oluşturulur; APK aynı `adb install` komutuyla kurulur. Ekran koruyucu ayarı öykünücüde **Ayarlar › Ekran › Ekran koruyucu** altındadır.

## Linux'ta deneme

X11 oturumu olmayan bir bilgisayarda (ör. yalnız Wayland olan Ubuntu 26.04 GNOME) xscreensaver, iç içe çalışan bir X sunucusu olan Xephyr'in penceresinde denenebilir. Önizleme, `-activate`, `-lock`, tuş ve fare, birden çok ekran ve ölçüm böyle denenir. Ekranın gerçekten kapanması (DPMS) ve gerçek oturum kilidi denenemez.

```bash
sudo apt-get install xscreensaver xserver-xephyr
Xephyr :5 -screen 1920x1080 &
export DISPLAY=:5 GDK_BACKEND=x11
unset WAYLAND_DISPLAY EGL_PLATFORM
xscreensaver --no-splash --verbose --log xscreensaver.log &
xscreensaver-settings
xscreensaver-command -activate
```

İki ekran için Xephyr `-screen 960x600 -screen 960x600 +xinerama` ile başlatılır. Ölçümde `-glamor` kullanılmaz: Bu kipte Xephyr penceresi başka bir pencerenin altında kalınca Xephyr saniyede bir kareye yavaşlar ve ölçüm bozulur. Ortamda `EGL_PLATFORM=wayland` kalırsa (bazı uygulamaların terminalleri bunu verir) xscreensaver'ın GL denetimi çöker, ekranda "No GL visuals" yazar ve hiçbir ekran koruyucu başlamaz. `GDK_BACKEND=x11` verilmezse ayarlar penceresi Wayland'e açılır ve önizleme çalışmaz.

Ölçümde xscreensaver kilitlenir (`-lock`). Kilitli değilken ana masaüstünden gelen "boşta kalmayı engelle" istekleri (ör. GNOME'un mutter'ı) xscreensaver'ın systemd yardımcısı üzerinden ekran koruyucuyu kapatır.

xscreensaver her ekran için ayrı bir program başlatır. Ekran koruyucudan çıkışta, **Cycle After** süresi dolunca ve ekran kapanınca programı SIGTERM ile kapatır; ekran açılınca yeniden başlatır. İşlemci ve bellek `pidstat -u -r -p <pid> 60` ile ölçülür.

## GNOME'da deneme

Wayland'de eklenti kodu ancak oturum açılırken yüklenir. Değişiklikler gerçek oturumu kapatmadan iç içe bir GNOME Shell'de denenir; bunun için `mutter-dev-bin` paketi gerekir. Ayarlar ve eklentiler gerçek oturumdan ayrı bir klasörde, oturum yolu (D-Bus) da ayrı tutulur. Aşağıdaki komutlar alt kabukta çalıştığı için terminalin ortamını değiştirmez; eklenti 1 dakika boşta kalınca başlar:

```bash
sudo apt-get install mutter-dev-bin
(
  N=~/.cache/matrix-ic-ice
  export XDG_CONFIG_HOME=$N/config XDG_DATA_HOME=$N/data XDG_CACHE_HOME=$N/cache XDG_STATE_HOME=$N/state
  export MATRIXCODERAIN_LOG=$N/matrix.log
  unset EGL_PLATFORM
  mkdir -p $N
  gnome-extensions install --force build/gnome/matrixcoderain@atifdag.github.io.shell-extension.zip
  dbus-run-session -- sh -c '
    gsettings set org.gnome.shell enabled-extensions "[\"matrixcoderain@atifdag.github.io\"]"
    gsettings --schemadir $XDG_DATA_HOME/gnome-shell/extensions/matrixcoderain@atifdag.github.io/schemas \
      set org.gnome.shell.extensions.matrixcoderain start-delay 1
    exec gnome-shell --wayland --devkit --virtual-monitor 1920x1080'
)
```

Bu komutlar GNOME 49 ve sonrası içindir. GNOME 42–48'de iç içe oturum `mutter-dev-bin` olmadan `gnome-shell --nested --wayland` ile açılır; GNOME 42–44'te yukarıdaki paket adı `gnome-42-44` paketiyle değiştirilir. X11 yolu iç içe oturumda denenemez; gerçek X11 oturumunda (giriş ekranında **Ubuntu on Xorg**) denenir.

- İç içe oturumda oturum yöneticisi (gnome-session) çalışmaz. Bu yüzden GNOME Shell'in kaydındaki "boşta kalma engeli okunamadı" uyarısı normaldir; eklenti bu durumda engel yokmuş gibi davranır. Aynı nedenle GNOME'un kendi karartması süreyle gelmez; iç içe oturumun yolunda `gdbus` ile `org.gnome.ScreenSaver.SetActive true` (karartma) ve `org.gnome.ScreenSaver.Lock` (kilit) çağrılarak denenir.
- Eklentide yapılan değişiklik için iç içe GNOME Shell kapatılıp yeniden açılır. Gerçek oturumun GNOME Shell'i de aynı adı taşıdığı için süreç adıyla kapatma (`pkill gnome-shell`) kullanılmaz; iç içe olanın süreç numarası kullanılır.
- Program, GNOME Shell'in ortam değişkenlerini alır; kayıt için `MATRIXCODERAIN_LOG` iç içe GNOME Shell'e verilir. Eklentinin iletileri GNOME Shell'in çıktısına yazılır.
- Gerçek oturumda denerken boşta kalmayı engelleyen bir uygulama varsa ne GNOME karartır ne Matrix başlar. Örneğin Claude masaüstü uygulamasının "çalışırken uyanık tut" ayarı, bir oturum çalıştıkça engel koyar. Engeller `gdbus call --session --dest org.gnome.SessionManager --object-path /org/gnome/SessionManager --method org.gnome.SessionManager.GetInhibitors` ile görülür.

## Test ve denetim

Rust paketinin tanımı (`Cargo.toml`, `Cargo.lock`) `src/` klasöründedir; Cargo komutları orada çalıştırılır; ara dosyalar `src/target/` altına yazılır.

```bash
cd src
cargo test                  # Testler
cargo clippy --all-targets  # Kod denetimi
cargo fmt --all             # Biçim
cargo run -- /w             # Deneme penceresi
```

Otomatik denetim yoktur. Bir değişiklik gönderilmeden önce `src/` klasöründe şunlar çalıştırılır. En düşük desteklenen Rust sürümü 1.98'dir; aynı komutlar `cargo +1.98.0` ile de denenebilir.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo check --lib --no-default-features --features ffi --locked
```

Windows'ta ayrıca gerçek sürüm kitaplığı başka bir dilden çağrılarak bellek ve hata sınırı sınanır (depo kökünde):

```powershell
cargo build --manifest-path src/Cargo.toml --release --lib --no-default-features --features ffi-test --locked
./tools/ffi_host_test.ps1
```

Ortam değişkenleri:

| Değişken | Etkisi |
|---|---|
| `MATRIXCODERAIN_LOG=1` | Sorun giderme kaydını geçici klasördeki `matrixcoderain.log` dosyasının sonuna ekler; dosya kendiliğinden silinmez. Dosya yolu da verilebilir. Değişken verilmezse hiçbir dosya yazılmaz. |
| `MATRIXCODERAIN_FONT=<dosya>` | `--font` seçeneğiyle aynı. |
| `MATRIXCODERAIN_DUMP=<dosya.bmp>` | Testte çizilen kareyi resim olarak kaydeder. |

## Ölçüm kayıtları

Her platform; çizim başladığında, durduğunda ve çalışırken dakikada bir şu biçimde bir satır yazar:

```text
Çalışıyor: 1500 yağmur adımı, 1498 kare; çizim süresi ortanca 2.1 ms, yüzde 95 3.4 ms
```

Çizim süreleri son 256 kareden hesaplanır. "Çizim durdu" ile sonraki "Çizim başladı" satırlarında kare sayısının aynı olması, aradaki sürede (ör. ekran kapalıyken) hiç çizim yapılmadığını gösterir. macOS'ta ekran koruyucu her başlangıçta yeniden kurulur; orada "Ekran uyudu" ile "Ekran uyandı" satırlarındaki kare sayısına bakılır. macOS satırlarında önizleme "(önizleme)" diye ayrılır ve "Çizim başladı" satırı görüntü boyutunu da yazar.

Linux'ta xscreensaver programı kapatıp yeniden başlattığı için yeni programın "Çizim başladı" satırı 0 kareyle başlar. xscreensaver ve GNOME eklentisi programı SIGTERM ile kapatır. Sürüm derlemesi bu sinyalle hemen kapanır ve "Çizim durdu" satırı yazmaz; xscreensaver'da durma anı onun kendi kaydında "killing pid" satırıyla görünür. Hata ayıklama derlemesi (`cargo build`) sinyali yakalar; çizimi durdurup "Çizim durdu" ve "Çıkış" satırlarını yazar.

| Platform | Kayıt nerede |
|---|---|
| Windows, Linux | `MATRIXCODERAIN_LOG=1` verilirse geçici klasördeki `matrixcoderain.log` |
| GNOME | Program aynı kaydı yazar; değişken GNOME Shell'in ortamında olmalıdır. Eklentinin iletileri: `journalctl -b /usr/bin/gnome-shell` |
| macOS | Konsol uygulaması; "Matrix - Kod Yağmuru" diye aratılır. Komut satırında `/usr/bin/log show --last 10m --predicate 'eventMessage CONTAINS "Matrix - Kod Yağmuru"'` (zsh'de `log` kabuğun kendi komutudur; tam yol yazılmalıdır) |
| Android | `adb logcat -s MatrixKodYagmuru` |
| iPhone simülatörü | `xcrun simctl spawn booted log stream --predicate 'process == "MatrixKodYagmuru"'` |

## Cihazda deneme listesi

Otomatik testler kodun doğru derlendiğini ve çizim kurallarını gösterir; gerçek ekran koruyucu davranışını göstermez. Her platformda şu denemeler cihaz üzerinde yapılmalıdır:

| Platform | Deneme | Beklenen |
|---|---|---|
| Hepsi | Ekran koruyucu çalışırken saat, yağmur ve imzaya bakın | Saat doğru; yağmur akıcı; “Atıf Dağ” Türkçe karakterlerle doğru |
| Windows, macOS, Android | Ekranın kapanmasını bekleyin, 10 dakika sonra uyandırın | Ölçüm kaydında durma ve başlama satırlarındaki kare sayısı aynı; yağmur sıçramadan sürer |
| Windows | Ayarlardaki küçük önizleme ve `/s` ile tam ekran | Önizleme pencerede kalır; `/s` ekranı kaplar |
| Windows | Parola seçeneği açıkken ekran koruyucudan dönün | Windows oturum açma ekranı gelir |
| Windows | Türkçe klavyede ekran koruyucu başlar başlamaz | Kendi kendine kapanmaz; ilk tuşta kapanır |
| macOS | Sistem Ayarları'ndaki önizleme ve gerçek ekran koruyucu | İkisi de çalışır; Retina ekranda keskin görünür |
| macOS | Ekran Koruyucu penceresinde seçimi birkaç kez değiştirin | Önizleme akar; kayıtta her "Çizim başladı" satırından önce eski görünümün "Çizim durdu" satırı gelir |
| macOS | Ekran koruyucudan çıkın, Etkinlik Monitörü'ne bakın | `legacyScreenSaver` işlemci kullanmaya devam etmez ve yarım dakika içinde kapanır |
| macOS | Kilitli Ekran'da parola "Hemen" iken ekranın kapanmasını bekleyin | Kayıtta "Ekran uyudu" ve "Çizim durdu" gelir; uyanınca parolayı macOS sorar |
| Linux (X11) | xscreensaver ile kilitleyip açın | Çizim xscreensaver penceresinde; parola xscreensaver'da |
| Linux (X11) | Ayarlardaki küçük önizleme | Önizleme alanını tam doldurur |
| Linux (X11) | Kilitliyken programı `kill -9` ile kapatın | Ekran kilitli kalır |
| Linux (X11) | Ekranın kapanmasını bekleyin, 10 dakika sonra uyandırın | xscreensaver kaydında "monitor has powered down; killing running hacks" ve "re-launching hacks"; aradaki sürede program çalışmaz, yeni programın "Çizim başladı" satırı 0 kareyle başlar |
| GNOME | Başlama süresi kadar boşta kalın | Matrix her ekranda tam ekran açılır; tuş, tıklama veya fare hareketiyle kapanır |
| GNOME | Video oynatırken başlama süresini bekleyin | Matrix açılmaz |
| GNOME | Matrix açıkken GNOME'un karartma süresini bekleyin | GNOME karartır ve Matrix kapanır; uyanınca kilit ekranını GNOME gösterir |
| GNOME | Matrix açıkken Super+L | Kilit ekranı gelir; Matrix kapanır |
| GNOME (X11 oturumu) | Başlama süresi kadar boşta kalın; Matrix açılınca fareye dokunmadan bekleyin, sonra bir tuşa basın | Matrix her ekranda tam ekran ve en üstte açılır, kendi kendine kapanmaz; ilk tuş Matrix'e gider ve onu kapatır |
| GNOME 42–48 | Eklenti ayarlarını açın, başlama süresini değiştirin | Ayar penceresi açılır; yeni süre bir sonraki boşta kalmada geçerlidir |
| Android | Şarjda, boşta; ekrana dokunun | Ekran koruyucu açılır; dokununca kapanır |
| Android | Ekran koruyucu açıkken ekranı güç tuşuyla kapatın | Çizim durur; PIN ve parmak izi ekranı değişmez |
| Android | 30 dakika çalıştırın | Isınma ve pil kullanımı kabul edilebilir; ölçüm kaydındaki yüzde 95 çizim süresi 66 ms'nin (saniyede 15 kare) altında |
| iPhone simülatörü | Uygulamayı açın | Tam ekran, durum çubuğu yok; görüntü ekranın piksel boyutunda ve keskin |
| iPhone simülatörü | Ana ekrana dönün veya telefonu kilitleyin, sonra uygulamaya dönün | Kayıtta "Çizim durdu", dönüşte yeni "Çizim başladı" gelir; arada işlemci kullanılmaz |
| iPhone simülatörü | Telefonu yan çevirin | Yağmur yeni boyutta yeniden başlar; saat ve imza yerinde |

## Kodun düzeni

| Konum | Sorumluluk |
|---|---|
| `src/Cargo.toml`, `src/Cargo.lock` | Rust paketinin tanımı ve kilitli bağımlılıkları. Sürüm numarası buradadır. |
| `src/lib.rs`, `engine.rs` | Ortak kitaplığın girişi; süre yönetimi, durdurma ve yeniden başlatma. |
| `src/rain.rs`, `render.rs`, `clock.rs`, `glyphs.rs`, `rng.rs`, `config.rs` | Yağmur, çizim, saat, harf resimleri, rastgele sayı ve görünüm ayarları. Pencere açmaz, dosya okumaz. |
| `src/tests.rs` | Ortak kitaplığın testleri. |
| `src/ffi.rs`, `src/matrix.h` | C bağlantısı. macOS, iPhone ve Android motoru buradan kullanır. |
| `src/jni.rs` | Android'in Kotlin kodundan C bağlantısına geçiş. |
| `src/desktop/` | Windows `.scr`, Linux xscreensaver bağlantısı ve deneme penceresi. GNOME eklentisi de bu programı çalıştırır. |
| `src/gnome/` | GNOME 46–50 için GNOME Shell eklentisi; boşta kalmayı izler, programı başlatır ve kapatır. `legacy/` aynı eklentinin GNOME 42–44 kopyasıdır. |
| `src/macos/` | macOS `.saver` paketi. |
| `src/android/` | Android ekran koruyucu uygulaması. |
| `src/ios/` | iPhone uygulaması. |
| `assets/fonts/` | Programa gömülen yazı tipleri. |
| `licenses/` | Üçüncü taraf parçaların lisansları: yazı tipleri, Rust kitaplıkları, Kotlin standart kitaplığı. Projenin kendi lisansı kökteki `LICENSE` dosyasındadır. |
| `tools/` | Derleme betikleri (`build.sh`, `build.ps1`), yazı tipi küçültme (`subset.py`), C bağlantısı denemesi (`ffi_host_test.ps1`) ve Rust lisans listesinin ayarı (`about.toml`, `about.hbs`). |
| `build/` | Bütün çıktılar. Son çıktılar `windows/`, `macos/`, `linux/`, `gnome/`, `android/`, `ios/` klasörlerindedir ve depoya eklenir; Android ara dosyaları `gradle/` altındadır ve Git'e eklenmez. Cargo'nun ara dosyaları `src/target/` altındadır. |

Yağmur, 40 milisaniyelik sabit adımlarla ilerler. Ekran saniyede kaç kez çizilirse çizilsin yağmur aynı hızda akar; aynı başlangıç sayısı ve süre her zaman aynı görüntüyü verir. Durdurulmuş motor hesap yapmaz; yeniden başlarken duraklamada geçen süre eklenmez.

C bağlantısında motorun bellek adresi dışarı verilmez; her motor bir kimlik numarasıyla tanınır. Görüntü her piksel için kırmızı, yeşil, mavi ve 255 sırasıyla yazılır. Bir motor yalnız onu oluşturan iş parçacığından kullanılabilir.

## Geliştirme ilkeleri

- **SOLID:** Her dosyanın tek bir sorumluluğu vardır. Ortak kitaplık işletim sistemine özel kod içermez.
- **DRY:** Yağmur ve saat tek yerde yazılır. macOS, iPhone ve Android aynı C bağlantısını kullanır; sürüm numarası yalnız `src/Cargo.toml` içindedir.
- **KISS:** İhtiyacı karşılayan en sade çözüm seçilir. Gerekçesiz katman veya bağımlılık eklenmez.
- Kod açıklamaları ve hata mesajları doğru Türkçe karakterlerle, sade Türkçeyle yazılır.
- Gerçek cihazda denenmemiş bir özellik tamamlanmış sayılmaz.
- README kurulum, kullanım, kaldırma ve destek sınırlarını anlatır; "denendi", "henüz" gibi durum bilgisi yazılmaz.

## Yazı tipleri

Yağmur için Noto Sans JP, imza için Source Sans 3 Medium'dan türetilen yazı tipleri kullanılır. İkisi de derlemede programa gömülür; cihazda kurulu olmaları gerekmez ve sistem yazı tipi klasörleri taranmaz.

Programın kullandığı yazı tipi kitaplığı bir yazı tipindeki bütün harfleri baştan belleğe açar. Bu yüzden iki yazı tipi de yalnız gereken harflere küçültülmüştür. Görüntü özgün dosyalarla piksel piksel aynıdır. Küçültmeyle Windows'ta bellek kullanımı 99,5 MB'den 7,2 MB'ye indi.

| Dosya | İçerik | Özgün kaynak |
|---|---|---|
| `NotoSansJP-Regular-Subset.otf` | Yarım genişlikli katakana, 0-9, A-Z (93 harf, 11 KB) | [Noto CJK Sans 2.004](https://github.com/notofonts/noto-cjk/blob/Sans2.004/Sans/SubsetOTF/JP/NotoSansJP-Regular.otf), SHA-256 `dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073` |
| `MatrixImza-Medium.ttf` | Türkçe Q klavyedeki büyük ve küçük harfler, rakamlar, işaretler, boşluk ve © (136 harf, 13 KB) | [Source Sans 3.052](https://github.com/adobe-fonts/source-sans/releases/tag/3.052R) `TTF/SourceSans3-Medium.ttf`, SHA-256 `a7af35c1cab5dd0b29d742f86f2edfdf83fa7ec213643e765c5c51a2dc24edbc` |

Source Sans 3'ün lisansı "Source" adını korur; değiştirilmiş kopya bu adı taşıyamayacağı için imza yazı tipinin adı **Matrix Imza** yapılmıştır. Telif ve lisans satırları olduğu gibi korunur. İki yazı tipinin de kullanım koşulları (SIL Open Font License 1.1) `licenses/` altındadır.

Küçültülmüş dosyalar özgün dosyalardan şu komutla yeniden üretilir; her çalıştırmada aynı dosya çıkar:

```bash
uvx --from fonttools==4.60.1 python tools/subset.py NotoSansJP-Regular.otf SourceSans3-Medium.ttf
```

İmza metni (`src/config.rs`) Türkçe Q klavyeyle yazılabilen karakterlerle değiştirilebilir. Başka bir karakter gerekirse `tools/subset.py` içindeki listeye eklenip yazı tipi yeniden üretilir. `--font` ile verilen yağmur yazı tipi küçültülmez; büyük bir dosya bellek kullanımını artırır.

## Üçüncü taraf lisansları

Programın içindeki üçüncü taraf parçaların lisansları `licenses/` klasöründedir; `build/` altındaki çıktılara kopyalanmaz. Yazı tiplerinin ve Kotlin standart kitaplığının dosyaları elle tutulur. Rust kitaplıklarının listesi (`RustCrates-LICENSE.txt`) bütün hedefleri kapsar ve `cargo-about` ile `Cargo.lock`'tan üretilir:

```bash
cargo install cargo-about --locked --features cli
cargo about generate --manifest-path src/Cargo.toml -c tools/about.toml --all-features --frozen --fail -o licenses/RustCrates-LICENSE.txt tools/about.hbs
```

Bağımlılık eklendiğinde veya `Cargo.lock` güncellendiğinde liste yeniden üretilir. Kabul listesinde olmayan bir lisans çıkarsa üretim durur; lisans incelenip `tools/about.toml` dosyasındaki listeye eklenir. Android uygulamasına yeni bir kitaplık eklenirse (ör. AndroidX) onun lisansı da `licenses/` altına konur.
