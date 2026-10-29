//! Ekranın ortasındaki yedi parçalı LED görünümlü dijital saat.
//!
//! Saat, saydam arka planlı küçük bir resme çizilir ve yalnızca saniye
//! değiştiğinde yeniden çizilir. Her karede bu hazır resim yağmurun üzerine bindirilir.
//! Parçaların çokgenleri başlangıçta bir kez hazırlanır; sonraki çizimlerde yalnızca
//! yerleri kaydırılır. Çokgenlerin kenarları `tiny-skia` ile yumuşatılır; yazı tipi gerekmez.

use crate::TimeOfDay;
use tiny_skia::{Color, FillRule, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

use crate::config::{CLOCK, CLOCK_HALO, CLOCK_SLANT, SEGMENTS};
use crate::render::{blend_premultiplied, ImageRef};

/// Saat şekillerini saklar; aynı saniyede tekrar hazırlamaz.
pub struct Clock {
    /// Rakam genişliği, piksel.
    w: f32,
    /// Rakamlar arasındaki boşluk.
    gap: f32,
    /// İki nokta işaretinin genişliği.
    colon_w: f32,
    /// Resmin kenar payı; kontur resmin dışına taşmasın diye.
    pad: f32,
    /// Saat resminin kare içindeki sol üst köşesi.
    x: i32,
    y: i32,
    /// Saat resmi (saydam arka plan, saydamlığı önceden çarpılmış RGBA).
    pixmap: Pixmap,
    /// En son çizilen saat, dakika, saniye; aynı saniye içinde yeniden çizim yapılmaz.
    last: Option<(u32, u32, u32)>,
    /// Yedi parçanın çokgenleri (a b c d e f g sırasıyla), rakamın sol üst köşesine göre.
    segments: [Path; 7],
    /// "1" rakamı için hücrenin ortasına kaydırılmış sağ parçalar (b ve c).
    one_segments: [Path; 2],
    /// İki nokta işaretinin iki karesi.
    colon: [Path; 2],
    /// Yanan parça rengi, kontur rengi ve kontur kalınlığı.
    fill: Paint<'static>,
    halo: Paint<'static>,
    stroke: Stroke,
}

impl Clock {
    /// Verilen pencere boyutuna göre saatin yerini ve büyüklüğünü hesaplar,
    /// çokgenleri hazırlar.
    pub fn new(width: u32, height: u32) -> Clock {
        let fw = width.max(1) as f32;
        let fh = height.max(1) as f32;

        // Rakam yüksekliği pencere yüksekliğinin yaklaşık 1/12'si. "SS:DD:ss" dizisi
        // rakam yüksekliğinin yaklaşık 4,9 katı genişliğinde olduğundan, tamamı
        // sığsın diye genişliğe göre de sınırlanır.
        let h = (fh / 12.0).min(fw * 0.09).max(8.0);
        let w = h * 0.55;
        let t = h * 0.14;
        let gap = w * 0.3;
        let colon_w = t;
        let total = 6.0 * w + 2.0 * colon_w + 7.0 * gap;
        let slant = h * CLOCK_SLANT;
        let halo_px = (h * 0.03).max(1.5);
        let pad = halo_px + 1.0;

        let pm_w = ((total + slant + 2.0 * pad).ceil() as u32).max(1);
        let pm_h = ((h + 2.0 * pad).ceil() as u32).max(1);
        let pixmap = Pixmap::new(pm_w, pm_h).expect("saat resmi oluşturulamadı");

        let x = ((fw - total - slant) / 2.0 - pad).round() as i32;
        let y = ((fh - h) / 2.0 - pad).round() as i32;

        // Parça geometrisi.
        let ht = t / 2.0; // yarım kalınlık
        let gp = t * 0.12; // komşu parça uçları arasındaki boşluk
        let mid = h / 2.0;
        let segments = [
            h_seg(ht + gp, w - ht - gp, ht, t),
            v_seg(w - ht, ht + gp, mid - gp, t),
            v_seg(w - ht, mid + gp, h - ht - gp, t),
            h_seg(ht + gp, w - ht - gp, h - ht, t),
            v_seg(ht, mid + gp, h - ht - gp, t),
            v_seg(ht, ht + gp, mid - gp, t),
            h_seg(ht + gp, w - ht - gp, mid, t),
        ]
        .map(|pts| polygon(&pts, h));

        // "1" rakamı yalnızca sağdaki iki parçayı kullanır; hücrenin sağ kenarına
        // yaslanırsa solunda büyük bir boşluk kalır ve iki noktanın çevresindeki
        // aralıklar dengesiz görünür. Bu yüzden "1" hücrenin ortasına çizilir.
        let one_segments = [
            v_seg(w / 2.0, ht + gp, mid - gp, t),
            v_seg(w / 2.0, mid + gp, h - ht - gp, t),
        ]
        .map(|pts| polygon(&pts, h));

        let colon = [h * 0.32, h * 0.68].map(|cy| {
            let pts = [
                (0.0, cy - t / 2.0),
                (t, cy - t / 2.0),
                (t, cy + t / 2.0),
                (0.0, cy + t / 2.0),
            ];
            polygon(&pts, h)
        });

        let mut fill = Paint::default();
        fill.set_color_rgba8(CLOCK.0, CLOCK.1, CLOCK.2, 255);
        fill.anti_alias = true;
        let mut halo = Paint::default();
        halo.set_color_rgba8(CLOCK_HALO.0, CLOCK_HALO.1, CLOCK_HALO.2, 255);
        halo.anti_alias = true;
        let stroke = Stroke {
            width: halo_px,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        Clock {
            w,
            gap,
            colon_w,
            pad,
            x,
            y,
            pixmap,
            last: None,
            segments,
            one_segments,
            colon,
            fill,
            halo,
            stroke,
        }
    }

    /// Dışarıdan verilen saat değiştiyse resmi yeniden çizer.
    pub fn update(&mut self, time: TimeOfDay) {
        let key = (
            u32::from(time.hour),
            u32::from(time.minute),
            u32::from(time.second),
        );
        if self.last == Some(key) {
            return;
        }
        self.last = Some(key);

        let digits = [
            key.0 / 10,
            key.0 % 10,
            key.1 / 10,
            key.1 % 10,
            key.2 / 10,
            key.2 % 10,
        ];

        self.pixmap.fill(Color::TRANSPARENT);
        let mut x = 0.0;
        for (i, &d) in digits.iter().enumerate() {
            self.draw_digit(x, SEGMENTS[d as usize]);
            x += self.w + self.gap;
            // Saat-dakika ve dakika-saniye arasına iki nokta.
            if i == 1 || i == 3 {
                self.draw_colon(x);
                x += self.colon_w + self.gap;
            }
        }
    }

    /// Hazır saat resmini, ortadaki yerinden `offset` kadar kaydırarak karenin üzerine bindirir.
    pub fn blit(&self, frame: &mut [u32], fw: i32, fh: i32, offset: (i32, i32)) {
        let img = ImageRef {
            w: self.pixmap.width() as usize,
            h: self.pixmap.height() as usize,
            data: self.pixmap.data(),
        };
        blend_premultiplied(frame, fw, fh, self.x + offset.0, self.y + offset.1, img);
    }

    /// Tek bir rakamı çizer; `mask` hangi parçaların yanacağını söyler.
    fn draw_digit(&mut self, x: f32, mask: u8) {
        let one = mask == SEGMENTS[1];
        for bit in 0..7 {
            if mask & (1 << bit) == 0 {
                continue;
            }
            let path = match (one, bit) {
                (true, 1) => &self.one_segments[0],
                (true, 2) => &self.one_segments[1],
                _ => &self.segments[bit],
            };
            stamp(
                &mut self.pixmap,
                path,
                &self.halo,
                &self.stroke,
                &self.fill,
                self.pad + x,
                self.pad,
            );
        }
    }

    /// İki nokta işaretini çizer (iki küçük kare).
    fn draw_colon(&mut self, x: f32) {
        for path in &self.colon {
            stamp(
                &mut self.pixmap,
                path,
                &self.halo,
                &self.stroke,
                &self.fill,
                self.pad + x,
                self.pad,
            );
        }
    }
}

/// Hazır bir çokgeni verilen kaydırmayla çizer: önce koyu kontur, sonra üzerine
/// yanan parça rengi.
fn stamp(
    pixmap: &mut Pixmap,
    path: &Path,
    halo: &Paint,
    stroke: &Stroke,
    fill: &Paint,
    dx: f32,
    dy: f32,
) {
    let shift = Transform::from_translate(dx, dy);
    pixmap.stroke_path(path, halo, stroke, shift, None);
    pixmap.fill_path(path, fill, FillRule::Winding, shift, None);
}

/// Köşe noktalarından kapalı bir çokgen yolu kurar; eğim burada uygulanır.
fn polygon(pts: &[(f32, f32)], h: f32) -> Path {
    let mut pb = PathBuilder::new();
    for (i, &(px, py)) in pts.iter().enumerate() {
        let x = px + (h - py) * CLOCK_SLANT;
        if i == 0 {
            pb.move_to(x, py);
        } else {
            pb.line_to(x, py);
        }
    }
    pb.close();
    pb.finish().expect("saat çokgeni oluşturulamadı")
}

/// `xa`'dan `xb`'ye uzanan, `yc` üzerinde ortalanmış, uçları sivri yatay parça.
fn h_seg(xa: f32, xb: f32, yc: f32, t: f32) -> [(f32, f32); 6] {
    let ht = t / 2.0;
    [
        (xa, yc),
        (xa + ht, yc - ht),
        (xb - ht, yc - ht),
        (xb, yc),
        (xb - ht, yc + ht),
        (xa + ht, yc + ht),
    ]
}

/// `ya`'dan `yb`'ye uzanan, `xc` üzerinde ortalanmış, uçları sivri dikey parça.
fn v_seg(xc: f32, ya: f32, yb: f32, t: f32) -> [(f32, f32); 6] {
    let ht = t / 2.0;
    [
        (xc, ya),
        (xc + ht, ya + ht),
        (xc + ht, yb - ht),
        (xc, yb),
        (xc - ht, yb - ht),
        (xc - ht, ya + ht),
    ]
}
