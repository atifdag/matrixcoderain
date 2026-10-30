//! Pencereleri ve kullanıcı olaylarını yönetir; çizim kuralları ortak kitaplıktadır.
use crate::{font_source, platform, power, settings::*};
use chrono::{Local, Timelike};
use matrix_core::{Engine, Fonts, TimeOfDay};
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU32,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    monitor::MonitorHandle,
    window::{Fullscreen, Window, WindowAttributes, WindowId, WindowLevel},
};

/// Programın çalışma biçimi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Her ekranı kaplayan ekran koruyucu; kullanıcı etkileşiminde kapanır.
    Saver,
    /// Boyutu değiştirilebilen deneme penceresi; Esc ile kapanır.
    Window,
    /// Başka programın verdiği pencerenin içine çizim (Windows önizlemesi,
    /// xscreensaver). Kapanma kararını o program verir.
    Embedded(u64),
}

/// İki çizim arasındaki süre.
const FRAME: Duration = Duration::from_millis(FRAME_MS);
/// Açılıştan sonra bu süre içindeki tuş ve tıklamalar yok sayılır, fare hareketi
/// yalnız başlangıç konumunu günceller; sistemin ekran koruyucuyu başlatırken
/// ürettiği olaylar onu hemen kapatmasın.
const INPUT_GRACE: Duration = Duration::from_millis(500);
/// Açılıştaki odak değişiklikleri bu süre boyunca kapanma sebebi sayılmaz.
const FOCUS_GRACE: Duration = Duration::from_millis(1500);
/// Çalışırken sayaçların sorun giderme kaydına yazılma aralığı.
const REPORT_INTERVAL: Duration = Duration::from_secs(60);

/// Ekran koruyucu penceresinin özellikleri (Windows).
///
/// Windows'ta `winit`'in tam ekran kipi kullanılmaz: winit, pencereyi öne getirmek
/// için sahte bir Alt tuşu basıp bırakır. Türkçe gibi AltGr'li klavye
/// düzenlerinde bu sağ Alt olarak gelir ve ekran koruyucu daha açılır açılmaz
/// "tuşa basıldı" diye kapanır. Bunun yerine ekranı tam örten, çerçevesiz ve
/// her zaman üstte duran bir pencere açılır.
#[cfg(windows)]
fn saver_attributes(base: WindowAttributes, monitor: Option<MonitorHandle>) -> WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;
    let attrs = base
        .with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_resizable(false)
        .with_undecorated_shadow(false);
    match monitor {
        Some(m) => attrs.with_position(m.position()).with_inner_size(m.size()),
        None => attrs.with_fullscreen(Some(Fullscreen::Borderless(None))),
    }
}

/// Ekran koruyucu penceresinin özellikleri (macOS ve Linux): işletim sisteminin
/// kendi çerçevesiz tam ekran kipi kullanılır.
#[cfg(not(windows))]
fn saver_attributes(base: WindowAttributes, monitor: Option<MonitorHandle>) -> WindowAttributes {
    base.with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_fullscreen(Some(Fullscreen::Borderless(monitor)))
}

/// Pencereye görüntü yazmak için kullanılan yüzey.
type Surface = softbuffer::Surface<Rc<Window>, Rc<Window>>;

/// Tek bir pencere ve ona ait çizim motoru.
struct View {
    window: Rc<Window>,
    surface: Surface,
    engine: Engine,
    /// Ekran koruyucuda farenin ilk görüldüğü yer; hareket buna göre ölçülür.
    mouse_origin: Option<PhysicalPosition<f64>>,
    /// Pencere başka pencerelerin altında kaldıysa veya küçültüldüyse doğru.
    occluded: bool,
    /// Çizim şu anda çalışıyor mu?
    active: bool,
    /// Yağmura en son süre verilen an.
    last_tick: Instant,
    /// Sayaçların sorun giderme kaydına en son yazıldığı an.
    last_report: Instant,
    /// Ekran gücü bildirim kaydı; pencereyle birlikte kaldırılır.
    _power: power::Subscription,
}

/// Bilgisayarın yerel saatini ekrandaki saat biçimine çevirir.
fn local_time() -> TimeOfDay {
    let now = Local::now();
    TimeOfDay::new(now.hour() as u8, now.minute() as u8, now.second() as u8).unwrap_or_default()
}

