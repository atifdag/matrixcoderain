//! Çizim sıklığını yağmurun ilerleyişinden ayırır; duraklamada geçen süreyi biriktirmez.
use crate::{render::Scene, Error, Fonts, TimeOfDay};
use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

/// Yağmurun bir adımı. Eski 25 kare/saniye görünümünü korur; ekran saniyede
/// kaç kez çizilirse çizilsin yağmur aynı hızda akar.
const STEP: Duration = Duration::from_millis(40);
/// Tek çağrıda kabul edilen en uzun süre. Uzun bir bekleme, yüzlerce adımın
/// bir anda hesaplanmasına ve yağmurun ileri sıçramasına yol açmasın.
const MAX_ELAPSED: Duration = Duration::from_millis(250);
/// En büyük görüntü 16.777.216 pikseldir (yaklaşık 64 MiB bellek).
pub const MAX_PIXELS: usize = 16_777_216;
/// Bir kenar en çok bu kadar piksel olabilir.
const MAX_SIDE: u32 = 16_384;
/// Çizim süresi istatistiği son bu kadar kareden hesaplanır.
const TIMING_SAMPLES: usize = 256;

/// Çizim ve yağmur adımı sayıları ile son karelerin çizim süreleri.
/// Duraklamada iş yapılmadığını ve çizimin ne kadar sürdüğünü göstermek içindir.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Bugüne kadar hesaplanan yağmur adımı sayısı.
    pub steps: u64,
    /// Bugüne kadar çizilen görüntü sayısı.
    pub frames: u64,
    /// Son karelerin ortanca çizim süresi, mikrosaniye.
    pub render_p50_us: u32,
    /// Son karelerin yüzde 95'inin altında kaldığı çizim süresi, mikrosaniye.
    pub render_p95_us: u32,
}

/// Kayıtlarda görünen tek satırlık özet.
impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} yağmur adımı, {} kare; çizim süresi ortanca {:.1} ms, yüzde 95 {:.1} ms",
            self.steps,
            self.frames,
            f64::from(self.render_p50_us) / 1000.0,
            f64::from(self.render_p95_us) / 1000.0,
        )
    }
}

/// Tek bir ekranın yağmur, saat ve imza çizimini yönetir; pencere açmaz.
/// Her motoru tek bir sahip kullanır; aynı anda birden çok yerden çağrılmaz.
pub struct Engine {
    scene: Scene,
    fonts: Arc<Fonts>,
    seed: u64,
    /// Henüz bir adıma dönüşmemiş geçen süre.
    pending: Duration,
    active: bool,
    steps: u64,
    frames: u64,
    /// Son karelerin çizim süreleri, mikrosaniye; dolunca en eskisinin üzerine yazılır.
    render_times: Vec<u32>,
}

impl Engine {
    /// Boyutları doğrular ve motoru duraklatılmış olarak hazırlar.
    /// Aynı boyut, yazı tipi ve başlangıç sayısı her zaman aynı yağmuru üretir.
    pub fn new(width: u32, height: u32, fonts: Arc<Fonts>, seed: u64) -> Result<Self, Error> {
        validate_size(width, height)?;
        Ok(Self {
            scene: Scene::new(width, height, &fonts, seed),
            fonts,
            seed,
            pending: Duration::ZERO,
            active: false,
            steps: 0,
            frames: 0,
            render_times: Vec::with_capacity(TIMING_SAMPLES),
        })
    }

    /// Geçerli görüntü boyutunu piksel cinsinden verir.
    pub fn size(&self) -> (u32, u32) {
        (self.scene.width(), self.scene.height())
    }

    /// Sayaçları ve son karelerin çizim sürelerini iç durumu değiştirmeden okur.
    pub fn stats(&self) -> Stats {
        let mut sorted = self.render_times.clone();
        sorted.sort_unstable();
        Stats {
            steps: self.steps,
            frames: self.frames,
            render_p50_us: percentile(&sorted, 50),
            render_p95_us: percentile(&sorted, 95),
        }
    }

    /// Çizimi başlatır veya durdurur. Durum değişince birikmiş süre silinir;
    /// böylece yeniden başlarken duraklamada geçen süre hesaplanmaz.
    pub fn set_active(&mut self, active: bool) {
        if self.active != active {
            self.pending = Duration::ZERO;
        }
        self.active = active;
    }

    /// Yeni boyut için sahneyi baştan kurar. Boyut geçersizse mevcut sahne korunur.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        validate_size(width, height)?;
        if self.size() != (width, height) {
            self.scene = Scene::new(width, height, &self.fonts, self.seed);
            self.pending = Duration::ZERO;
        }
        Ok(())
    }

    /// Geçen süre kadar yağmuru ilerletir ve saati günceller. Motor durmuşsa
    /// hiçbir şey yapmaz. Tek çağrıda en çok 250 milisaniye hesaba katılır.
    pub fn update(&mut self, elapsed: Duration, time: TimeOfDay) {
        if !self.active {
            return;
        }
        self.scene.set_time(time);
        self.pending += elapsed.min(MAX_ELAPSED);
        while self.pending >= STEP {
            self.scene.step();
            self.pending -= STEP;
            self.steps += 1;
        }
    }

    /// Hazır durumu `0x00RRGGBB` biçimindeki görüntü alanına çizer.
    /// Yağmuru veya rastgele sayı sırasını ilerletmez; aynı durum tekrar
    /// çizildiğinde aynı görüntü çıkar.
    pub fn render(&mut self, target: &mut [u32]) -> Result<(), Error> {
        if !self.active {
            return Err(Error::Paused);
        }
        let (w, h) = self.size();
        if target.len() < w as usize * h as usize {
            return Err(Error::ShortBuffer);
        }
        let started = Instant::now();
        self.scene.render(target);
        self.record_render_time(started.elapsed());
        self.frames += 1;
        Ok(())
    }

    /// Çizim süresini kaydeder; kayıt dolduysa en eskisinin yerine yazar.
    fn record_render_time(&mut self, duration: Duration) {
        let micros = u32::try_from(duration.as_micros()).unwrap_or(u32::MAX);
        if self.render_times.len() < TIMING_SAMPLES {
            self.render_times.push(micros);
        } else {
            self.render_times[(self.frames % TIMING_SAMPLES as u64) as usize] = micros;
        }
    }
}

/// Sıralı değerlerden yüzdelik değeri verir (en yakın sıra yöntemi); boşsa sıfır.
fn percentile(sorted: &[u32], percent: usize) -> u32 {
    match sorted.len() {
        0 => 0,
        n => sorted[(n - 1) * percent / 100],
    }
}

/// Boyutu, bellek ayrılmadan önce denetler ve toplam piksel sayısını verir.
/// Çarpım taşarsa veya sınırı aşarsa hata döner.
pub fn validate_size(width: u32, height: u32) -> Result<usize, Error> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(Error::InvalidSize);
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(Error::InvalidSize)?;
    if pixels > MAX_PIXELS {
        return Err(Error::InvalidSize);
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::percentile;

    /// Yüzdelik değerler sıralı listeden doğru seçilmeli.
    #[test]
    fn percentiles() {
        let values: Vec<u32> = (1..=100).collect();
        assert_eq!(percentile(&values, 50), 50);
        assert_eq!(percentile(&values, 95), 95);
        assert_eq!(percentile(&[], 95), 0);
        assert_eq!(percentile(&[7], 50), 7);
    }
}
