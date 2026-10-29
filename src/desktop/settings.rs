//! Kullanıcıya görünen ad ve masaüstü pencere ayarları.

/// Pencere başlığında, mesaj kutularında ve çıktı dosyalarında görünen ad.
pub const APP_NAME: &str = "Matrix - Kod Yağmuru";
/// İki çizim arasındaki süre, milisaniye (saniyede 25 kare).
pub const FRAME_MS: u64 = 40;
/// Ekran koruyucuyu kapatmak için farenin kaç piksel oynaması gerektiği.
/// Masadaki küçük titreşimler ekran koruyucuyu kapatmasın.
pub const MOUSE_THRESHOLD: f64 = 5.0;
/// Deneme penceresinin ilk genişliği.
pub const WINDOW_WIDTH: u32 = 800;
/// Deneme penceresinin ilk yüksekliği.
pub const WINDOW_HEIGHT: u32 = 600;
