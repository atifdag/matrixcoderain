//! Bir karenin oluşturulması: yağmur, saat ve imza tek bir piksel tamponuna çizilir.
//!
//! Tampon, her pikseli `0x00RRGGBB` biçiminde tutan bir `u32` dizisidir; `softbuffer`
//! bu biçimi doğrudan pencereye basar. Çizim sırasında yeni bellek ayrılmaz:
//! harf resimleri (`palette`), saat ve imza sahne kurulurken bir kez hazırlanır.

use crate::clock::Clock;
use crate::config::{Rgb, BG, BRIGHT, CLOCK_HALO, CREDIT, CREDIT_TEXT, HEAD};
use crate::glyphs::{Face, Fonts, Mask};
use crate::rain::{Picker, Rain};
use crate::TimeOfDay;

/// Saydamlığı önceden çarpılmış RGBA resim; her piksel 4 bayt.
pub struct Image {
    /// Genişlik, piksel.
    pub w: usize,
    /// Yükseklik, piksel.
    pub h: usize,
    /// Satır satır kırmızı, yeşil, mavi ve saydamlık baytları.
    pub data: Vec<u8>,
}

/// Başka bir yerde tutulan RGBA resme (örneğin saatin resmine) kopyalamadan bakış.
#[derive(Clone, Copy)]
pub struct ImageRef<'a> {
    /// Genişlik, piksel.
    pub w: usize,
    /// Yükseklik, piksel.
    pub h: usize,
    /// Satır satır kırmızı, yeşil, mavi ve saydamlık baytları.
    pub data: &'a [u8],
}

impl<'a> From<&'a Image> for ImageRef<'a> {
    fn from(img: &'a Image) -> Self {
        ImageRef {
            w: img.w,
            h: img.h,
            data: &img.data,
        }
    }
}

/// Bir pencereye ait tüm çizim durumu.
pub struct Scene {
    /// Görüntü genişliği, piksel.
    width: u32,
    /// Görüntü yüksekliği, piksel.
    height: u32,
    /// Bir hücrenin genişliği ve yüksekliği, piksel.
    cell_w: i32,
    cell_h: i32,
    /// Hücre üstünden taban çizgisine uzaklık.
    baseline: i32,
    /// Yağmurda kullanılan tüm harflerin hazır resimleri.
    palette: Vec<Mask>,
    /// Hücrelerdeki harfler ve damlalar.
    rain: Rain,
    /// Ortadaki dijital saat.
    clock: Clock,
    /// Alt ortadaki imzanın hazır resmi ve konumu.
    credit: Image,
    credit_x: i32,
    credit_y: i32,
    /// Saat ve imzanın en çok kaç piksel kayabileceği.
    drift_range: i32,
    /// Saat ve imzanın bu dakikaki kayması.
    drift: (i32, i32),
}

impl Scene {
    /// Verilen pencere boyutu için sahneyi kurar.
    pub fn new(width: u32, height: u32, fonts: &Fonts, seed: u64) -> Scene {
        let w = width.max(1) as i32;
        let h = height.max(1) as i32;

        // Hücre yüksekliği: ekrana yaklaşık 45 satır sığacak şekilde. Böylece
        // harflerin ekrana oranı her çözünürlükte aynı kalır: 1080p'de 24 piksel,
        // 4K'da 48 piksel, 8K'da 96 piksel. Alt sınır 8 (çok küçük pencereler),
        // üst sınır 96 (8K'ya kadar yeter).
        let cell_h = (h / 45).clamp(8, 96);
        let px = (cell_h - 2) as f32;

        // Yağmur harfleri için yazı tipi görünümü (katakana içeren yazı tipi önde).
        let rain_face = fonts.rain_face();
        let (palette, picker) = build_palette(&rain_face, px);

        // Hücre genişliği: kana harfinin ilerlemesi + 1 piksel; en az hücre yüksekliğinin yarısı.
        let advance = palette
            .get(picker.kana.0 as usize)
            .or(palette.first())
            .map(|m| m.advance)
            .unwrap_or(px / 2.0);
        let cell_w = (advance.ceil() as i32 + 1).max(cell_h / 2).max(4);

        let cols = (w / cell_w).max(1);
        let rows = (h / cell_h).max(1);
        let baseline = rain_face.ascent(px).round() as i32;

        let rain = Rain::new(cols, rows, picker, seed);
        let clock = Clock::new(width, height);

        // İmza: küçük, alt ortada, 1 piksel koyu konturlu. Türkçe harfleri içeren
        // yazı tipi görünümüyle çizilir; böylece "ı" ve "Ğ" doğru görünür ve
        // imzanın tamamı tek yazı tipinden gelir.
        let text_face = fonts.text_face();
        // İmza yüksekliği: 1080p'de 15, 4K'da 30 piksel; en az 9 (okunabilirlik).
        let credit_h = (h / 70).clamp(9, 40);
        let credit = text_image(&text_face, CREDIT_TEXT, credit_h as f32, CREDIT, CLOCK_HALO);
        let margin = credit_h / 2 + 4;
        let credit_x = (w - credit.w as i32) / 2;
        let credit_y = h - margin - credit.h as i32;

        Scene {
            width,
            height,
            cell_w,
            cell_h,
            baseline,
            palette,
            rain,
            clock,
            credit,
            credit_x,
            credit_y,
            // 1080p'de 10, 4K'da 21 piksel; küçük önizlemede en az 2.
            drift_range: (h / 100).max(2),
            drift: (0, 0),
        }
    }

