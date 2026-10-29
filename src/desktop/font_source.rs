//! Yalnız kullanıcının açıkça seçtiği dosyayı okur; sistem klasörlerini taramaz.
use matrix_core::Fonts;
use std::{path::Path, sync::Arc};

/// Dosya verilmezse programa gömülü iki yazı tipini kullanır. Dosya okunamaz
/// veya geçersizse kullanıcıya gösterilecek açıklamayı döndürür.
pub fn load(explicit: Option<&Path>) -> Result<Arc<Fonts>, String> {
    match explicit {
        None => Fonts::embedded().map_err(|e| e.to_string()),
        Some(path) => {
            let data = std::fs::read(path).map_err(|e| format!("Yazı tipi okunamadı: {e}"))?;
            Fonts::from_bytes(Some(&data))
                .map(Arc::new)
                .map_err(|e| e.to_string())
        }
    }
}