impl View {
    /// Pencereye görüntü yüzeyi ve durdurulmuş bir motor bağlar.
    fn new(window: Window, fonts: Arc<Fonts>, seed: u64) -> Result<Self, String> {
        let window = Rc::new(window);
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| format!("Çizim başlatılamadı: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| format!("Görüntü yüzeyi açılamadı: {e}"))?;
        let size = window.inner_size();
        let engine = Engine::new(size.width.max(1), size.height.max(1), fonts, seed)
            .map_err(|e| e.to_string())?;
        let subscription = power::subscribe(&window)?;
        Ok(Self {
            window,
            surface,
            engine,
            mouse_origin: None,
            occluded: false,
            active: false,
            last_tick: Instant::now(),
            last_report: Instant::now(),
            _power: subscription,
        })
    }

    /// Çizimi başlatır veya durdurur. Yeniden başlarken süre ölçümü sıfırlanır.
    fn set_active(&mut self, active: bool) {
        if self.active != active {
            self.active = active;
            self.engine.set_active(active);
            self.last_tick = Instant::now();
            self.report(if active {
                "Çizim başladı"
            } else {
                "Çizim durdu"
            });
            if active {
                self.window.request_redraw();
            }
        }
    }

    /// Geçen süreyi motora verir ve yeni çizim ister. Dakikada bir sayaçları kaydeder.
    fn tick(&mut self, now: Instant) {
        if !self.active {
            return;
        }
        self.engine
            .update(now.duration_since(self.last_tick), local_time());
        self.last_tick = now;
        if now.duration_since(self.last_report) >= REPORT_INTERVAL {
            self.report("Çalışıyor");
        }
        self.window.request_redraw();
    }

    /// Sayaçları ve çizim sürelerini sorun giderme kaydına yazar. Durunca ve
    /// yeniden başlayınca kare sayısının aynı kalması, aradaki sürede çizim
    /// yapılmadığını gösterir.
    fn report(&mut self, reason: &str) {
        platform::log(&format!(
            "{reason} ({:?}): {}",
            self.window.id(),
            self.engine.stats()
        ));
        self.last_report = Instant::now();
    }

    /// Hazır görüntüyü pencereye yazar. Pencere boyutu değiştiyse önce motoru uyarlar.
    fn draw(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        let size = self.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Ok(());
        };
        self.engine
            .resize(size.width, size.height)
            .map_err(|e| e.to_string())?;
        self.surface
            .resize(w, h)
            .map_err(|e| format!("Görüntü boyutu değiştirilemedi: {e}"))?;
        // Süre vermeden saati günceller; boyut değişince saat hemen görünsün.
        self.engine.update(Duration::ZERO, local_time());
        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|e| format!("Görüntü alanı alınamadı: {e}"))?;
        self.engine.render(&mut buffer).map_err(|e| e.to_string())?;
        self.window.pre_present_notify();
        buffer
            .present()
            .map_err(|e| format!("Görüntü sunulamadı: {e}"))
    }
}

/// Programın bütün pencereleri ve ortak durumu.
struct App {
    mode: Mode,
    fonts: Arc<Fonts>,
    views: HashMap<WindowId, View>,
    /// Bir sonraki çizimin zamanı.
    next_frame: Instant,
    /// Programın başladığı an; açılış süreleri buna göre ölçülür.
    started: Instant,
    /// Bu andan önceki tuş ve tıklamalar yok sayılır.
    input_ready: Instant,
    /// Odağı olan pencereler; hepsi odağı kaybedince ekran koruyucu kapanır.
    focused: HashSet<WindowId>,
    /// İşletim sistemi programı askıya aldıysa doğru.
    suspended: bool,
    /// Programı durduran hata; olay döngüsü bitince çağırana iletilir.
    error: Option<String>,
}

