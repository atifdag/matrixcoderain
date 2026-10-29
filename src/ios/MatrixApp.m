// Matrix - Kod Yağmuru: iPhone uygulaması.
//
// iOS üçüncü taraf ekran koruyucusuna izin vermez. Bu uygulama açıkken yağmuru
// ve saati tam ekran gösterir ve ekranın kendiliğinden kararmasını engeller.
// Arka plana geçince veya telefon kilitlenince çizim durur, kaynaklar bırakılır.
// Yağmur ve saat ortak Rust kitaplığında çizilir; bu dosya yalnız uygulamanın
// açılıp kapanmasını kitaplığa iletir ve hazır görüntüyü ekrana koyar.

#import <UIKit/UIKit.h>
#import <QuartzCore/QuartzCore.h>
#import <IOSurface/IOSurfaceRef.h>
#import <CoreVideo/CoreVideo.h>
#include <os/log.h>
#include <time.h>
#include "matrix.h"

// Saniyede 25 kare; diğer platformlarla aynı görünüm.
static const float kFramesPerSecond = 25;

// Çalışırken sayaçların kayda yazılma aralığı, saniye.
static const CFTimeInterval kReportInterval = 60.0;

// Yağmuru çizen tam ekran görünüm.
@interface MatrixRainView : UIView
- (void)start;
- (void)pause;
- (void)stop;
@end

@implementation MatrixRainView {
    // Rust motorunun kimliği; 0 ise motor yok.
    uint64_t _engine;
    // Rust'ın doğrudan yazdığı iki görüntü yüzeyi. Biri ekrandayken diğerine
    // yazılır; böylece yarım yazılmış kare görünmez. Bellekteki sıra:
    // kırmızı, yeşil, mavi, 255. Ekran kartı bu sırayı dönüştürmeden gösterir.
    IOSurfaceRef _surfaces[2];
    // Sıradaki karenin yazılacağı yüzey: 0 veya 1.
    int _next;
    // Motorun kurulduğu görünüm boyutu, piksel. Değişirse motor yeniden kurulur.
    CGSize _builtFor;
    // Ekran tazelemesine bağlı zamanlayıcı; nil ise yağmur bekliyor.
    CADisplayLink *_displayLink;
    // Yağmura en son süre verilen an, saniye.
    CFTimeInterval _lastTick;
    // Sayaçların en son kayda yazıldığı an, saniye.
    CFTimeInterval _lastReport;
}

- (instancetype)initWithFrame:(CGRect)frame {
    self = [super initWithFrame:frame];
    if (self) {
        self.backgroundColor = UIColor.blackColor;
        // Yüzey görünümün tamamına yumuşatılmadan yerleştirilir.
        self.layer.contentsGravity = kCAGravityResize;
        self.layer.magnificationFilter = kCAFilterNearest;
    }
    return self;
}

- (void)dealloc {
    [self releaseEngine];
}

// Zamanlayıcıyı başlatır; motor yoksa ilk karede kurulur.
- (void)start {
    if (_displayLink != nil) {
        return;
    }
    _lastTick = CACurrentMediaTime();
    if (_engine != 0) {
        matrix_set_active(_engine, 1);
    }
    _displayLink = [CADisplayLink displayLinkWithTarget:self selector:@selector(tick:)];
    _displayLink.preferredFrameRateRange =
        CAFrameRateRangeMake(kFramesPerSecond, kFramesPerSecond, kFramesPerSecond);
    [_displayLink addToRunLoop:NSRunLoop.mainRunLoop forMode:NSRunLoopCommonModes];
}

// Zamanlayıcıyı durdurur; yağmur bekler, son kare ekranda kalır.
- (void)pause {
    [_displayLink invalidate];
    _displayLink = nil;
    if (_engine != 0) {
        matrix_set_active(_engine, 0);
    }
}

// Zamanlayıcıyı durdurur, motoru ve görüntü yüzeylerini bırakır.
- (void)stop {
    [self pause];
    [self report:@"Çizim durdu"];
    [self releaseEngine];
}

// Sayaçları ve çizim sürelerini sistem kaydına yazar.
- (void)report:(NSString *)reason {
    char text[256];
    if (_engine != 0 && matrix_stats_text(_engine, (uint8_t *)text, sizeof text) == 0) {
        // Sistem, uygulamaların kayıt metnini gizli sayar; okunabilmesi için
        // metin açık olarak işaretlenir.
        os_log(OS_LOG_DEFAULT, "Matrix - Kod Yağmuru: %{public}@: %{public}s", reason, text);
    }
    _lastReport = CACurrentMediaTime();
}

