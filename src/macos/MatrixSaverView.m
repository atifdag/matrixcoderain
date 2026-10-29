// Matrix - Kod Yağmuru: macOS ekran koruyucusu (.saver).
//
// Yağmur ve saat ortak Rust kitaplığında çizilir. Bu dosya yalnız sistemin
// ekran koruyucu çağrılarını kitaplığa iletir ve hazır görüntüyü ekrana koyar.
// Parola ve kilit ekranı macOS'un işidir; burada yapılmaz.

#import <ScreenSaver/ScreenSaver.h>
#import <QuartzCore/QuartzCore.h>
#import <IOSurface/IOSurface.h>
#include <os/log.h>
#include <time.h>
#include "matrix.h"

// Saniyede 25 kare; masaüstü programıyla aynı görünüm.
static const NSTimeInterval kFrameInterval = 1.0 / 25.0;

// Ekran koruyucu bittiğinde sistemin bütün programlara duyurduğu bildirim.
static NSString *const kSaverWillStop = @"com.apple.screensaver.willstop";

// Çalışırken sayaçların Konsol uygulamasına yazılma aralığı, saniye.
static const CFTimeInterval kReportInterval = 60.0;

@interface MatrixKodYagmuruView : ScreenSaverView
@end

@implementation MatrixKodYagmuruView {
    // Rust motorunun kimliği; 0 ise motor yok.
    uint64_t _engine;
    // Rust'ın doğrudan yazdığı iki görüntü yüzeyi. Biri ekrandayken diğerine
    // yazılır; böylece yarım yazılmış kare görünmez. Bellekteki sıra:
    // kırmızı, yeşil, mavi, 255. Ekran kartı bu sırayı dönüştürmeden gösterir.
    IOSurfaceRef _surfaces[2];
    // Sıradaki karenin yazılacağı yüzey: 0 veya 1.
    int _next;
    // Ekranda gösterilecek son yüzey; sahibi _surfaces dizisidir.
    IOSurfaceRef _shown;
    // Motorun kurulduğu görünüm boyutu, piksel. Değişirse motor yeniden kurulur.
    NSSize _builtFor;
    // Yağmura en son süre verilen an, saniye.
    CFTimeInterval _lastTick;
    // Sayaçların en son kayda yazıldığı an, saniye.
    CFTimeInterval _lastReport;
    // Sistem ekran koruyucuyu başlattı ve henüz durdurmadı.
    BOOL _running;
    // Ekranlar uyku durumunda.
    BOOL _screensAsleep;
}

- (instancetype)initWithFrame:(NSRect)frame isPreview:(BOOL)isPreview {
    self = [super initWithFrame:frame isPreview:isPreview];
    if (self) {
        [self setAnimationTimeInterval:kFrameInterval];
        self.wantsLayer = YES;
        NSNotificationCenter *workspace = NSWorkspace.sharedWorkspace.notificationCenter;
        [workspace addObserver:self
                      selector:@selector(screensDidSleep:)
                          name:NSWorkspaceScreensDidSleepNotification
                        object:nil];
        [workspace addObserver:self
                      selector:@selector(screensDidWake:)
                          name:NSWorkspaceScreensDidWakeNotification
                        object:nil];
        // Yeni macOS sürümlerinde sistem stopAnimation çağrısını her zaman
        // yapmayabiliyor; ekran koruyucunun bittiği bu bildirimden de anlaşılır.
        [NSDistributedNotificationCenter.defaultCenter addObserver:self
                                                          selector:@selector(saverWillStop:)
                                                              name:kSaverWillStop
                                                            object:nil];
    }
    return self;
}

- (void)dealloc {
    [NSWorkspace.sharedWorkspace.notificationCenter removeObserver:self];
    [NSDistributedNotificationCenter.defaultCenter removeObserver:self];
    // Sistem görünümü stopAnimation çağırmadan da kaldırabiliyor (ör. ekran
    // uyuyunca); kayıtta her başlangıcın bir bitişi olsun.
    [self report:@"Çizim durdu"];
    [self releaseEngine];
}

// Görüntü her zaman opaktır; arkasındaki içeriğin çizilmesine gerek yoktur.
- (BOOL)isOpaque {
    return YES;
}

