# Matrix - Kod Yağmuru

Matrix tarzı akan karakterleri, ortada yedi parçalı bir dijital saati ve alt ortada küçük bir imzayı gösteren ekran koruyucu. Windows, macOS, Linux ve Android'de aynı görüntüyü kendi ekran koruyucu altyapısı üzerinden çizer; iPhone'da açıkken bu görüntüyü gösteren bir uygulamadır.

## Ne yapar, ne yapmaz

- Bilgisayar veya telefon normal kullanılırken animasyon görünmez. Yalnız işletim sistemi ekran koruyucuyu başlattığında çalışır. iPhone'da ekran koruyucu olmadığı için uygulama açıldığında çalışır.
- Ekran kapandığında, cihaz uyuduğunda veya ekran koruyucu durdurulduğunda çizim de durur. Uygulama ekranı uyanık tutmaz ve güç ayarlarına dokunmaz. iPhone uygulaması yalnız açıkken ekranın kararmasını engeller.
- Parola, PIN, parmak izi ve yüz tanıma işletim sisteminin işidir. Uygulama kendi kilit ekranını yapmaz, sistemin kilit ekranını değiştirmez.
- OLED ekranlarda iz bırakmaması için saat her dakika küçük bir çember üzerinde kayar; imza da yanlara ve yukarı kayar, ekranın altından taşmaz. Kayma ekran yüksekliğinin yaklaşık yüzde biridir; 1080p'de en çok 10 piksel.
- Canlı duvar kâğıdı, sürekli arka plan animasyonu ve iPhone kilit ekranı kapsam dışıdır.

## İndirme

Kuruluma hazır paketler depodaki `build/` klasöründedir. Depoyu GitHub'da **Code › Download ZIP** ile indirip açın ve platformunuzun klasörünü kullanın.

| Platform | Nasıl çalışır | Hazır paket |
|---|---|---|
| Windows 10/11 | Sistem ayarlarından seçilen ekran koruyucu; çoklu ekran ve küçük önizleme | [build/windows](build/windows): `Matrix - Kod Yağmuru.scr` |
| macOS 11 ve sonrası | Sistem Ayarları'ndan seçilen ekran koruyucu; Apple ve Intel işlemciler | [build/macos](build/macos): `Matrix - Kod Yağmuru.saver` |
| Ubuntu 24.04 ve 26.04 (GNOME 46–50) | GNOME eklentisi; boşta kalınca tam ekran açılır, karartma ve kilit GNOME'da kalır | [build/gnome](build/gnome): `matrixcoderain@atifdag.github.io.shell-extension.zip` |
| Ubuntu 22.04 ve Debian 12 (GNOME 42–44) | Aynı GNOME eklentisi, bu sürümler için ayrı paket | [build/gnome](build/gnome): `matrixcoderain@atifdag.github.io.gnome-42-44.shell-extension.zip` |
| Xfce gibi diğer Linux masaüstleri (Xorg) | xscreensaver'ın ekran koruyucusu; kilidi xscreensaver yönetir. Ubuntu 22.04, Debian 12 ve sonrası | [build/linux](build/linux): `matrixcoderain` |
| Diğer Wayland masaüstleri (ör. KDE) | Desteklenmiyor. Bu masaüstleri ekran koruyucuyu kendileri çizer. | — |
| Android 8 ve sonrası | Sistemin ekran koruyucu listesinde görünür; şarjda ve boştayken çalışır | [build/android](build/android): `matrixcoderain.apk` |
| iPhone (iOS 15 ve sonrası) | Ekran koruyucu değil; açıkken yağmuru ve saati gösteren uygulama. Kendi Apple hesabınızla imzalanarak kurulur. | [build/ios](build/ios): `matrixcoderain.ipa` |

## Windows

1. `build/windows/Matrix - Kod Yağmuru.scr` dosyasını kalıcı bir klasöre koyun. Dosya sonradan taşınırsa Windows onu bulamaz.
2. Dosyaya sağ tıklayıp **Yükle**'yi seçin. Ekran koruyucu ayarları bu seçimle açılır.
3. Dönüşte parola istenmesi için aynı pencerede **Devam edildiğinde oturum açma ekranını göster** seçeneğini işaretleyin.

Ayarlara daha sonra **Ayarlar › Kişiselleştirme › Kilit ekranı › Ekran koruyucu** yolundan ulaşılır. Win+L ile açılan kilit ekranı Windows'a aittir ve değişmez.

Birden çok ekranda yalnız biri kapatılırsa o ekranda da çizim sürer; bütün ekranlar kapanınca çizim durur.

Kaldırmak için ekran koruyucu ayarlarında **(Yok)** ya da başka bir ekran koruyucu seçilir, ardından `.scr` dosyası silinir.

## macOS

`build/macos/Matrix - Kod Yağmuru.saver` paketine çift tıklayın; macOS kurmayı önerir. Ardından **Sistem Ayarları › Ekran Koruyucu** bölümünden seçin. Yeni macOS sürümlerinde bu bölüm **Sistem Ayarları › Duvar Kâğıdı › Ekran Koruyucu…** altındadır; orada **Özel** işaretlenip **Matrix - Kod Yağmuru** seçilir.

