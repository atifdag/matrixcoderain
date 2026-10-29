//! Hazır yazı tipi verisini harf resimlerine dönüştürür; dosya ve sistem aramaz.

use font8x8::UnicodeFonts;

use crate::Error;
use std::sync::{Arc, OnceLock};

/// Yağmur yazı tipi de programın içindedir; cihazda kurulmuş olması gerekmez.
/// Noto Sans JP'nin yalnız yağmurda kullanılan harflerini (yarım genişlikli
/// katakana, rakamlar, büyük Latin harfleri) içeren küçültülmüş kopyasıdır.
/// Yazı tipi kitaplığı bütün harfleri baştan belleğe açtığı için tam dosya
/// yaklaşık 100 MB bellek harcıyordu.
static RAIN_FONT: &[u8] = include_bytes!("../assets/fonts/NotoSansJP-Regular-Subset.otf");

/// Tek bir harfin gri tonlu kapsama haritası.
///
/// `data` içindeki her bayt bir pikselin ne kadar dolu olduğunu söyler
/// (0 = boş, 255 = tamamen dolu). Konum bilgileri taban çizgisine göredir:
/// harf, kalem noktasının `left` piksel sağına ve taban çizgisinin `top` piksel
/// altına (eksi değer: üstüne) yerleştirilir.
pub struct Mask {
    /// Harf resminin genişliği, piksel.
    pub w: usize,
    /// Harf resminin yüksekliği, piksel.
    pub h: usize,
    /// Kalem noktasından sağa kaydırma.
    pub left: i32,
    /// Taban çizgisinden aşağı kaydırma (eksi değer: yukarı).
    pub top: i32,
    /// Bir sonraki harfin kalem noktası kaç piksel sağda olmalı.
    pub advance: f32,
    /// Satır satır, soldan sağa doluluk değerleri.
    pub data: Vec<u8>,
}

impl Mask {
    /// Hiç dolu piksel yoksa doğru döner (boş harf, boşluk vb.).
    pub fn is_blank(&self) -> bool {
        self.data.iter().all(|&a| a == 0)
    }

    /// Hiçbir yazı tipinde bulunmayan harf için boş yer tutucu.
    fn empty(px: f32) -> Mask {
        Mask {
            w: 0,
            h: 0,
            left: 0,
            top: 0,
            advance: px / 2.0,
            data: Vec::new(),
        }
    }
}

/// Yarım genişlikli katakana var mı diye bakılan harf (ｱ).
const HALF_KANA_PROBE: char = '\u{FF71}';
/// Tam genişlikli katakana var mı diye bakılan harf (ア).
const FULL_KANA_PROBE: char = '\u{30A2}';
/// İmza metni için programın içine gömülü yazı tipi: Source Sans 3 Medium'dan
/// türetilmiş "Matrix Imza" (Adobe, SIL Open Font License 1.1; lisans metni
/// depodaki licenses/ klasöründedir). Türkçe Q klavyeyle yazılabilen harfleri,
/// rakamları ve işaretleri içerir; böylece "Atıf Dağ" her işletim sisteminde
/// sistemdeki yazı tiplerinden bağımsız olarak aynı ve doğru görünür.
/// Dosya derleme sırasında programın içine kopyalanır; çalışma anında diskten okunmaz.
static CREDIT_FONT: &[u8] = include_bytes!("../assets/fonts/MatrixImza-Medium.ttf");

/// Yüklenen yazı tipleri. İlk sıradaki ana yazı tipidir; sonrakiler onda
/// bulunmayan harfler için yedektir.
pub struct Fonts {
    /// Öncelik sırasıyla yüklenmiş yazı tipleri.
    list: Vec<fontdue::Font>,
    /// Gömülü imza yazı tipinin `list` içindeki sırası (yüklenemediyse `None`).
    credit: Option<usize>,
}

impl Fonts {
    /// İsteğe bağlı yağmur yazı tipini ve gömülü Türkçe imza yazı tipini hazırlar.
    /// Yağmur yazı tipi verilmezse gömülü Noto Sans JP kullanılır. Dosya birden
    /// çok yazı tipi içeriyorsa önce yarım, sonra tam genişlikli katakana içeren seçilir.
    pub fn from_bytes(rain: Option<&[u8]>) -> Result<Self, Error> {
        let mut fonts = Self {
            list: Vec::new(),
            credit: None,
        };
        {
            let bytes = rain.unwrap_or(RAIN_FONT);
            let index = find_face(bytes, &[HALF_KANA_PROBE])
                .or_else(|| find_face(bytes, &[FULL_KANA_PROBE]))
                .or_else(|| find_face(bytes, &[]))
                .ok_or(Error::InvalidFont)?;
            fonts
                .list
                .push(load_font(bytes.to_vec(), index).ok_or(Error::InvalidFont)?);
        }
        fonts.credit = Some(fonts.list.len());
        fonts
            .list
            .push(load_font(CREDIT_FONT.to_vec(), 0).ok_or(Error::InvalidFont)?);
        Ok(fonts)
    }

