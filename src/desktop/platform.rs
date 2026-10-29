//! İşletim sistemine özel küçük yardımcılar ve sorun giderme kaydı.
//!
//! İki iş işletim sistemine özeldir: kullanıcıya kısa bir mesaj göstermek ve
//! başka bir programın verdiği pencereye (Windows önizlemesi, xscreensaver)
//! yerleşmek. Hata ayıklama derlemesinde Linux'ta ayrıca xscreensaver'ın
//! kapatma sinyali olay döngüsüne iletilir. Programın geri kalanı işletim
//! sisteminden bağımsızdır.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::RawWindowHandle;

/// Başka programın verdiği pencere: bağlanma bilgisi, genişlik ve yükseklik.
pub type EmbedTarget = (RawWindowHandle, u32, u32);

/// Sorun giderme kaydı. Yalnız `MATRIXCODERAIN_LOG` ortam değişkeni verilmişse
/// çalışır; normal kullanımda hiçbir dosya yazılmaz ve konsola bir şey basılmaz.
///
/// - `MATRIXCODERAIN_LOG=1` → geçici klasördeki `matrixcoderain.log` (Windows'ta `%TEMP%`)
/// - `MATRIXCODERAIN_LOG=/yol/kayit.log` → verilen dosya
///
/// Satırlar dosyanın sonuna eklenir. Böylece aynı anda ya da art arda çalışan
/// programlar (ör. xscreensaver'da her ekranın ayrı programı) birbirinin
/// satırlarını silmez. Dosya yazılamazsa sessizce atlanır.
pub fn log(message: &str) {
    static FILE: OnceLock<Option<Mutex<File>>> = OnceLock::new();
    let file = FILE.get_or_init(|| {
        let value = std::env::var_os("MATRIXCODERAIN_LOG")?;
        let path = if value.is_empty() || value == "1" {
            std::env::temp_dir().join("matrixcoderain.log")
        } else {
            PathBuf::from(value)
        };
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(Mutex::new)
    });
    if let Some(file) = file {
        if let Ok(mut f) = file.lock() {
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            // Satır tek seferde yazılır; başka programın satırıyla karışmaz.
            let _ = f.write_all(format!("{now} {message}\n").as_bytes());
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::EmbedTarget;
    use std::ffi::c_void;
    use std::num::NonZeroIsize;

    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, IsWindow, MessageBoxW, MB_ICONINFORMATION, MB_OK,
    };
    use winit::raw_window_handle::{RawWindowHandle, Win32WindowHandle};

    /// Rust metnini Windows'un beklediği, sonu sıfırla biten UTF-16 diziye çevirir.
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Basit bir mesaj kutusu gösterir. Program konsolsuz derlendiği için
    /// Windows'ta metin göstermenin tek yolu budur.
    pub fn show_message(title: &str, text: &str) {
        super::log(&format!("{title}: {text}"));
        let title = wide(title);
        let text = wide(text);
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    /// Windows ekran koruyucu ayarlarındaki önizleme penceresini doğrular ve
    /// iç alanının boyutunu okur.
    pub fn embed_target(id: u64) -> Option<EmbedTarget> {
        let hwnd = NonZeroIsize::new(isize::try_from(id).ok()?)?;
        let handle = hwnd.get() as *mut c_void;
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // Pencere kimliği dışarıdan geldiği için önce sistemden doğrulanır.
        let valid = unsafe { IsWindow(handle) != 0 && GetClientRect(handle, &mut rc) != 0 };
        if !valid {
            return None;
        }
        Some((
            RawWindowHandle::Win32(Win32WindowHandle::new(hwnd)),
            (rc.right - rc.left).max(1) as u32,
            (rc.bottom - rc.top).max(1) as u32,
        ))
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::EmbedTarget;
    use std::os::raw::{c_int, c_ulong};
    use winit::raw_window_handle::{RawWindowHandle, XlibWindowHandle};
    use x11_dl::xlib::{Display, XErrorEvent, XWindowAttributes, Xlib};

    /// X11 hatalarını yok sayar. Varsayılan davranış, geçersiz pencere
    /// kimliğinde bütün programı kapatmaktır; burada yalnız `None` dönmeliyiz.
    unsafe extern "C" fn ignore_error(_: *mut Display, _: *mut XErrorEvent) -> c_int {
        0
    }

    /// xscreensaver'ın verdiği X11 penceresini doğrular ve boyutunu okur.
    pub fn embed_target(id: u64) -> Option<EmbedTarget> {
        let window = c_ulong::try_from(id).ok()?;
        let xlib = Xlib::open().ok()?;
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return None;
            }
            let previous = (xlib.XSetErrorHandler)(Some(ignore_error));
            let mut attributes: XWindowAttributes = std::mem::zeroed();
            let found = (xlib.XGetWindowAttributes)(display, window, &mut attributes) != 0;
            (xlib.XSetErrorHandler)(previous);
            (xlib.XCloseDisplay)(display);
            if !found || attributes.width <= 0 || attributes.height <= 0 {
                return None;
            }
            Some((
                RawWindowHandle::Xlib(XlibWindowHandle::new(window)),
                attributes.width as u32,
                attributes.height as u32,
            ))
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::EmbedTarget;

    /// Bu sistemde başka pencereye yerleşme yoktur; macOS kendi `.saver` paketini kullanır.
    pub fn embed_target(_id: u64) -> Option<EmbedTarget> {
        None
    }
}

/// Mesajı konsola yazar. Windows dışındaki sistemlerde program konsoldan çalışır.
#[cfg(not(windows))]
pub fn show_message(title: &str, text: &str) {
    log(&format!("{title}: {text}"));
    println!("{title}\n\n{text}");
}

/// Yalnız hata ayıklama derlemesinde (Linux): xscreensaver programı SIGTERM
/// sinyaliyle kapatır. Sinyalin olağan davranışı programı hemen bitirmektir;
/// o zaman son sayaçlar kayda yazılamaz. Bu yüzden sinyal ayrı bir iş
/// parçacığında beklenir ve olay döngüsüne iletilir.
#[cfg(all(target_os = "linux", debug_assertions))]
pub fn forward_termination(proxy: EventLoopProxy<()>) {
    // Sinyal bu iş parçacığında ve ondan sonra açılanlarda bekletilir;
    // yalnız aşağıdaki bekleyen iş parçacığı onu alır.
    let mut signals: libc::sigset_t = unsafe { std::mem::zeroed() };
    let blocked = unsafe {
        libc::sigemptyset(&mut signals);
        libc::sigaddset(&mut signals, libc::SIGTERM);
        libc::pthread_sigmask(libc::SIG_BLOCK, &signals, std::ptr::null_mut()) == 0
    };
    if !blocked {
        return;
    }
    std::thread::spawn(move || {
        let mut signal = 0;
        if unsafe { libc::sigwait(&signals, &mut signal) } == 0 {
            let _ = proxy.send_event(());
        }
    });
}

/// Sürüm derlemesinde ve diğer sistemlerde sinyal beklenmez; program
/// sinyal gelince işletim sistemince hemen kapatılır.
#[cfg(not(all(target_os = "linux", debug_assertions)))]
pub fn forward_termination(_: EventLoopProxy<()>) {}

pub use imp::embed_target;
#[cfg(windows)]
pub use imp::show_message;
