//! Matrix - Kod Yağmuru: işletim sisteminden bağımsız çizim ve zaman yönetimi.
//!
//! Bu kitaplık pencere açmaz, sistem klasörü taramaz ve ekranın uyanık kalmasını
//! istemez. Geçen süreyi, saati ve görüntü alanını onu kullanan program verir.
mod clock;
mod config;
mod engine;
#[cfg(feature = "ffi")]
pub mod ffi;
mod glyphs;
#[cfg(all(feature = "ffi", target_os = "android"))]
mod jni;
mod rain;
mod render;
mod rng;
#[cfg(test)]
mod tests;

pub use engine::{Engine, Stats};
pub use glyphs::Fonts;

use std::fmt;

/// Görüntü ve süre sınırlarında oluşan beklenen hatalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Genişlik veya yükseklik sıfır ya da izin verilenden büyük.
    InvalidSize,
    /// Yazı tipi verisi okunamadı veya gerekli harfleri içermiyor.
    InvalidFont,
    /// Saat, dakika veya saniye aralık dışında.
    InvalidTime,
    /// Görüntü alanı ya da satır uzunluğu çizime yetmiyor.
    ShortBuffer,
    /// Motor durdurulmuşken çizim istendi.
    Paused,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidSize => "Görüntü boyutu geçersiz veya izin verilenden büyük.",
            Self::InvalidFont => "Yazı tipi verisi okunamadı.",
            Self::InvalidTime => "Saat bilgisi geçersiz.",
            Self::ShortBuffer => "Görüntü alanı veya satır uzunluğu yetersiz.",
            Self::Paused => "Çizim duraklatılmış.",
        })
    }
}

impl std::error::Error for Error {}

/// Ekrandaki saatin değeri. Sistem saati dışarıdan gelir; yağmurun geçen
/// süresi bundan etkilenmez.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
    second: u8,
}

impl TimeOfDay {
    /// Saat, dakika ve saniye aralıklarını oluşturmadan önce denetler.
    pub fn new(hour: u8, minute: u8, second: u8) -> Result<Self, Error> {
        if hour > 23 || minute > 59 || second > 59 {
            return Err(Error::InvalidTime);
        }
        Ok(Self {
            hour,
            minute,
            second,
        })
    }
}