    /// Büyük yazı tipi tabloları her pencere ve motor için yeniden hazırlanmaz.
    pub fn embedded() -> Result<Arc<Self>, Error> {
        static FONTS: OnceLock<Result<Arc<Fonts>, Error>> = OnceLock::new();
        FONTS
            .get_or_init(|| Self::from_bytes(None).map(Arc::new))
            .clone()
    }

    /// Koleksiyon içindeki herhangi bir yazı tipi istenen harfi içeriyor mu?
    pub fn supports(bytes: &[u8], character: char) -> bool {
        find_face(bytes, &[character]).is_some()
    }

    /// Verilen harflerin tamamını içeren ilk yazı tipinin listedeki sırası.
    fn index_with(&self, chars: &[char]) -> Option<usize> {
        self.list
            .iter()
            .position(|f| chars.iter().all(|&c| f.has_glyph(c)))
    }

    /// Verilen harflerin tamamını içeren bir yazı tipi yüklü mü?
    fn has_all(&self, chars: &[char]) -> bool {
        self.index_with(chars).is_some()
    }

    /// Yağmur harfleri için görünüm. Önce yağmur yazı tipine, harf orada yoksa
    /// imza yazı tipine, en son gömülü 8x8 harflere bakılır.
    pub fn rain_face(&self) -> Face<'_> {
        Face {
            fonts: self,
            preferred: 0,
        }
    }

    /// İmza metni için görünüm: gömülü Source Sans 3 tercih edilir. Böylece
    /// imzanın tamamı tek ve Türkçe harfleri içeren bir yazı tipiyle çizilir;
    /// Japonca yazı tipinden Latin harfi, başka bir yazı tipinden Türkçe harf
    /// alınıp karışık bir görünüm oluşmaz.
    pub fn text_face(&self) -> Face<'_> {
        Face {
            fonts: self,
            preferred: self.credit.unwrap_or(0),
        }
    }

    /// Yağmurda akacak kana harflerinin Unicode aralığı (başlangıç ve bitiş dahil).
    pub fn kana_range(&self) -> (u32, u32) {
        if self.has_all(&[HALF_KANA_PROBE]) {
            // Yarım genişlikli katakana: filmdeki görünüm.
            (0xFF66, 0xFF9D)
        } else if self.has_all(&[FULL_KANA_PROBE]) {
            // Tam genişlikli katakana.
            (0x30A1, 0x30F6)
        } else {
            // Gömülü 8x8 yazı tipinde yalnızca hiragana var.
            (0x3041, 0x3093)
        }
    }
}

/// Belirli bir amaç için seçilmiş yazı tipi görünümü.
///
/// Harf çizerken önce tercih edilen yazı tipine bakılır; harf orada yoksa sırayla
/// diğer yüklü yazı tiplerine, sonra gömülü 8x8 harflere düşülür. Satır ölçüleri
/// (taban çizgisi, satır yüksekliği) tercih edilen yazı tipinden alınır.
pub struct Face<'a> {
    fonts: &'a Fonts,
    /// Tercih edilen yazı tipinin `Fonts::list` içindeki sırası.
    preferred: usize,
}

impl Face<'_> {
    /// Satır ölçülerinin alındığı, tercih edilen yazı tipi.
    fn preferred_font(&self) -> Option<&fontdue::Font> {
        self.fonts.list.get(self.preferred)
    }

    /// Yağmurda akacak kana harflerinin Unicode aralığı (bkz. `Fonts::kana_range`).
    pub fn kana_range(&self) -> (u32, u32) {
        self.fonts.kana_range()
    }

    /// Harfi içeren ilk yazı tipi: önce tercih edilen, sonra diğerleri sırayla.
    fn font_for(&self, ch: char) -> Option<&fontdue::Font> {
        if let Some(f) = self.preferred_font() {
            if f.has_glyph(ch) {
                return Some(f);
            }
        }
        self.fonts
            .list
            .iter()
            .enumerate()
            .find(|(i, f)| *i != self.preferred && f.has_glyph(ch))
            .map(|(_, f)| f)
    }

    /// Taban çizgisinin satırın üstünden uzaklığı, piksel.
    pub fn ascent(&self, px: f32) -> f32 {
        self.preferred_font()
            .and_then(|f| f.horizontal_line_metrics(px))
            .map(|m| m.ascent)
            .unwrap_or(px * 0.8)
    }

    /// Bir satırın toplam yüksekliği, piksel.
    pub fn line_height(&self, px: f32) -> f32 {
        self.preferred_font()
            .and_then(|f| f.horizontal_line_metrics(px))
            .map(|m| m.ascent - m.descent)
            .unwrap_or(px)
    }

    /// Bir harfi verilen boyutta piksel haritasına çevirir.
    pub fn mask(&self, ch: char, px: f32) -> Mask {
        // Önce yüklü yazı tiplerinde harfin kendisi, sonra en yakın karşılığı aranır.
        for c in std::iter::once(ch).chain(substitute(ch)) {
            if let Some(font) = self.font_for(c) {
                return ttf_mask(font, c, px);
            }
        }
        // Yazı tiplerinde yoksa gömülü 8x8 harflere bakılır; o da yoksa soru işareti.
        for c in std::iter::once(ch)
            .chain(substitute(ch))
            .chain(std::iter::once('?'))
        {
            if let Some(m) = bitmap_mask(c, px) {
                return m;
            }
        }
        Mask::empty(px)
    }
}