// Zamanlayıcı her karede çağırır.
- (void)tick:(CADisplayLink *)displayLink {
    if (![self prepareEngine]) {
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
    self.layer.contents = (__bridge id)surface;
    _next = 1 - _next;
    if (now - _lastReport >= kReportInterval) {
        [self report:@"Çalışıyor"];
    }
}

// Görünüm boyutuna uygun motoru ve görüntü yüzeylerini hazırlar; boyut
// değiştiyse (ör. telefon döndürüldüyse) yeniden kurar. Hazırlanamazsa NO döner.
- (BOOL)prepareEngine {
    // contentScaleFactor kendi çizimini yapmayan görünümde 1'dir; ekranın
    // gerçek ölçeği buradan alınır.
    CGFloat scale = self.traitCollection.displayScale;
    CGSize pixels = CGSizeMake(round(self.bounds.size.width * scale),
                               round(self.bounds.size.height * scale));
    if (pixels.width < 1 || pixels.height < 1) {
        return NO;
    }
    if (_engine != 0 && CGSizeEqualToSize(pixels, _builtFor)) {
        return YES;
    }
    [self releaseEngine];
    if (![self createEngineWidth:(uint32_t)pixels.width height:(uint32_t)pixels.height]) {
        return NO;
    }
    _builtFor = pixels;
    return YES;
}

// Verilen boyutta çalışan bir motor ve ona uygun görüntü yüzeylerini oluşturur.
- (BOOL)createEngineWidth:(uint32_t)width height:(uint32_t)height {
    uint64_t engine = 0;
    // Her açılışta farklı yağmur görünsün.
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
    self.layer.contents = nil;
    for (int i = 0; i < 2; i++) {
        if (_surfaces[i] != NULL) {
            CFRelease(_surfaces[i]);
            _surfaces[i] = NULL;
        }
    }
    _builtFor = CGSizeZero;
}

@end

// Durum çubuğunu ve alttaki çizgiyi gizleyen tam ekran sayfa.
@interface MatrixViewController : UIViewController
@end

@implementation MatrixViewController

- (void)loadView {
    self.view = [[MatrixRainView alloc] initWithFrame:CGRectZero];
}

- (BOOL)prefersStatusBarHidden {
    return YES;
}

- (BOOL)prefersHomeIndicatorAutoHidden {
    return YES;
}

@end

// Uygulama penceresini kurar ve yağmuru uygulamanın durumuna göre yönetir.
@interface MatrixSceneDelegate : UIResponder <UIWindowSceneDelegate>
@property (nonatomic, strong) UIWindow *window;
@end

@implementation MatrixSceneDelegate {
    MatrixRainView *_rain;
}

- (void)scene:(UIScene *)scene
    willConnectToSession:(UISceneSession *)session
                 options:(UISceneConnectionOptions *)options {
    MatrixViewController *controller = [MatrixViewController new];
    _rain = (MatrixRainView *)controller.view;
    self.window = [[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController = controller;
    [self.window makeKeyAndVisible];
}

// Uygulama önde ve kullanımda: yağmur akar, ekran kendiliğinden kararmaz.
- (void)sceneDidBecomeActive:(UIScene *)scene {
    UIApplication.sharedApplication.idleTimerDisabled = YES;
    [_rain start];
}

// Kontrol Merkezi açıldığında veya arama geldiğinde yağmur bekler.
- (void)sceneWillResignActive:(UIScene *)scene {
    [_rain pause];
    UIApplication.sharedApplication.idleTimerDisabled = NO;
}

// Arka plana geçildi veya telefon kilitlendi: motor ve görüntü belleği bırakılır.
- (void)sceneDidEnterBackground:(UIScene *)scene {
    [_rain stop];
}

@end

// Pencereyi MatrixSceneDelegate kurar; uygulamanın kendisine ayrıca iş düşmez.
@interface MatrixAppDelegate : UIResponder <UIApplicationDelegate>
@end

@implementation MatrixAppDelegate
@end

int main(int argc, char *argv[]) {
    @autoreleasepool {
        return UIApplicationMain(argc, argv, nil, NSStringFromClass(MatrixAppDelegate.class));
    }
}
