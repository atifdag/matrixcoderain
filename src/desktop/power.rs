//! Ekran gücü bildirimleri çizim zamanlayıcısını uyandırır veya durdurur.
use winit::{event_loop::EventLoopBuilder, window::Window};
#[cfg(windows)]
mod imp {
    use super::*;
    use std::{
        ffi::c_void,
        sync::atomic::{AtomicBool, Ordering},
    };
    use windows_sys::{
        core::GUID,
        Win32::{
            System::Power::{
                RegisterPowerSettingNotification, UnregisterPowerSettingNotification,
                POWERBROADCAST_SETTING,
            },
            UI::WindowsAndMessaging::{
                DEVICE_NOTIFY_WINDOW_HANDLE, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND,
                PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE, WM_DISPLAYCHANGE, WM_POWERBROADCAST,
            },
        },
    };
    use winit::{
        platform::windows::EventLoopBuilderExtWindows,
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };
    /// Windows'un "oturum ekranı açık mı" bildirimi (GUID_SESSION_DISPLAY_STATUS).
    /// Windows bu bilgiyi tek tek ekranlar için değil, oturumun bütünü için verir.
    const DISPLAY: GUID = GUID::from_u128(0x2b84c20e_ad23_4ddf_93db_05ffbd7efca5);
    /// Ekran açık mı? Bildirim gelene kadar açık kabul edilir.
    static DISPLAY_ON: AtomicBool = AtomicBool::new(true);
    /// Bilgisayar uyku dışında mı?
    static SYSTEM_AWAKE: AtomicBool = AtomicBool::new(true);
    /// Ekran takıldı, çıkarıldı veya çözünürlüğü değişti mi?
    static DISPLAY_CHANGED: AtomicBool = AtomicBool::new(false);
    /// Bildirim kaydı, pencere bırakılırken otomatik olarak kaldırılır.
    pub struct Subscription(isize);
    impl Drop for Subscription {
        fn drop(&mut self) {
            unsafe {
                UnregisterPowerSettingNotification(self.0);
            }
        }
    }
    /// Pencerenin ekran gücü bildirimlerini alabilmesini sağlar.
    pub fn subscribe(window: &Window) -> Result<Subscription, String> {
        let handle = window.window_handle().map_err(|e| e.to_string())?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err("Windows pencere kimliği alınamadı.".into());
        };
        let notification = unsafe {
            RegisterPowerSettingNotification(
                handle.hwnd.get() as *mut c_void,
                &DISPLAY,
                DEVICE_NOTIFY_WINDOW_HANDLE,
            )
        };
        if notification == 0 {
            return Err(format!(
                "Ekran gücü bildirimi kaydedilemedi: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Subscription(notification))
    }
    /// Sistem iletilerini gözler; iletinin normal işlenmesini engellemez.
    pub fn configure(builder: &mut EventLoopBuilder<()>) {
        builder.with_msg_hook(|raw| {
            // Adres, winit tarafından yalnız bu çağrı süresince geçerli verilir.
            let msg = unsafe { &*(raw as *const MSG) };
            if msg.message == WM_DISPLAYCHANGE {
                DISPLAY_CHANGED.store(true, Ordering::Relaxed);
            }
            if msg.message == WM_POWERBROADCAST {
                match msg.wParam as u32 {
                    PBT_APMSUSPEND => SYSTEM_AWAKE.store(false, Ordering::Relaxed),
                    PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => {
                        SYSTEM_AWAKE.store(true, Ordering::Relaxed)
                    }
                    PBT_POWERSETTINGCHANGE if msg.lParam != 0 => {
                        let setting = unsafe { &*(msg.lParam as *const POWERBROADCAST_SETTING) };
                        let guid = setting.PowerSetting;
                        let matches = guid.data1 == DISPLAY.data1
                            && guid.data2 == DISPLAY.data2
                            && guid.data3 == DISPLAY.data3
                            && guid.data4 == DISPLAY.data4;
                        if matches && setting.DataLength == 4 {
                            let state = unsafe {
                                std::ptr::read_unaligned(setting.Data.as_ptr() as *const u32)
                            };
                            DISPLAY_ON.store(state != 0, Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
            false
        });
    }
    /// Ekran açık ve sistem uyanıkken çizime izin verir.
    pub fn display_awake() -> bool {
        DISPLAY_ON.load(Ordering::Relaxed) && SYSTEM_AWAKE.load(Ordering::Relaxed)
    }
    /// Ekran yerleşimi değişikliğini bir kez tüketir.
    pub fn take_display_change() -> bool {
        DISPLAY_CHANGED.swap(false, Ordering::Relaxed)
    }
}
/// Windows dışındaki masaüstü sistemleri. Linux'ta ekran kapanınca programı
/// xscreensaver durdurur; macOS'ta ekran koruyucu `.saver` paketiyle çalışır.
/// Bu nedenle burada yalnız deneme penceresi için ekran açık kabul edilir.
#[cfg(not(windows))]
mod imp {
    use super::*;
    /// Bildirim kaydı tutulmadığından boş bir işarettir.
    pub struct Subscription;
    /// Uyumluluk için vardır; kayıt yapmaz.
    pub fn subscribe(_: &Window) -> Result<Subscription, String> {
        Ok(Subscription)
    }
    /// Uyumluluk için vardır; olay döngüsüne ek yapmaz.
    pub fn configure(_: &mut EventLoopBuilder<()>) {}
    /// Ekranı her zaman açık kabul eder.
    pub fn display_awake() -> bool {
        true
    }
    /// Ekran yerleşimi değişikliğini bir kez tüketir.
    pub fn take_display_change() -> bool {
        false
    }
}
pub use imp::*;