/// Yazı tipinde bulunmayan harfler için en yakın karşılık.
fn substitute(ch: char) -> Option<char> {
    Some(match ch {
        '\u{0131}' => 'i', // ı
        '\u{0130}' => 'I', // İ
        '\u{011F}' => 'g', // ğ
        '\u{011E}' => 'G', // Ğ
        '\u{015F}' => 's', // ş
        '\u{015E}' => 'S', // Ş
        '\u{00E7}' => 'c', // ç
        '\u{00C7}' => 'C', // Ç
        '\u{00F6}' => 'o', // ö
        '\u{00D6}' => 'O', // Ö
        '\u{00FC}' => 'u', // ü
        '\u{00DC}' => 'U', // Ü
        '\u{00A9}' => 'c', // ©
        _ => return None,
    })
}

/// Dosyadaki yazı tiplerinden, verilen harflerin hepsini içeren ilkinin sırasını verir.
/// Harf listesi boşsa okunabilen ilk yazı tipi seçilir.
fn find_face(bytes: &[u8], probes: &[char]) -> Option<u32> {
    let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    (0..count).find(|&index| {
        ttf_parser::Face::parse(bytes, index)
            .map(|face| probes.iter().all(|&c| face.glyph_index(c).is_some()))
            .unwrap_or(false)
    })
}

/// Dosyadaki belirli sıradaki yazı tipini harf çizmeye hazır hale getirir.
fn load_font(bytes: Vec<u8>, index: u32) -> Option<fontdue::Font> {
    let settings = fontdue::FontSettings {
        collection_index: index,
        ..fontdue::FontSettings::default()
    };
    fontdue::Font::from_bytes(bytes, settings).ok()
}

/// Yazı tipi dosyasındaki harfi piksel haritasına çevirir.
fn ttf_mask(font: &fontdue::Font, ch: char, px: f32) -> Mask {
    let (m, data) = font.rasterize(ch, px);
    // `ymin` taban çizgisinden harfin alt kenarına olan uzaklıktır (yukarı yönde).
    // Ekranda y aşağı doğru büyüdüğünden üst kenar: -(ymin + yükseklik).
    Mask {
        w: m.width,
        h: m.height,
        left: m.xmin,
        top: -(m.ymin + m.height as i32),
        advance: m.advance_width,
        data,
    }
}

/// Gömülü 8x8 yazı tipinden harfi bulur.
fn bitmap_glyph(ch: char) -> Option<[u8; 8]> {
    font8x8::BASIC_FONTS
        .get(ch)
        .or_else(|| font8x8::LATIN_FONTS.get(ch))
        .or_else(|| font8x8::HIRAGANA_FONTS.get(ch))
        .or_else(|| font8x8::GREEK_FONTS.get(ch))
}

/// Gömülü 8x8 harfi istenen boyuta büyüterek piksel haritasına çevirir.
fn bitmap_mask(ch: char, px: f32) -> Option<Mask> {
    let rows = bitmap_glyph(ch)?;
    let scale = ((px / 8.0) * 0.9).round().max(1.0) as usize;
    let w = 8 * scale;
    let h = 8 * scale;
    let mut data = vec![0u8; w * h];
    for (ry, row) in rows.iter().enumerate() {
        for rx in 0..8 {
            // Her bayt bir satırdır; en düşük bit en soldaki piksel.
            if (row >> rx) & 1 == 1 {
                for dy in 0..scale {
                    let line = (ry * scale + dy) * w;
                    let start = line + rx * scale;
                    data[start..start + scale].fill(255);
                }
            }
        }
    }
    // 8x8 harflerde taban çizgisi 7. satırın altındadır.
    Some(Mask {
        w,
        h,
        left: 0,
        top: -((7 * scale) as i32),
        advance: (w + scale) as f32,
        data,
    })
}
