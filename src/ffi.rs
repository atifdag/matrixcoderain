//! C bağlantısı: macOS ve Android gibi Rust dışındaki uygulamalar motoru buradan kullanır.
//!
//! Motorun bellek adresi dışarıya verilmez. Her motor bir kimlik numarasıyla
//! tanınır; kapatılmış veya hiç oluşturulmamış kimlikler güvenle reddedilir.
//! Çağrı sözleşmesi `matrix.h` dosyasında C diliyle de yazılıdır.
use crate::{Engine, Error, Fonts, TimeOfDay};
use std::{
    collections::HashMap,
    panic::{catch_unwind, AssertUnwindSafe},
    slice,
    sync::{Arc, Mutex, OnceLock},
    thread::{self, ThreadId},
    time::Duration,
};

/// Çağrı sözleşmesinin sürümü. Sözleşme değişirse artırılır.
pub const ABI_VERSION: u32 = 1;

// Sonuç kodları; `matrix.h` dosyasındaki açıklamayla aynı olmalıdır.
/// Başarılı.
pub(crate) const OK: i32 = 0;
/// Geçersiz girdi.
pub(crate) const ARGUMENT: i32 = 1;
/// Geçersiz veya kapatılmış motor kimliği.
pub(crate) const HANDLE: i32 = 2;
/// Görüntü alanı veya satır uzunluğu yetersiz.
pub(crate) const BUFFER: i32 = 3;
/// Motor durdurulmuşken çizim istendi.
pub(crate) const PAUSED: i32 = 4;
/// Motor, onu oluşturan iş parçacığı dışından çağrıldı.
pub(crate) const THREAD: i32 = 5;
/// Beklenmeyen Rust hatası yakalandı.
pub(crate) const INTERNAL: i32 = 6;

/// Yazı tipi verisi için kabul edilen en büyük boyut (32 MiB).
const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;

/// Çağrının sonucu: başarı ya da sonuç kodu.
type CallResult = Result<(), i32>;

/// Oluşturulmuş tek bir motor ve ona ait ara görüntü alanı.
struct Instance {
    engine: Engine,
    /// Motorun `0x00RRGGBB` biçiminde çizdiği ara alan; her çizimde yeniden ayrılmaz.
    pixels: Vec<u32>,
    /// Motoru oluşturan iş parçacığı; diğer iş parçacıklarından gelen çağrılar reddedilir.
    owner: ThreadId,
}

/// Bütün motorların kimlik numarasına göre tutulduğu kayıt.
struct Registry {
    /// Sıradaki motor kimliği. Sıfır hiçbir zaman verilmez.
    next: u64,
    engines: HashMap<u64, Instance>,
}

/// Süreç boyunca tek bir kayıt bulunur; ilk kullanımda oluşturulur.
fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Mutex::new(Registry {
            next: 1,
            engines: HashMap::new(),
        })
    })
}

/// Beklenmeyen bir Rust hatasının C tarafına taşmasını önler ve sonucu koda çevirir.
fn boundary(call: impl FnOnce() -> CallResult) -> i32 {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => code,
        Err(_) => INTERNAL,
    }
}

/// Kitaplık hatasını sonuç koduna çevirir.
fn convert(error: Error) -> i32 {
    match error {
        Error::ShortBuffer => BUFFER,
        Error::Paused => PAUSED,
        Error::InvalidSize | Error::InvalidFont | Error::InvalidTime => ARGUMENT,
    }
}

/// Kimliği ve çağıran iş parçacığını doğruladıktan sonra motora erişim verir.
fn with_engine(handle: u64, call: impl FnOnce(&mut Instance) -> CallResult) -> CallResult {
    let mut registry = registry().lock().map_err(|_| INTERNAL)?;
    let instance = registry.engines.get_mut(&handle).ok_or(HANDLE)?;
    if instance.owner != thread::current().id() {
        return Err(THREAD);
    }
    call(instance)
}

/// Bağlantının desteklediği sözleşme sürümünü döndürür.
#[no_mangle]
pub extern "C" fn matrix_abi_version() -> u32 {
    ABI_VERSION
}