    /// Saat yağmurun süresinden bağımsızdır; değişiklik yalnız burada uygulanır.
    pub fn set_time(&mut self, time: TimeOfDay) {
        self.drift = drift(time, self.drift_range);
        self.clock.update(time);
    }

    /// Çizimin yatay piksel sayısını verir.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Çizimin düşey piksel sayısını verir.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Yağmuru bir adım ilerletir.
    pub fn step(&mut self) {
        self.rain.step();
    }

    /// Kareyi `frame` tamponuna çizer. Tampon en az `width * height` eleman içermeli.
    pub fn render(&self, frame: &mut [u32]) {
        let fw = self.width as i32;
        let fh = self.height as i32;
        let n = (fw * fh) as usize;
        if frame.len() < n {
            return;
        }
        let frame = &mut frame[..n];
        frame.fill(pack(BG));

        let rows = self.rain.rows();
        let cells = self.rain.cells();
        for d in self.rain.drops() {
            let head_row = d.head.floor() as i32;
            for i in 0..d.len {
                let row = head_row - i;
                if row < 0 || row >= rows {
                    continue;
                }
                let color = if i == 0 {
                    HEAD
                } else {
                    // Baş harfin hemen arkasında en parlak, kuyruğa doğru arka plana sönen renk.
                    let t = (i as f32 / d.len as f32).powf(1.4);
                    lerp(BRIGHT, BG, t)
                };
                let index = cells[(d.col * rows + row) as usize] as usize;
                let mask = &self.palette[index];
                blend_mask(
                    frame,
                    fw,
                    fh,
                    d.col * self.cell_w + mask.left,
                    row * self.cell_h + self.baseline + mask.top,
                    mask,
                    color,
                );
            }
        }

        self.clock.blit(frame, fw, fh, self.drift);
        // İmza alt kenarda durduğu için dikeyde yalnız yukarı kayar; ekrandan taşmaz.
        let (dx, dy) = self.drift;
        let range = self.drift_range;
        blend_premultiplied(
            frame,
            fw,
            fh,
            self.credit_x + dx / 2,
            self.credit_y + (dy - range) / 2,
            ImageRef::from(&self.credit),
        );
    }
}

/// OLED ekranlarda uzun süre aynı yerde duran parlak saat ve imza iz bırakabilir.
/// Bu yüzden her dakika küçük bir çember üzerinde sıradaki noktaya geçilir. Saat bu
/// noktaya kayar; imza alt kenardan taşmasın diye yatayda yarısı kadar, dikeyde yalnız
/// yukarı kayar.
/// Aynı dakika her zaman aynı konumu verir; kayma `range` pikseli geçmez.
fn drift(time: TimeOfDay, range: i32) -> (i32, i32) {
    // Çember üzerinde 45 derece aralıklı sekiz nokta.
    const POINTS: [(f32, f32); 8] = [
        (1.0, 0.0),
        (0.707, 0.707),
        (0.0, 1.0),
        (-0.707, 0.707),
        (-1.0, 0.0),
        (-0.707, -0.707),
        (0.0, -1.0),
        (0.707, -0.707),
    ];
    let minute = u32::from(time.hour) * 60 + u32::from(time.minute);
    let (x, y) = POINTS[(minute % 8) as usize];
    let range = range as f32;
    ((x * range).round() as i32, (y * range).round() as i32)
}

