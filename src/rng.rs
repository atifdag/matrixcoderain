//! Küçük ve hızlı rastgele sayı üreteci (xorshift64).
//!
//! Dış bir kitaplık eklememek için kendi üretecimizi kullanıyoruz; görsel bir
//! etki için yeterince rastgeledir.

/// Görsel seçimler için hızlı üreteçtir; güvenlik amaçlı kullanılmaz.
pub struct Rng(u64);

impl Rng {
    /// Sıfır başlangıç sayısı üreteci kilitlemesin; diğer sayılar aynen korunur.
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// Bir sonraki 64 bitlik rastgele sayı.
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// `lo..hi` aralığında tam sayı (`hi` dahil değil, `hi > lo` olmalı).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % ((hi - lo).max(1) as u64)) as i32
    }

    /// `0.0..1.0` aralığında ondalık sayı.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}
