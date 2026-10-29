//! Görünüm ve hız ayarları.
//!
//! Renkleri, hızı ve imza metnini değiştirmek için yalnızca bu dosyayı düzenlemek yeterlidir.

/// Kırmızı, yeşil, mavi bileşenleri 0-255 arasında olan renk.
pub type Rgb = (u8, u8, u8);

/// Arka plan rengi (filmdeki gibi siyah).
pub const BG: Rgb = (0, 0, 0);

/// Damlanın baş harfinin hemen arkasındaki en parlak iz rengi.
pub const BRIGHT: Rgb = (0, 255, 70);

/// Her damlanın en öndeki (baş) harfinin rengi.
pub const HEAD: Rgb = (210, 255, 210);

/// Yedi parçalı saatin yanan parça rengi.
pub const CLOCK: Rgb = (90, 255, 120);

/// Saatin her parçasının çevresine çizilen koyu kontur; saat yağmurun üzerinde okunur kalsın diye.
pub const CLOCK_HALO: Rgb = (0, 0, 0);

/// Saat rakamlarının yatay eğimi, yüksekliğe oranla (0.0 = dik, 0.08 = hafif yatık LED görünümü).
pub const CLOCK_SLANT: f32 = 0.0;

/// Ekranın alt ortasındaki küçük imza metni. Gömülü imza yazı tipinde Türkçe Q
/// klavyeyle yazılabilen karakterler ve © bulunur; bunların dışında bir
/// karakter yazılırsa yerine benzeri ya da basit 8x8 harf çizilir.
pub const CREDIT_TEXT: &str = "\u{00A9} At\u{0131}f Da\u{011F}";

/// İmza metninin rengi.
pub const CREDIT: Rgb = (0, 170, 60);

/// Rakamlar 0-9 için yanan parça bit maskeleri.
/// Bitler: a=1 b=2 c=4 d=8 e=16 f=32 g=64
/// (a üst, b sağ üst, c sağ alt, d alt, e sol alt, f sol üst, g orta).
pub const SEGMENTS: [u8; 10] = [63, 6, 91, 79, 102, 109, 125, 7, 127, 111];