/// Yağmurda kullanılacak tüm harflerin resimlerini hazırlar ve grupların
/// palet içindeki yerlerini döndürür.
fn build_palette(face: &Face, px: f32) -> (Vec<Mask>, Picker) {
    let mut palette = Vec::new();
    let (lo, hi) = face.kana_range();
    let kana = add_group(&mut palette, face, px, (lo..=hi).filter_map(char::from_u32));
    let digits = add_group(&mut palette, face, px, '0'..='9');
    let letters = add_group(&mut palette, face, px, 'A'..='Z');
    (
        palette,
        Picker {
            kana,
            digits,
            letters,
        },
    )
}

/// Verilen harfleri palete ekler; boş çıkanlar (yazı tipinde bulunmayanlar) atlanır.
/// Grubun `(başlangıç, adet)` bilgisini döndürür.
fn add_group(
    palette: &mut Vec<Mask>,
    face: &Face,
    px: f32,
    chars: impl Iterator<Item = char>,
) -> (u16, u16) {
    let start = palette.len();
    for c in chars {
        let m = face.mask(c, px);
        if !m.is_blank() {
            palette.push(m);
        }
    }
    (start as u16, (palette.len() - start) as u16)
}

/// Metni konturlu olarak saydam bir resme çizer.
fn text_image(face: &Face, text: &str, px: f32, color: Rgb, halo: Rgb) -> Image {
    let masks: Vec<Mask> = text.chars().map(|c| face.mask(c, px)).collect();
    let total: f32 = masks.iter().map(|m| m.advance).sum();
    let ascent = face.ascent(px).round() as i32;
    // Kontur için her yöne 2 piksel pay.
    let w = total.ceil() as usize + 4;
    let h = face.line_height(px).ceil() as usize + 4;
    let mut img = Image {
        w,
        h,
        data: vec![0; w * h * 4],
    };

    // Önce dört yöne 1 piksel kaydırılmış koyu kontur, sonra asıl renk.
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        stamp_text(&mut img, &masks, 2 + dx, 2 + ascent + dy, halo);
    }
    stamp_text(&mut img, &masks, 2, 2 + ascent, color);
    img
}

/// Harfleri soldan sağa, verilen taban çizgisine yerleştirerek resme basar.
fn stamp_text(img: &mut Image, masks: &[Mask], x: i32, baseline: i32, color: Rgb) {
    let mut pen = x as f32;
    for m in masks {
        stamp_mask(img, pen.round() as i32 + m.left, baseline + m.top, m, color);
        pen += m.advance;
    }
}