/// Pencere olayları bitene kadar çalışır; hata varsa çağırana iletir.
pub fn run(mode: Mode, font: Option<PathBuf>) -> Result<(), String> {
    let fonts = font_source::load(font.as_deref())?;
    let mut builder = EventLoop::builder();
    power::configure(&mut builder);
    // xscreensaver penceresi bir X11 penceresidir; Wayland oturumunda da X11 ile bağlanılır.
    #[cfg(target_os = "linux")]
    if let Mode::Embedded(_) = mode {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11();
    }
    let event_loop = builder
        .build()
        .map_err(|e| format!("Olay döngüsü açılamadı: {e}"))?;
    platform::forward_termination(event_loop.create_proxy());
    let now = Instant::now();
    let mut app = App {
        mode,
        fonts,
        views: HashMap::new(),
        next_frame: now + FRAME,
        started: now,
        input_ready: now + INPUT_GRACE,
        focused: HashSet::new(),
        suspended: false,
        error: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("Olay döngüsü hatası: {e}"))?;
    app.error.map_or(Ok(()), Err)
}

impl App {
    /// Pencereyi açar ve ona her açılışta farklı yağmur üreten bir motor bağlar.
    fn add_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        attrs: WindowAttributes,
    ) -> Result<(), String> {
        let window = event_loop
            .create_window(attrs)
            .map_err(|e| format!("Pencere açılamadı: {e}"))?;
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let view = View::new(window, self.fonts.clone(), seed)?;
        self.views.insert(view.window.id(), view);
        Ok(())
    }

    /// Çalışma biçimine göre pencereleri açar: ekran koruyucuda her ekrana bir pencere.
    fn create_windows(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let base = || Window::default_attributes().with_title(APP_NAME);
        match self.mode {
            Mode::Window => self.add_window(
                event_loop,
                base().with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
            )?,
            Mode::Saver => {
                let monitors: Vec<_> = event_loop.available_monitors().collect();
                if monitors.is_empty() {
                    self.add_window(event_loop, saver_attributes(base(), None))?;
                }
                for monitor in monitors {
                    self.add_window(event_loop, saver_attributes(base(), Some(monitor)))?;
                }
                for view in self.views.values() {
                    view.window.set_cursor_visible(false);
                }
            }
            Mode::Embedded(id) => {
                let (parent, w, h) =
                    platform::embed_target(id).ok_or("Verilen pencere bulunamadı.")?;
                // Pencerenin geçerliliği yukarıda işletim sistemiyle denetlendi.
                let attrs = unsafe { base().with_parent_window(Some(parent)) }
                    .with_decorations(false)
                    .with_position(PhysicalPosition::new(0, 0))
                    .with_inner_size(PhysicalSize::new(w, h));
                self.add_window(event_loop, attrs)?;
            }
        }
        Ok(())
    }

    /// Hatayı kaydeder ve programı kapatır.
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        platform::log(&message);
        self.error = Some(message);
        event_loop.exit();
    }

    /// Ekran koruyucuyu kapatır ve sebebini sorun giderme kaydına yazar.
    /// Hata ayıklama derlemesinde önce çizimler durdurulur; son sayaçlar da kayda geçer.
    fn quit(&mut self, event_loop: &ActiveEventLoop, reason: &str) {
        #[cfg(debug_assertions)]
        for view in self.views.values_mut() {
            view.set_active(false);
        }
        platform::log(&format!(
            "Çıkış: {reason}, {} ms sonra",
            self.started.elapsed().as_millis()
        ));
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    /// Program ilk kez veya askıdan sonra çalışmaya başladığında pencereleri hazırlar.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = false;
        if self.views.is_empty() {
            if let Err(e) = self.create_windows(event_loop) {
                self.fail(event_loop, e);
            }
        }
        self.next_frame = Instant::now() + FRAME;
    }

    /// Program dışarıdan kapatılmak istendi (hata ayıklama derlemesinde
    /// Linux'ta xscreensaver'ın SIGTERM sinyali).
    fn user_event(&mut self, event_loop: &ActiveEventLoop, _: ()) {
        self.quit(event_loop, "sonlandırma sinyali");
    }

    /// Sistem programı askıya aldığında bütün çizimleri durdurur.
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        for view in self.views.values_mut() {
            view.set_active(false);
        }
    }

    /// Çizim zamanı geldiğinde her pencerenin yağmurunu ilerletir.
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            let now = Instant::now();
            for view in self.views.values_mut() {
                view.tick(now);
            }
            self.next_frame = now + FRAME;
        }
    }

    /// Olaylar işlendikten sonra hangi pencerelerin çizileceğine karar verir.
    /// Hiçbiri çizilmeyecekse zamanlayıcı kurulmaz; program yalnız olay bekler.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Kapanış başladıysa çizim yeniden başlatılmaz.
        if event_loop.exiting() {
            return;
        }
        if power::take_display_change() && self.mode == Mode::Saver {
            // Ekran takıldı veya çıkarıldı: pencereler yeni yerleşime göre açılır.
            self.views.clear();
            self.focused.clear();
            self.input_ready = Instant::now() + INPUT_GRACE;
            if let Err(e) = self.create_windows(event_loop) {
                self.fail(event_loop, e);
                return;
            }
        }
        let mut running = false;
        for view in self.views.values_mut() {
            let size = view.window.inner_size();
            let enabled = !self.suspended
                && power::display_awake()
                && !view.occluded
                && size.width > 0
                && size.height > 0;
            view.set_active(enabled);
            running |= enabled;
        }
        if running {
            if self.next_frame <= Instant::now() {
                self.next_frame = Instant::now() + FRAME;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    /// Pencere ve kullanıcı olaylarını işler.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self.views.contains_key(&id) {
            return;
        }
        let saver = self.mode == Mode::Saver;
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Occluded(hidden) => {
                if let Some(v) = self.views.get_mut(&id) {
                    v.occluded = hidden;
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(v) = self.views.get(&id) {
                    v.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(v) = self.views.get_mut(&id) {
                    if let Err(e) = v.draw() {
                        self.fail(event_loop, e);
                    }
                }
            }

            // Fare hareketi: ilk konum başlangıç kabul edilir, eşikten fazla uzaklaşınca çıkılır.
            // X11'de pencere önce normal boyutta açılıp sonra tam ekrana geçer; aynı fare
            // konumu pencereye göre kayar. Bu yüzden açılış sırasında gelen konumlar yalnız
            // başlangıcı günceller.
            WindowEvent::CursorMoved { position, .. } if saver => {
                let opening = Instant::now() < self.input_ready;
                let Some(view) = self.views.get_mut(&id) else {
                    return;
                };
                let origin = match view.mouse_origin {
                    Some(origin) if !opening => origin,
                    _ => {
                        view.mouse_origin = Some(position);
                        return;
                    }
                };
                let (dx, dy) = (position.x - origin.x, position.y - origin.y);
                if dx.abs() > MOUSE_THRESHOLD || dy.abs() > MOUSE_THRESHOLD {
                    self.quit(event_loop, &format!("fare hareketi ({dx:.0}, {dy:.0})"));
                }
            }

            // Tıklama, tekerlek ya da dokunma: ekran koruyucuyu kapatır.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::Touch(_)
                if saver =>
            {
                if Instant::now() < self.input_ready {
                    platform::log("Yok sayıldı: açılış sırasında tıklama.");
                    return;
                }
                self.quit(event_loop, "tıklama, tekerlek veya dokunma");
            }

            // Tuş: ekran koruyucuda her tuş kapatır; deneme penceresinde yalnız Esc.
            // `is_synthetic`: pencere odak alırken o anda basılı tuşlar için üretilen
            // yapay olaylardır; gerçek tuş vuruşu değildir, dikkate alınmaz.
            WindowEvent::KeyboardInput {
                event: key,
                is_synthetic,
                ..
            } => {
                if is_synthetic || key.state != ElementState::Pressed {
                    return;
                }
                if Instant::now() < self.input_ready {
                    platform::log("Yok sayıldı: açılış sırasında tuş olayı.");
                    return;
                }
                let escape = key.logical_key == Key::Named(NamedKey::Escape);
                if saver || (self.mode == Mode::Window && escape) {
                    self.quit(event_loop, "kullanıcı tuşa bastı");
                }
            }

            // Odak kaybı (başka uygulama öne geçti, kilit ekranı vb.): ekran koruyucuyu kapatır.
            WindowEvent::Focused(has_focus) if saver => {
                platform::log(&format!(
                    "Odak: {}, {} ms sonra",
                    if has_focus { "alındı" } else { "kaybedildi" },
                    self.started.elapsed().as_millis()
                ));
                if has_focus {
                    self.focused.insert(id);
                } else {
                    self.focused.remove(&id);
                    if self.focused.is_empty() && self.started.elapsed() > FOCUS_GRACE {
                        self.quit(event_loop, "odak kaybı");
                    }
                }
            }

            _ => {}
        }
    }
}