// Görüntü işlemciyle yeniden çizilmez; hazır yüzey katmana verilir.
- (BOOL)wantsUpdateLayer {
    return YES;
}

// Son yüzeyi siyah zeminli katmanın tamamına yerleştirir. Yüzey ekran noktası
// boyutunda kurulduysa yumuşatılmadan büyütülür.
- (void)updateLayer {
    CALayer *layer = self.layer;
    layer.backgroundColor = CGColorGetConstantColor(kCGColorBlack);
    layer.contentsGravity = kCAGravityResize;
    layer.magnificationFilter = kCAFilterNearest;
    layer.contents = (__bridge id)_shown;
}

// Sistem ekran koruyucuyu başlattı.
- (void)startAnimation {
    _running = YES;
    [self updateTimer];
}

// Sistem ekran koruyucuyu durdurdu: zamanlayıcı durur, kaynaklar bırakılır.
// Bu çağrıdan sonra gelebilecek geç çizim istekleri _running ile engellenir.
- (void)stopAnimation {
    _running = NO;
    [super stopAnimation];
    [self report:@"Çizim durdu"];
    [self releaseEngine];
}

// Ekranlar uyudu: zamanlayıcı durdurulur, yağmur hesaplanmaz.
- (void)screensDidSleep:(NSNotification *)notification {
    _screensAsleep = YES;
    [self updateTimer];
    [self report:@"Ekran uyudu"];
}

// Ekranlar uyandı: ekran koruyucu hâlâ çalışıyorsa zamanlayıcı yeniden başlar.
- (void)screensDidWake:(NSNotification *)notification {
    _screensAsleep = NO;
    [self report:@"Ekran uyandı"];
    [self updateTimer];
}

// Pencereden çıkarılan görünüm ekranda görünmez, bu yüzden çizmez. Sistem
// Ayarları'ndaki önizleme yenilenirken eski görünüm durdurulmadan pencereden
// çıkarılabiliyor.
- (void)viewDidMoveToWindow {
    [super viewDidMoveToWindow];
    [self updateTimer];
}

// Ekran koruyucu çalışıyorsa, ekranlar uyanıksa ve görünüm bir penceredeyse
// zamanlayıcıyı başlatır; değilse durdurur ve yağmuru da bekletir.
- (void)updateTimer {
    BOOL wanted = _running && !_screensAsleep && self.window != nil;
    if (wanted == self.isAnimating) {
        return;
    }
    if (wanted) {
        _lastTick = CACurrentMediaTime();
        [super startAnimation];
    } else {
        [super stopAnimation];
    }
    if (_engine != 0) {
        matrix_set_active(_engine, wanted ? 1 : 0);
    }
}

// Sayaçları ve çizim sürelerini Konsol uygulamasında görünecek şekilde yazar.
// Ekran uyurken ve uyandığında kare sayısının aynı kalması, aradaki sürede
// çizim yapılmadığını gösterir.
- (void)report:(NSString *)reason {
    char text[256];
    if (_engine != 0 && matrix_stats_text(_engine, (uint8_t *)text, sizeof text) == 0) {
        // Sistem, ekran koruyucunun kayıt metnini gizli sayar; Konsol'da
        // okunabilmesi için metin açık olarak işaretlenir.
        os_log(OS_LOG_DEFAULT, "Matrix - Kod Yağmuru%{public}s: %{public}@: %{public}s",
               self.isPreview ? " (önizleme)" : "", reason, text);
    }
    _lastReport = CACurrentMediaTime();
}

// Gerçek ekran koruyucu bitti. Sistem Ayarları'ndaki küçük önizleme bundan etkilenmez.
- (void)saverWillStop:(NSNotification *)notification {
    if (!self.isPreview && _running) {
        [self stopAnimation];
    }
}

