//! Aynı girdilerin aynı sonucu verdiğini ve duraklamada iş yapılmadığını sınar.
use crate::{Engine, Error, Fonts, TimeOfDay};
use std::time::{Duration, Instant};

/// 160x120 boyutunda, sabit başlangıç sayılı ve çalışır durumda bir motor.
fn engine() -> Engine {
    let mut e = Engine::new(160, 120, Fonts::embedded().unwrap(), 81).unwrap();
    e.set_active(true);
    e
}

/// Motorun güncel görüntüsünü yeni bir alana çizer.
fn frame(e: &mut Engine) -> Vec<u32> {
    let (w, h) = e.size();
    let mut p = vec![0; (w * h) as usize];
    e.render(&mut p).unwrap();
    p
}

/// Saniyede 10, 15, 25 veya 60 kez çizilse de dört saniye sonunda yağmur
/// aynı yerde olmalı ve aynı görüntü çıkmalı.
#[test]
fn frame_rate_does_not_change_simulation() {
    let time = TimeOfDay::new(12, 34, 56).unwrap();
    let mut expected = None;
    for fps in [10u64, 15, 25, 60] {
        let mut e = engine();
        let mut previous = 0;
        for n in 1..=fps * 4 {
            let now = n * 1_000_000_000 / fps;
            e.update(Duration::from_nanos(now - previous), time);
            previous = now;
            // Farklı çizim sayısı rastgele seçim sırasını değiştirmemeli.
            let _ = frame(&mut e);
        }
        assert_eq!(e.stats().steps, 100);
        let picture = frame(&mut e);
        if let Some(ref expected) = expected {
            assert_eq!(&picture, expected);
        } else {
            expected = Some(picture);
        }
    }
}

/// Durdurulmuş motor geç gelen çağrılarda iş yapmamalı; yeniden başlarken
/// duraklamada geçen süre yağmura eklenmemeli.
#[test]
fn pause_late_calls_and_resume() {
    let mut e = engine();
    let time = TimeOfDay::default();
    e.update(Duration::from_millis(20), time);
    e.set_active(false);
    let before = e.stats();
    e.update(Duration::from_secs(600), time);
    assert_eq!(e.render(&mut vec![0; 160 * 120]), Err(Error::Paused));
    assert_eq!(e.stats(), before);
    e.set_active(true);
    e.update(Duration::from_millis(20), time);
    assert_eq!(e.stats().steps, 0);
    e.update(Duration::from_millis(20), time);
    assert_eq!(e.stats().steps, 1);
}

/// Gece yarısı saat yenilenmeli; geçersiz boyut, saat ve yazı tipi reddedilmeli.
#[test]
fn midnight_redraw_and_input_limits() {
    let mut e = engine();
    e.update(Duration::ZERO, TimeOfDay::new(23, 59, 59).unwrap());
    let before = frame(&mut e);
    e.update(Duration::ZERO, TimeOfDay::new(0, 0, 0).unwrap());
    let after = frame(&mut e);
    assert_ne!(before, after);
    assert_eq!(after, frame(&mut e));
    assert_eq!(e.stats().steps, 0);
    assert_eq!(e.render(&mut [0]), Err(Error::ShortBuffer));
    assert_eq!(e.resize(0, 1), Err(Error::InvalidSize));
    assert_eq!(e.resize(u32::MAX, 1), Err(Error::InvalidSize));
    assert_eq!(e.size(), (160, 120));
    e.resize(1, 1).unwrap();
    assert_eq!(frame(&mut e).len(), 1);
    assert!(Fonts::from_bytes(Some("bozuk yazı tipi".as_bytes())).is_err());
    assert!(TimeOfDay::new(23, 60, 0).is_err());
}

/// Çözünürlük değiştiğinde motor bütün pikselleri yazmalı ve saati ortada tutmalı.
#[test]
fn display_resolutions_render_the_entire_frame() {
    let resolutions = [
        (720, 480),   // SD
        (720, 576),   // SD
        (1280, 720),  // HD
        (1366, 768),  // HD dizüstü ekranı
        (1920, 1080), // Full HD
        (2048, 1080), // 2K
        (2560, 1440), // QHD
        (2880, 1800), // 3K
        (3200, 1800), // 3K sınıfı
        (3840, 2160), // 4K UHD
        (4096, 2160), // 4K
        (2160, 3840), // Dikey 4K
        (7680, 4320), // 8K UHD
        (8192, 4320), // 8K
    ];
    let mut e = engine();
    let time = TimeOfDay::new(12, 34, 56).unwrap();
    let (r, g, b) = crate::config::CLOCK;
    let clock_color = u32::from_be_bytes([0, r, g, b]);

    for (width, height) in resolutions {
        e.resize(width, height)
            .unwrap_or_else(|error| panic!("{width}×{height}: {error}"));
        for _ in 0..80 {
            e.update(Duration::from_millis(40), time);
        }
        let (w, h) = (width as usize, height as usize);
        let end = w * h;
        let mut pixels = vec![u32::MAX; end + 1];
        let start = Instant::now();
        e.render(&mut pixels).unwrap();
        println!(
            "{width}×{height}: {:.2} ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
        assert!(
            !pixels[..end].contains(&u32::MAX),
            "{width}×{height}: yazılmayan piksel var"
        );
        assert_eq!(
            pixels[end],
            u32::MAX,
            "{width}×{height}: görüntü alanı aşıldı"
        );
        assert!(
            (h / 4..3 * h / 4)
                .any(|y| { pixels[y * w + w / 4..y * w + 3 * w / 4].contains(&clock_color) }),
            "{width}×{height}: saat orta bölgede görünmüyor"
        );
    }

    assert_eq!(e.resize(8192, 4321), Err(Error::InvalidSize));
    assert_eq!(e.size(), (8192, 4320));
}

/// Küçültülmüş yağmur yazı tipi yağmurun bütün harflerini, imza yazı tipi de
/// Türkçe Q klavyeyle yazılabilen bütün karakterleri içermeli.
#[test]
fn embedded_fonts_cover_rain_and_keyboard() {
    let rain = include_bytes!("../assets/fonts/NotoSansJP-Regular-Subset.otf");
    let kana = (0xFF66..=0xFF9D).filter_map(char::from_u32);
    for character in kana.chain('0'..='9').chain('A'..='Z') {
        assert!(
            Fonts::supports(rain, character),
            "yağmurda {character} eksik"
        );
    }
    let credit = include_bytes!("../assets/fonts/MatrixImza-Medium.ttf");
    let keyboard = (' '..='~').chain("ÇçĞğİıÖöŞşÜü€£½ÉéÆæß´¨©".chars());
    for character in keyboard {
        assert!(
            Fonts::supports(credit, character),
            "imzada {character} eksik"
        );
    }
}