/// Yalnız `ffi-test` özelliğiyle derlenen deneme kitaplığında bulunur; dağıtılan uygulamada yoktur.
/// Bilerek hata üreterek bu hatanın C tarafına taşmadığını gösterir.
#[cfg(feature = "ffi-test")]
#[no_mangle]
pub extern "C" fn matrix_test_panic() -> i32 {
    boundary(|| panic!("C bağlantısı hata sınırı denemesi"))
}

/// Motoru duraklatılmış olarak oluşturur ve kimliğini `result` adresine yazar.
/// Sıfır yazı tipi uzunluğu gömülü yazı tiplerini seçer.
///
/// # Güvenlik
/// Yazı tipi adresi belirtilen uzunluk boyunca okunabilir, sonuç adresi
/// yazılabilir olmalıdır. Bu alanlar çağrı bitene kadar geçerli kalmalıdır.
#[no_mangle]
#[allow(
    clippy::missing_safety_doc,
    reason = "Güvenlik koşulları Türkçe Güvenlik bölümünde açıklanır."
)]
pub unsafe extern "C" fn matrix_create(
    version: u32,
    width: u32,
    height: u32,
    seed: u64,
    font: *const u8,
    font_len: usize,
    result: *mut u64,
) -> i32 {
    boundary(|| {
        if version != ABI_VERSION
            || result.is_null()
            || font_len > MAX_FONT_BYTES
            || (font_len > 0 && font.is_null())
        {
            return Err(ARGUMENT);
        }
        // Başarısız oluşturma geçerli bir motor kimliği bırakmamalı.
        unsafe {
            result.write(0);
        }
        let fonts = if font_len == 0 {
            Fonts::embedded().map_err(convert)?
        } else {
            let bytes = unsafe { slice::from_raw_parts(font, font_len) };
            Arc::new(Fonts::from_bytes(Some(bytes)).map_err(convert)?)
        };
        let engine = Engine::new(width, height, fonts, seed).map_err(convert)?;
        let pixels = vec![0; width as usize * height as usize];
        let mut registry = registry().lock().map_err(|_| INTERNAL)?;
        let handle = registry.next;
        registry.next = registry.next.checked_add(1).ok_or(INTERNAL)?;
        registry.engines.insert(
            handle,
            Instance {
                engine,
                pixels,
                owner: thread::current().id(),
            },
        );
        unsafe {
            result.write(handle);
        }
        Ok(())
    })
}

/// Çizimi başlatır (1) veya durdurur (0); diğer değerler kabul edilmez.
#[no_mangle]
pub extern "C" fn matrix_set_active(handle: u64, active: u32) -> i32 {
    boundary(|| {
        if active > 1 {
            return Err(ARGUMENT);
        }
        with_engine(handle, |i| {
            i.engine.set_active(active == 1);
            Ok(())
        })
    })
}

/// Geçen süre kadar yağmuru ilerletir ve saati günceller.
/// Süre mikrosaniyedir; saat bilgisinin geçerliliği ayrıca denetlenir.
#[no_mangle]
pub extern "C" fn matrix_update(
    handle: u64,
    elapsed_us: u64,
    hour: u8,
    minute: u8,
    second: u8,
) -> i32 {
    boundary(|| {
        let time = TimeOfDay::new(hour, minute, second).map_err(convert)?;
        with_engine(handle, |i| {
            i.engine.update(Duration::from_micros(elapsed_us), time);
            Ok(())
        })
    })
}

/// Görüntü boyutunu değiştirir. Geçersiz boyut mevcut motoru ve alanı değiştirmez.
#[no_mangle]
pub extern "C" fn matrix_resize(handle: u64, width: u32, height: u32) -> i32 {
    boundary(|| {
        with_engine(handle, |i| {
            i.engine.resize(width, height).map_err(convert)?;
            i.pixels.resize(width as usize * height as usize, 0);
            Ok(())
        })
    })
}