Ekran koruyucunun başlama süresi, **Sistem Ayarları › Kilitli Ekran** bölümündeki ekranı kapatma süresinden kısa olmalıdır; aksi halde ekran, ekran koruyucu başlamadan kararır. Dönüşte parola istenmesi de aynı bölümden ayarlanır.

Paketi güncellemeden önce Sistem Ayarları'nı kapatın. Açıkken güncellenirse önizleme, Sistem Ayarları yeniden açılana kadar görünmez.

Hazır paket Apple geliştirici imzası taşımaz. Başka bir Mac'te macOS paketi açarken uyarı verebilir veya açmayı reddedebilir.

Kaldırmak için Sistem Ayarları kapatılır ve `Matrix - Kod Yağmuru.saver` paketi silinir. Paket yalnız sizin için kurulduysa `~/Library/Screen Savers/`, bütün kullanıcılar için kurulduysa `/Library/Screen Savers/` klasöründedir.

## Ubuntu ve GNOME

GNOME masaüstünde ekran koruyucu bir GNOME eklentisidir. Bilgisayar ayarlanan süre kadar boşta kalınca Matrix her ekranda tam ekran açılır; bir tuş, tıklama veya fare hareketiyle kapanır. Ekranı karartma, kilit ve parola GNOME'da kalır: GNOME ekranı karartmaya veya kilitlemeye başlayınca Matrix kapanır. Video izlerken olduğu gibi bir uygulama boşta kalmayı engelliyorsa Matrix açılmaz.

Eklenti, giriş ekranında seçilen oturum ne olursa olsun çalışır (ör. **Ubuntu** veya **Ubuntu on Xorg**). Xorg oturumunda Matrix açıkken Alt+Tab listesinde görünür.

GNOME sürümüne göre iki paket vardır. GNOME sürümü **Ayarlar** uygulamasının **Hakkında** bölümünde ya da `gnome-shell --version` komutuyla görülür:

| GNOME | Örnek sistem | Paket |
|---|---|---|
| 46–50 | Ubuntu 24.04, Ubuntu 26.04, Debian 13 | `matrixcoderain@atifdag.github.io.shell-extension.zip` |
| 42–44 | Ubuntu 22.04, Debian 12 | `matrixcoderain@atifdag.github.io.gnome-42-44.shell-extension.zip` |

GNOME 45 ve GNOME 41 ile öncesi desteklenmez.

1. Sisteminize uyan paketi kurun (örnekte Ubuntu 24.04 ve 26.04 paketi):

   ```bash
   gnome-extensions install --force build/gnome/matrixcoderain@atifdag.github.io.shell-extension.zip
   ```

2. Oturumu kapatıp yeniden açın. GNOME yeni eklentileri ancak oturum açılırken yükler.
3. **Eklentiler** uygulamasında **Matrix - Kod Yağmuru**'nu açın ya da şu komutu çalıştırın:

   ```bash
   gnome-extensions enable matrixcoderain@atifdag.github.io
   ```

4. Eklentinin ayarlarından **Başlama süresi**ni seçin (varsayılan 2 dakika). Bu süre **Ayarlar › Güç** bölümündeki ekranı karartma süresinden kısa olmalıdır; aksi halde GNOME ekranı Matrix başlamadan karartır.

Kaldırmak için **Eklentiler** uygulamasından kaldırılır ya da `gnome-extensions uninstall matrixcoderain@atifdag.github.io` çalıştırılır.

## Diğer Linux masaüstleri (xscreensaver)

Matrix, xscreensaver'ın ekran koruyucularından biri olarak çalışır. Kilit ekranı ve parola xscreensaver'dadır; Matrix kapanırsa veya hata verirse kilit açılmaz.

Bu yol Xfce gibi masaüstleri içindir; GNOME'da yukarıdaki eklenti kullanılır. Ubuntu 22.04, Debian 12 veya daha yeni bir sistem, Xorg oturumu ve xscreensaver 6 gerekir; Wayland oturumlarında xscreensaver ekranı kilitleyemez. Masaüstünün kendi ekran koruyucusu (ör. xfce4-screensaver, light-locker) kapatılmalıdır. Debian ve Ubuntu'da xscreensaver şöyle kurulur:

```bash
sudo apt-get install xscreensaver
```

1. Programı komut yolundaki bir klasöre kopyalayın. `~/.local/bin` klasörü yeni oluşturulduysa komut yoluna girmesi için oturumu kapatıp açın.

   ```bash
   install -D -m 755 build/linux/matrixcoderain ~/.local/bin/matrixcoderain
   ```

2. `xscreensaver-settings`'i açın, herhangi bir ayarı (ör. **Blank After**) değiştirip pencereyi kapatın. Ayar dosyası `~/.xscreensaver` ancak bu ilk değişiklikte oluşur.
3. `~/.xscreensaver` dosyasında `programs:` satırının hemen altına şu satırı ekleyin:

   ```text
   "Matrix - Kod Yağmuru"  matrixcoderain -root  \n\
   ```