/// Tek bir harfi saydam resmin üzerine bindirir.
fn stamp_mask(img: &mut Image, x: i32, y: i32, m: &Mask, color: Rgb) {
    for my in 0..m.h {
        let py = y + my as i32;
        if py < 0 || py >= img.h as i32 {
            continue;
        }
        for mx in 0..m.w {
            let px = x + mx as i32;
            if px < 0 || px >= img.w as i32 {
                continue;
            }
            let a = m.data[my * m.w + mx] as u32;
            if a == 0 {
                continue;
            }
            let inv = 255 - a;
            let i = (py as usize * img.w + px as usize) * 4;
            let d = &mut img.data[i..i + 4];
            d[0] = ((color.0 as u32 * a + d[0] as u32 * inv) / 255) as u8;
            d[1] = ((color.1 as u32 * a + d[1] as u32 * inv) / 255) as u8;
            d[2] = ((color.2 as u32 * a + d[2] as u32 * inv) / 255) as u8;
            d[3] = ((255 * a + d[3] as u32 * inv) / 255) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Piksel işlemleri
// ---------------------------------------------------------------------------

/// Rengi `0x00RRGGBB` biçimine çevirir.
fn pack(c: Rgb) -> u32 {
    ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | (c.2 as u32)
}

/// İki renk arasında doğrusal geçiş; `t` 0.0 ile 1.0 arasında.
fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

/// `w × h` boyutundaki bir kaynak resmin, karede `(x, y)` konumuna yerleştirildiğinde
/// kare içinde kalan kısmını kaynak koordinatlarında verir: `(x0, x1, y0, y1)`,
/// bitişler hariç. Tamamen dışarıdaysa `None`.
///
/// Kırpma bir kez burada yapılır; böylece piksel döngüleri sınır denetimi yapmadan
/// düz dilimler üzerinde çalışır. 4K'da piksel sayısı 1080p'nin dört katı olduğu
/// için bu fark önemlidir.
fn clip(
    fw: i32,
    fh: i32,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
) -> Option<(usize, usize, usize, usize)> {
    let x0 = (-x).max(0) as usize;
    let y0 = (-y).max(0) as usize;
    let x1 = ((fw - x).max(0) as usize).min(w);
    let y1 = ((fh - y).max(0) as usize).min(h);
    if x0 >= x1 || y0 >= y1 {
        None
    } else {
        Some((x0, x1, y0, y1))
    }
}

/// Tek renkli bir harfi harf haritasına göre karenin üzerine bindirir.
///
/// Haritadaki değer 255 ise piksel doğrudan rengi alır (en sık durum, hızlı yol);
/// aradaysa (harfin yumuşatılmış kenarları) renk arka planla karıştırılır.
fn blend_mask(frame: &mut [u32], fw: i32, fh: i32, x: i32, y: i32, m: &Mask, color: Rgb) {
    let Some((x0, x1, y0, y1)) = clip(fw, fh, x, y, m.w, m.h) else {
        return;
    };
    let packed = pack(color);
    let (cr, cg, cb) = (color.0 as u32, color.1 as u32, color.2 as u32);
    let stride = fw as usize;
    for sy in y0..y1 {
        // Bu satırın karedeki ve kaynaktaki dilimleri; ikisi de aynı uzunlukta.
        let dst_start = (y + sy as i32) as usize * stride + (x + x0 as i32) as usize;
        let dst = &mut frame[dst_start..dst_start + (x1 - x0)];
        let src = &m.data[sy * m.w + x0..sy * m.w + x1];
        for (d, &a) in dst.iter_mut().zip(src) {
            let a = a as u32;
            if a == 0 {
                continue;
            }
            if a == 255 {
                *d = packed;
                continue;
            }
            let inv = 255 - a;
            let v = *d;
            let r = (cr * a + ((v >> 16) & 0xFF) * inv) / 255;
            let g = (cg * a + ((v >> 8) & 0xFF) * inv) / 255;
            let b = (cb * a + (v & 0xFF) * inv) / 255;
            *d = (r << 16) | (g << 8) | b;
        }
    }
}

/// Saydamlığı önceden çarpılmış RGBA resmi karenin üzerine bindirir.
/// Formül: `yeni = kaynak + eski * (1 - kaynak_saydamlık)`.
pub fn blend_premultiplied(frame: &mut [u32], fw: i32, fh: i32, x: i32, y: i32, img: ImageRef) {
    let ImageRef { w, h, data: rgba } = img;
    let Some((x0, x1, y0, y1)) = clip(fw, fh, x, y, w, h) else {
        return;
    };
    let stride = fw as usize;
    for sy in y0..y1 {
        let dst_start = (y + sy as i32) as usize * stride + (x + x0 as i32) as usize;
        let dst = &mut frame[dst_start..dst_start + (x1 - x0)];
        let src = &rgba[(sy * w + x0) * 4..(sy * w + x1) * 4];
        for (d, px) in dst.iter_mut().zip(src.as_chunks::<4>().0.iter()) {
            let a = px[3] as u32;
            if a == 0 {
                continue;
            }
            let inv = 255 - a;
            let v = *d;
            // Yuvarlama hataları 255'i aşmasın diye `min(255)`.
            let r = (px[0] as u32 + ((v >> 16) & 0xFF) * inv / 255).min(255);
            let g = (px[1] as u32 + ((v >> 8) & 0xFF) * inv / 255).min(255);
            let b = (px[2] as u32 + (v & 0xFF) * inv / 255).min(255);
            *d = (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sahne pencere olmadan kurulabilmeli ve çizilen karede siyah dışında piksel olmalı.
    /// `MATRIXCODERAIN_DUMP=/yol/kare.bmp` verilirse kare dosyaya yazılır; gözle kontrol için.
    #[test]
    fn frame_has_content() {
        let fonts = Fonts::embedded().expect("gömülü yazı tipi");
        let mut scene = Scene::new(800, 600, &fonts, 7);
        scene.set_time(TimeOfDay::new(12, 34, 56).expect("geçerli saat"));
        for _ in 0..60 {
            scene.step();
        }
        let mut frame = vec![0u32; 800 * 600];
        scene.render(&mut frame);

        let lit = frame.iter().filter(|&&p| p != 0).count();
        assert!(lit > 1000, "karede yeterli dolu piksel yok: {lit}");

        if let Some(path) = std::env::var_os("MATRIXCODERAIN_DUMP") {
            std::fs::write(path, bmp(&frame, 800, 600)).expect("kare dosyaya yazılamadı");
        }
    }

    /// `0x00RRGGBB` tamponunu 24 bit BMP dosyası olarak paketler (satırlar alttan üste).
    fn bmp(frame: &[u32], w: usize, h: usize) -> Vec<u8> {
        let row_bytes = (w * 3 + 3) & !3;
        let pixel_bytes = row_bytes * h;
        let mut out = Vec::with_capacity(54 + pixel_bytes);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&((54 + pixel_bytes) as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(w as i32).to_le_bytes());
        out.extend_from_slice(&(h as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
        out.extend_from_slice(&[0; 16]);
        for y in (0..h).rev() {
            for &p in &frame[y * w..(y + 1) * w] {
                out.extend_from_slice(&[p as u8, (p >> 8) as u8, (p >> 16) as u8]);
            }
            out.resize(out.len() + (row_bytes - w * 3), 0);
        }
        out
    }

    /// Kapsama haritası tam dolu (255) olduğunda piksel doğrudan rengi almalı,
    /// yarı dolu olduğunda arka planla karışmalı.
    #[test]
    fn mask_blending() {
        let mask = Mask {
            w: 2,
            h: 1,
            left: 0,
            top: 0,
            advance: 2.0,
            data: vec![255, 128],
        };
        let mut frame = vec![0u32; 2];
        blend_mask(&mut frame, 2, 1, 0, 0, &mask, (0, 255, 0));
        assert_eq!(frame[0], 0x00FF00);
        assert_eq!(frame[1], 0x008000);
    }

    /// Kare dışına taşan harfler ve resimler çökmeden kırpılmalı; yalnızca içeride
    /// kalan pikseller boyanmalı.
    #[test]
    fn clipping_at_edges() {
        // 3x3 tam dolu harf, 4x4 karenin sol üst köşesinden 1 piksel dışarıya taşıyor.
        let mask = Mask {
            w: 3,
            h: 3,
            left: 0,
            top: 0,
            advance: 3.0,
            data: vec![255; 9],
        };
        let mut frame = vec![0u32; 16];
        blend_mask(&mut frame, 4, 4, -1, -1, &mask, (255, 255, 255));
        let lit: Vec<usize> = (0..16).filter(|&i| frame[i] != 0).collect();
        assert_eq!(lit, vec![0, 1, 4, 5]);

        // Sağ alt köşeden taşan 2x2 resim.
        let img = Image {
            w: 2,
            h: 2,
            data: [255u8; 4].repeat(4),
        };
        let mut frame = vec![0u32; 16];
        blend_premultiplied(&mut frame, 4, 4, 3, 3, ImageRef::from(&img));
        let lit: Vec<usize> = (0..16).filter(|&i| frame[i] != 0).collect();
        assert_eq!(lit, vec![15]);

        // Tamamen dışarıda: hiçbir şey olmamalı.
        let mut frame = vec![0u32; 16];
        blend_mask(&mut frame, 4, 4, 10, 10, &mask, (255, 255, 255));
        assert!(frame.iter().all(|&p| p == 0));
    }

    /// Saat ve imza aynı dakika içinde yerinde durmalı, her dakika kaymalı ve
    /// kayma verilen sınırı geçmemeli.
    #[test]
    fn drift_moves_each_minute_within_range() {
        let at = |h, m, s| drift(TimeOfDay::new(h, m, s).expect("geçerli saat"), 10);
        assert_eq!(at(12, 0, 0), at(12, 0, 59));
        for minute in 0..59 {
            let (x, y) = at(12, minute, 0);
            assert!(x.abs() <= 10 && y.abs() <= 10);
            assert_ne!((x, y), at(12, minute + 1, 0));
        }
    }

    /// 4K (3840x2160) sahne kurulmalı ve kare üretmeli. Ortalama kare süresi
    /// bilgi amaçlı yazdırılır (`cargo test --release -- --nocapture`); süre
    /// makineye bağlı olduğundan testin sonucunu etkilemez.
    #[test]
    fn frame_4k_renders() {
        let fonts = Fonts::embedded().expect("gömülü yazı tipi");
        let (w, h) = (3840u32, 2160u32);
        let mut scene = Scene::new(w, h, &fonts, 7);
        for _ in 0..200 {
            scene.step();
        }
        let mut frame = vec![0u32; (w * h) as usize];
        let count = 25;
        let start = std::time::Instant::now();
        for _ in 0..count {
            scene.step();
            scene.render(&mut frame);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / count as f64;
        println!("4K kare süresi: {ms:.2} ms (bütçe 40 ms)");

        let lit = frame.iter().filter(|&&p| p != 0).count();
        assert!(lit > 10_000, "4K karede yeterli dolu piksel yok: {lit}");
    }
}