/// Görüntüyü her piksel için kırmızı, yeşil, mavi ve 255 baytlarıyla yazar.
/// `stride` bir satırın bayt uzunluğudur; satır sonundaki boşluklara dokunulmaz.
///
/// # Güvenlik
/// Hedef alan belirtilen uzunluk boyunca yazılabilir ve çağrı boyunca geçerli
/// olmalıdır. Alanı aynı anda başka iş parçacığı okumamalı veya değiştirmemelidir.
#[no_mangle]
#[allow(
    clippy::missing_safety_doc,
    reason = "Güvenlik koşulları Türkçe Güvenlik bölümünde açıklanır."
)]
pub unsafe extern "C" fn matrix_render_rgba(
    handle: u64,
    target: *mut u8,
    len: usize,
    stride: usize,
) -> i32 {
    boundary(|| {
        with_engine(handle, |i| {
            let (w, h) = i.engine.size();
            let row = w as usize * 4;
            // Son satırın yalnız piksel kısmının sığması yeterlidir.
            let needed = stride
                .checked_mul(h as usize - 1)
                .and_then(|n| n.checked_add(row))
                .ok_or(BUFFER)?;
            if target.is_null() || stride < row || len < needed || len > isize::MAX as usize {
                return Err(BUFFER);
            }
            i.engine.render(&mut i.pixels).map_err(convert)?;
            let out = unsafe { slice::from_raw_parts_mut(target, len) };
            for (y, source) in i.pixels.chunks_exact(w as usize).enumerate() {
                let line = &mut out[y * stride..y * stride + row];
                for (pixel, rgba) in source.iter().zip(line.as_chunks_mut::<4>().0) {
                    *rgba = [(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8, 255];
                }
            }
            Ok(())
        })
    })
}

/// Sayaçları ve son karelerin çizim sürelerini Türkçe tek satır olarak yazar.
/// Metin UTF-8'dir ve sonuna sıfır baytı eklenir; 256 bayt her zaman yeterlidir.
///
/// # Güvenlik
/// Hedef alan belirtilen uzunluk boyunca yazılabilir ve çağrı boyunca geçerli olmalıdır.
#[no_mangle]
#[allow(
    clippy::missing_safety_doc,
    reason = "Güvenlik koşulları Türkçe Güvenlik bölümünde açıklanır."
)]
pub unsafe extern "C" fn matrix_stats_text(handle: u64, text: *mut u8, len: usize) -> i32 {
    boundary(|| {
        with_engine(handle, |i| {
            let line = i.engine.stats().to_string();
            let bytes = line.as_bytes();
            if text.is_null() || len <= bytes.len() {
                return Err(BUFFER);
            }
            let out = unsafe { slice::from_raw_parts_mut(text, len) };
            out[..bytes.len()].copy_from_slice(bytes);
            out[bytes.len()] = 0;
            Ok(())
        })
    })
}