// Sistemin zamanlayıcısı her karede çağırır.
- (void)animateOneFrame {
    if (!_running || _screensAsleep || ![self prepareEngine]) {
        return;
    }
    CFTimeInterval now = CACurrentMediaTime();
    uint64_t elapsed = now > _lastTick ? (uint64_t)((now - _lastTick) * 1e6) : 0;
    _lastTick = now;

    time_t seconds = time(NULL);
    struct tm local;
    localtime_r(&seconds, &local);
    // Artık saniye (60) saatte 59 olarak gösterilir.
    uint8_t second = (uint8_t)MIN(local.tm_sec, 59);
    if (matrix_update(_engine, elapsed, (uint8_t)local.tm_hour, (uint8_t)local.tm_min, second) != 0) {
        return;
    }

    IOSurfaceRef surface = _surfaces[_next];
    IOSurfaceLock(surface, 0, NULL);
    int32_t rendered = matrix_render_rgba(_engine, IOSurfaceGetBaseAddress(surface),
                                          IOSurfaceGetAllocSize(surface),
                                          IOSurfaceGetBytesPerRow(surface));
    IOSurfaceUnlock(surface, 0, NULL);
    if (rendered != 0) {
        return;
    }
    _shown = surface;
    _next = 1 - _next;
    [self setNeedsDisplay:YES];
    if (now - _lastReport >= kReportInterval) {
        [self report:@"Çalışıyor"];
    }
}

// Görünüm boyutuna uygun motor ve görüntü alanını hazırlar; boyut
// değiştiyse yeniden kurar. Hazırlanamazsa NO döner.
- (BOOL)prepareEngine {
    NSSize pixels = [self convertRectToBacking:self.bounds].size;
    if (pixels.width < 1 || pixels.height < 1) {
        return NO;
    }
    if (_engine != 0 && NSEqualSizes(pixels, _builtFor)) {
        return YES;
    }
    [self releaseEngine];
    // Çok büyük ekranlarda (ör. 6K) kitaplığın piksel sınırı aşılırsa
    // ekran noktası boyutunda çizilir ve görüntü büyütülerek gösterilir.
    NSSize points = self.bounds.size;
    if (![self createEngineWidth:(uint32_t)pixels.width height:(uint32_t)pixels.height] &&
        ![self createEngineWidth:(uint32_t)points.width height:(uint32_t)points.height]) {
        return NO;
    }
    _builtFor = pixels;
    return YES;
}

// Verilen boyutta çalışan bir motor ve ona uygun görüntü yüzeylerini oluşturur.
- (BOOL)createEngineWidth:(uint32_t)width height:(uint32_t)height {
    if (width == 0 || height == 0) {
        return NO;
    }
    uint64_t engine = 0;
    // Her açılışta ve her ekranda farklı yağmur görünsün.
    uint64_t seed = (uint64_t)(CACurrentMediaTime() * 1e9) ^ (uint64_t)(uintptr_t)self;
    if (matrix_create(matrix_abi_version(), width, height, seed, NULL, 0, &engine) != 0) {
        return NO;
    }
    _engine = engine;
    NSDictionary *properties = @{
        (id)kIOSurfaceWidth: @(width),
        (id)kIOSurfaceHeight: @(height),
        (id)kIOSurfaceBytesPerElement: @4,
        (id)kIOSurfacePixelFormat: @(kCVPixelFormatType_32RGBA),
    };
    for (int i = 0; i < 2; i++) {
        _surfaces[i] = IOSurfaceCreate((__bridge CFDictionaryRef)properties);
        if (_surfaces[i] == NULL) {
            [self releaseEngine];
            return NO;
        }
    }
    _lastTick = CACurrentMediaTime();
    matrix_set_active(_engine, 1);
    [self report:[NSString stringWithFormat:@"Çizim başladı, %u × %u piksel", width, height]];
    return YES;
}

// Motoru ve görüntü yüzeylerini bırakır; görünüm siyah kalır. Tekrar
// çağrılması güvenlidir.
- (void)releaseEngine {
    if (_engine != 0) {
        matrix_destroy(_engine);
        _engine = 0;
    }
    _shown = NULL;
    self.layer.contents = nil;
    for (int i = 0; i < 2; i++) {
        if (_surfaces[i] != NULL) {
            CFRelease(_surfaces[i]);
            _surfaces[i] = NULL;
        }
    }
    _builtFor = NSZeroSize;
}

@end