4. `xscreensaver-settings`'i yeniden açın. **Mode** listesinden **Only One Screen Saver**'ı, altındaki listeden **Matrix - Kod Yağmuru**'nu seçin; sağdaki önizlemede yağmur görünür. Dönüşte parola istenmesi için **Lock Screen After** işaretlenir.

`xscreensaver-command -activate` ekran koruyucuyu hemen başlatır, `xscreensaver-command -lock` ekranı kilitler. Birden çok ekranda her ekranda ayrı yağmur çizilir.

Matrix yalnız xscreensaver içinden çalışır; masaüstü arka planı olarak kullanılamaz.

Kaldırmak için `~/.xscreensaver` dosyasına eklenen satır silinir ve program kaldırılır:

```bash
rm ~/.local/bin/matrixcoderain
```

## Android

`build/android/matrixcoderain.apk` dosyasını telefona aktarıp açın; Android, bilinmeyen kaynaktan kurulum için bir kez izin ister. Bilgisayardan kurmak için:

```bash
adb install -r build/android/matrixcoderain.apk
```

Samsung Galaxy cihazlarda **Ayarlar › Ekran › Ekran koruyucu** bölümünden **Matrix - Kod Yağmuru** seçilir. Aynı sayfadaki önizleme ile hemen denenebilir. Uygulama simgesi bu ayar sayfasını açan küçük bir bilgi ekranı gösterir.

Android ekran koruyucuyu yalnız telefon şarj olurken başlatır; ekran zaman aşımı dolunca açılır. Pildeyken ekran koruyucu açılmaz; ekran kapanır ya da her zaman açık ekran görünür.

Pili ve ısınmayı azaltmak için Android'de görüntü biraz daha düşük çözünürlükte ve daha az akıcı çizilir.

Kaldırmak için **Ayarlar › Uygulamalar** bölümünden **Matrix - Kod Yağmuru** kaldırılır ya da bilgisayardan şu komut çalıştırılır:

```bash
adb uninstall com.atifdag.matrixcoderain
```

## iPhone

iOS, uygulamaların ekran koruyucu olmasına ve kilit ekranında çizim yapmasına izin vermez. Bu yüzden iPhone'da Matrix - Kod Yağmuru, açıkken yağmuru ve saati tam ekran gösteren bir uygulamadır; açık kaldığı sürece ekran kararmaz. Ana ekrana dönünce veya telefon kilitlenince çizim durur. Şarjdayken masada saat olarak kullanılabilir.

iPhone, Android'deki gibi dosyayı açarak kurmaya izin vermez; her uygulama bir Apple hesabıyla imzalanmalıdır. `build/ios/matrixcoderain.ipa` imzasızdır ve sizin Apple hesabınızla imzalayan bir araçla kurulur. Ücretsiz bir Apple hesabı yeter; Mac gerekmez.

1. Bilgisayara [Sideloadly](https://sideloadly.io) veya [AltStore](https://altstore.io) kurun. İkisi de Windows'ta ve macOS'ta çalışır.
2. iPhone'u kabloyla bilgisayara bağlayın. `matrixcoderain.ipa` dosyasını araca verin ve Apple hesabınızla giriş yapın; araç uygulamayı imzalayıp telefona kurar.
3. iPhone'da **Ayarlar › Gizlilik ve Güvenlik › Geliştirici Modu**'nu açın; telefon yeniden başlar.
4. **Ayarlar › Genel › VPN ve Aygıt Yönetimi** bölümünde kendi Apple hesabınızı seçip **Güven**'e dokunun.
5. Ana ekrandaki **Kod Yağmuru** uygulamasını açın.

Ücretsiz Apple hesabıyla yapılan kurulum 7 gün geçerlidir; süre dolunca aynı yolla yeniden kurulur. AltStore bu yenilemeyi bilgisayar aynı ağdayken kendisi yapabilir. Ücretli Apple geliştirici hesabıyla süre bir yıldır. Uygulamanın simgesi yoktur; ana ekranda boş bir simgeyle görünür.

Kaldırmak için ana ekranda simgeye basılı tutup **Uygulamayı Sil**'e dokunulur.

## Lisans

Projenin lisansı GPL-3.0 veya sonrasıdır; metin [LICENSE](LICENSE) dosyasındadır. Programın içindeki üçüncü taraf parçaların lisansları [licenses](licenses) klasöründedir:

| Dosya | Parça |
|---|---|
| `NotoSansJP-LICENSE.txt` | Yağmur yazı tipi; Noto Sans JP'den türetilmiştir (SIL Open Font License 1.1) |
| `SourceSans3-LICENSE.txt` | İmza yazı tipi; Source Sans 3'ten türetilmiştir (SIL Open Font License 1.1) |
| `RustCrates-LICENSE.txt` | Programın içindeki açık kaynak kitaplıklar |
| `Kotlin-LICENSE.txt` | Android uygulamasının içindeki açık kaynak kitaplık (Apache 2.0) |