/// Motoru kapatır. Aynı kimliği tekrar kapatmak hata döndürür; başka motoru etkilemez.
#[no_mangle]
pub extern "C" fn matrix_destroy(handle: u64) -> i32 {
    boundary(|| {
        let mut registry = registry().lock().map_err(|_| INTERNAL)?;
        let instance = registry.engines.get(&handle).ok_or(HANDLE)?;
        if instance.owner != thread::current().id() {
            return Err(THREAD);
        }
        registry.engines.remove(&handle);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 32x24 boyutunda, gömülü yazı tipli bir motor oluşturur.
    fn create() -> u64 {
        let mut id = 0;
        assert_eq!(
            unsafe { matrix_create(1, 32, 24, 9, std::ptr::null(), 0, &mut id) },
            OK
        );
        id
    }

    /// Görüntü alanının dışına ve satır sonu boşluklarına yazılmamalı;
    /// geçersiz girdiler ve kapatılmış kimlik doğru sonuç kodunu almalı.
    #[test]
    fn handles_buffers_and_shutdown() {
        let id = create();
        let mut buffer = vec![77; 2 + 24 * 136 + 2];
        assert_eq!(matrix_set_active(id, 1), OK);
        assert_eq!(matrix_update(id, 40_000, 23, 59, 59), OK);
        let ptr = unsafe { buffer.as_mut_ptr().add(2) };
        assert_eq!(unsafe { matrix_render_rgba(id, ptr, 24 * 136, 136) }, OK);
        assert_eq!(&buffer[..2], &[77, 77]);
        assert_eq!(&buffer[buffer.len() - 2..], &[77, 77]);
        for row in buffer[2..2 + 24 * 136].as_chunks::<136>().0 {
            assert!(row[..128].as_chunks::<4>().0.iter().all(|p| p[3] == 255));
            assert!(row[128..].iter().all(|&b| b == 77));
        }
        assert_eq!(unsafe { matrix_render_rgba(id, ptr, 12, 136) }, BUFFER);
        assert_eq!(
            unsafe { matrix_render_rgba(id, ptr, buffer.len(), usize::MAX) },
            BUFFER
        );
        assert_eq!(matrix_resize(id, u32::MAX, u32::MAX), ARGUMENT);
        assert_eq!(matrix_update(id, 1, 24, 0, 0), ARGUMENT);
        assert_eq!(matrix_set_active(id, 0), OK);
        assert_eq!(
            unsafe { matrix_render_rgba(id, ptr, 24 * 136, 136) },
            PAUSED
        );
        assert_eq!(matrix_destroy(id), OK);
        assert_eq!(matrix_destroy(id), HANDLE);
        assert_eq!(matrix_update(id, 1, 0, 0, 0), HANDLE);
    }

    /// Başka iş parçacığından gelen çağrı, yanlış sürüm, sıfır kimlik ve
    /// beklenmeyen hata güvenle reddedilmeli.
    #[test]
    fn caller_thread_and_invalid_input() {
        let id = create();
        assert_eq!(
            std::thread::spawn(move || matrix_set_active(id, 1))
                .join()
                .unwrap(),
            THREAD
        );
        assert_eq!(matrix_destroy(id), OK);
        assert_eq!(
            unsafe { matrix_create(99, 1, 1, 0, std::ptr::null(), 0, std::ptr::null_mut()) },
            ARGUMENT
        );
        assert_eq!(matrix_set_active(0, 1), HANDLE);
        assert_eq!(boundary(|| panic!("deneme hatası")), INTERNAL);
    }

    /// Boyut değişikliğinden sonra çizim yeni boyuta göre yapılmalı.
    #[test]
    fn resize_changes_required_area() {
        let id = create();
        assert_eq!(matrix_set_active(id, 1), OK);
        assert_eq!(matrix_resize(id, 8, 4), OK);
        let mut buffer = vec![0u8; 8 * 4 * 4];
        let ptr = buffer.as_mut_ptr();
        assert_eq!(unsafe { matrix_render_rgba(id, ptr, buffer.len(), 32) }, OK);
        assert_eq!(matrix_resize(id, 16, 4), OK);
        assert_eq!(
            unsafe { matrix_render_rgba(id, ptr, buffer.len(), 32) },
            BUFFER
        );
        assert_eq!(matrix_destroy(id), OK);
    }

    /// Özet metni sıfırla bitmeli, kısa alanı reddetmeli ve çizim sayısını içermeli.
    #[test]
    fn stats_text() {
        let id = create();
        assert_eq!(matrix_set_active(id, 1), OK);
        let mut pixels = vec![0u8; 32 * 24 * 4];
        assert_eq!(
            unsafe { matrix_render_rgba(id, pixels.as_mut_ptr(), pixels.len(), 128) },
            OK
        );
        let mut text = [0xFFu8; 256];
        assert_eq!(
            unsafe { matrix_stats_text(id, text.as_mut_ptr(), text.len()) },
            OK
        );
        let end = text
            .iter()
            .position(|&b| b == 0)
            .expect("sonu sıfır olmalı");
        let line = std::str::from_utf8(&text[..end]).expect("geçerli UTF-8");
        assert!(line.contains("1 kare"), "{line}");
        assert_eq!(
            unsafe { matrix_stats_text(id, text.as_mut_ptr(), 4) },
            BUFFER
        );
        assert_eq!(matrix_destroy(id), OK);
    }
}
