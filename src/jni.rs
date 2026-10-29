//! Android bağlantısı: Kotlin'deki `MatrixNative` nesnesinin çağrılarını C bağlantısına iletir.
//!
//! Burada çizim veya denetim kuralı yoktur; Java türleri C bağlantısının
//! türlerine çevrilir ve sonuç kodu olduğu gibi geri verilir. Böylece kurallar
//! yalnız `ffi.rs` içinde kalır.
use crate::ffi;
use jni_sys::{jboolean, jint, jlong, jobject, jstring, JNIEnv};

/// Java tam sayısını işaretsiz sayıya çevirir; eksi değerler geçersiz sayılır.
fn unsigned(value: jint) -> Option<u32> {
    u32::try_from(value).ok()
}

/// Java tam sayısını saat bileşenine çevirir; aralık denetimi C bağlantısındadır.
fn time_part(value: jint) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

/// Gömülü yazı tipleriyle, durdurulmuş bir motor oluşturur.
/// Başarıda motor kimliğini (sıfırdan büyük), hatada sonuç kodunun eksi değerini döndürür.
#[no_mangle]
pub extern "system" fn Java_com_atifdag_matrixcoderain_MatrixNative_create(
    _env: *mut JNIEnv,
    _this: jobject,
    width: jint,
    height: jint,
    seed: jlong,
) -> jlong {
    let (Some(width), Some(height)) = (unsigned(width), unsigned(height)) else {
        return -jlong::from(ffi::ARGUMENT);
    };
    let mut handle = 0u64;
    // Yazı tipi verilmediğinden yalnız sonuç adresi kullanılır; o da bu çağrıya aittir.
    let code = unsafe {
        ffi::matrix_create(
            ffi::ABI_VERSION,
            width,
            height,
            seed as u64,
            std::ptr::null(),
            0,
            &mut handle,
        )
    };
    if code == ffi::OK {
        handle as jlong
    } else {
        -jlong::from(code)
    }
}

/// Çizimi başlatır veya durdurur.
#[no_mangle]
pub extern "system" fn Java_com_atifdag_matrixcoderain_MatrixNative_setActive(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    active: jboolean,
) -> jint {
    ffi::matrix_set_active(handle as u64, u32::from(active))
}

/// Geçen süreyi (mikrosaniye) ve ekrandaki saati verir.
#[no_mangle]
pub extern "system" fn Java_com_atifdag_matrixcoderain_MatrixNative_update(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    elapsed_micros: jlong,
    hour: jint,
    minute: jint,
    second: jint,
) -> jint {
    let Ok(elapsed) = u64::try_from(elapsed_micros) else {
        return ffi::ARGUMENT;
    };
    ffi::matrix_update(
        handle as u64,
        elapsed,
        time_part(hour),
        time_part(minute),
        time_part(second),
    )
}

/// Görüntüyü Kotlin'in ayırdığı doğrudan bellekli `ByteBuffer` alanına yazar.
///
/// # Güvenlik
/// `env` geçerli bir Java ortamı, `buffer` doğrudan bellekli bir `ByteBuffer`
/// olmalıdır. Alan, çağrı boyunca başka iş parçacığı tarafından kullanılmamalıdır.
#[no_mangle]
#[allow(
    clippy::missing_safety_doc,
    reason = "Güvenlik koşulları Türkçe Güvenlik bölümünde açıklanır."
)]
pub unsafe extern "system" fn Java_com_atifdag_matrixcoderain_MatrixNative_render(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    buffer: jobject,
    stride: jint,
) -> jint {
    if env.is_null() || buffer.is_null() {
        return ffi::ARGUMENT;
    }
    let Ok(stride) = usize::try_from(stride) else {
        return ffi::ARGUMENT;
    };
    // Java ortamının işlev tablosu; doğrudan bellek işlevleri 1.4 sürümünden beri vardır.
    let table = unsafe { (**env).v1_4 };
    let address = unsafe { (table.GetDirectBufferAddress)(env, buffer) }.cast::<u8>();
    let capacity = unsafe { (table.GetDirectBufferCapacity)(env, buffer) };
    let Ok(capacity) = usize::try_from(capacity) else {
        return ffi::ARGUMENT;
    };
    // Adres ve uzunluk Java'dan geldi; kalan denetimler C bağlantısında yapılır.
    unsafe { ffi::matrix_render_rgba(handle as u64, address, capacity, stride) }
}

/// Sayaçları ve çizim sürelerini Türkçe tek satır olarak döndürür; hata olursa boş değer.
///
/// # Güvenlik
/// `env` geçerli bir Java ortamı olmalıdır.
#[no_mangle]
#[allow(
    clippy::missing_safety_doc,
    reason = "Güvenlik koşulları Türkçe Güvenlik bölümünde açıklanır."
)]
pub unsafe extern "system" fn Java_com_atifdag_matrixcoderain_MatrixNative_statsText(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jstring {
    let mut text = [0u8; 256];
    let code = unsafe { ffi::matrix_stats_text(handle as u64, text.as_mut_ptr(), text.len()) };
    if env.is_null() || code != ffi::OK {
        return std::ptr::null_mut();
    }
    // Metindeki Türkçe harfler Java'nın beklediği UTF-8 biçimiyle aynıdır.
    unsafe { ((**env).v1_1.NewStringUTF)(env, text.as_ptr().cast()) }
}

/// Motoru kapatır.
#[no_mangle]
pub extern "system" fn Java_com_atifdag_matrixcoderain_MatrixNative_destroy(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    ffi::matrix_destroy(handle as u64)
}
