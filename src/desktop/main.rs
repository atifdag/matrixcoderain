//! Matrix - Kod Yağmuru masaüstü programı.
//!
//! Windows'ta `.scr` ekran koruyucusu olarak, Linux'ta xscreensaver'ın verdiği
//! pencerede, her sistemde ise deneme penceresi olarak çalışır.
#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
mod app;
mod font_source;
mod platform;
mod power;
mod settings;

use app::Mode;
use settings::APP_NAME;
use std::path::PathBuf;

/// Komut satırından seçilen çalışma biçimi.
#[derive(Debug, PartialEq, Eq)]
enum Launch {
    /// Bütün ekranları kaplayan ekran koruyucu.
    Saver,
    /// Boyutu değiştirilebilen deneme penceresi.
    Window,
    /// Başka bir programın verdiği pencerenin içine çizim (kimliği ile).
    Embedded(u64),
    /// Windows'un "Ayarlar" düğmesi.
    Config,
    /// Kullanım bilgisi.
    Help,
}

/// Komut satırının çözümlenmiş hali.
#[derive(Debug)]
struct Options {
    launch: Launch,
    /// Kullanıcının açıkça seçtiği yağmur yazı tipi dosyası.
    font: Option<PathBuf>,
}

const HELP: &str = "Matrix - Kod Yağmuru

/s                  Ekran koruyucu
/w                  Deneme penceresi
/p <pencere>        Windows ayarlarındaki küçük önizleme
/c                  Ayarlar bilgisi
-root               xscreensaver altında çalışma (Linux, X11)
-window-id <kimlik> Verilen X11 penceresine çizim (Linux)
--font <dosya>      Yağmur için başka bir yazı tipi
--help              Bu yardım";

/// Pencere kimliğini onlu veya `0x` ile başlayan on altılı sayı olarak okur.
/// Windows önizlemesi onlu, X11 araçları çoğunlukla on altılı kimlik verir.
fn parse_window_id(text: &str) -> Option<u64> {
    let text = text.trim();
    let value = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).ok()?,
        None => text.parse().ok()?,
    };
    (value > 0).then_some(value)
}

/// Komut satırını çözümler. `xscreensaver_window`, xscreensaver'ın ortam
/// değişkeniyle verdiği pencere kimliğidir; denetim kolaylığı için dışarıdan gelir.
fn parse_args(
    args: impl IntoIterator<Item = String>,
    xscreensaver_window: Option<String>,
) -> Result<Options, String> {
    let mut result = Options {
        // Windows, ayarlar düğmesinde programı seçeneksiz de çalıştırabilir.
        launch: if cfg!(windows) {
            Launch::Config
        } else {
            Launch::Window
        },
        font: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        // Windows seçenekleri `/p:123` biçiminde de gelebilir.
        let (raw, inline) = arg
            .split_once(':')
            .map_or((arg.as_str(), None), |(k, v)| (k, Some(v.to_string())));
        let key = raw.trim_start_matches(['/', '-']).to_ascii_lowercase();
        match key.as_str() {
            "s" | "saver" => result.launch = Launch::Saver,
            "w" | "window" => result.launch = Launch::Window,
            "c" | "config" => result.launch = Launch::Config,
            "h" | "help" | "?" => result.launch = Launch::Help,
            "p" | "preview" | "window-id" => {
                let id = inline
                    .or_else(|| args.next())
                    .as_deref()
                    .and_then(parse_window_id)
                    .ok_or("Geçerli bir pencere kimliği gerekli.")?;
                result.launch = Launch::Embedded(id);
            }
            "root" => {
                // Kendi başına çalıştırıldığında masaüstü arka planına çizmek kapsam dışıdır.
                let id = xscreensaver_window
                    .as_deref()
                    .and_then(parse_window_id)
                    .ok_or("-root yalnız xscreensaver altında kullanılabilir.")?;
                result.launch = Launch::Embedded(id);
            }
            "font" => {
                let path = inline
                    .or_else(|| args.next())
                    .filter(|v| !v.is_empty())
                    .ok_or("Yazı tipi dosyası belirtilmedi.")?;
                result.font = Some(PathBuf::from(path));
            }
            _ => return Err(format!("Bilinmeyen seçenek: {arg}")),
        }
    }
    Ok(result)
}

/// Seçilen biçimi çalıştırır; hata olursa kullanıcıya gösterilecek metni döndürür.
fn run() -> Result<(), String> {
    let mut options = parse_args(
        std::env::args().skip(1),
        std::env::var("XSCREENSAVER_WINDOW").ok(),
    )?;
    if options.font.is_none() {
        options.font = std::env::var_os("MATRIXCODERAIN_FONT").map(PathBuf::from);
    }
    let mode = match options.launch {
        Launch::Help => {
            platform::show_message(APP_NAME, HELP);
            return Ok(());
        }
        Launch::Config => {
            platform::show_message(
                APP_NAME,
                "Ekran koruyucu ve dönüşte oturum açma ayarları işletim sisteminden yönetilir.",
            );
            return Ok(());
        }
        Launch::Saver => Mode::Saver,
        Launch::Window => Mode::Window,
        Launch::Embedded(id) => Mode::Embedded(id),
    };
    app::run(mode, options.font)
}

fn main() {
    if let Err(e) = run() {
        platform::show_message(&format!("{APP_NAME} - Hata"), &e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Yalnız komut satırıyla çözümleme yapar; xscreensaver değişkeni verilmez.
    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(args.iter().map(|s| s.to_string()), None)
    }

    /// Windows'un ekran koruyucu seçenekleri büyük/küçük harf ve iki nokta
    /// biçimleriyle tanınmalı; eksik veya hatalı değerler reddedilmeli.
    #[test]
    fn windows_options() {
        for value in ["/s", "/S", "--saver"] {
            assert_eq!(parse(&[value]).unwrap().launch, Launch::Saver);
        }
        assert_eq!(parse(&["/p:123"]).unwrap().launch, Launch::Embedded(123));
        assert_eq!(parse(&["/P", "123"]).unwrap().launch, Launch::Embedded(123));
        assert_eq!(parse(&["/w"]).unwrap().launch, Launch::Window);
        assert_eq!(parse(&["/c:123"]).unwrap().launch, Launch::Config);
        for args in [
            &["/p"][..],
            &["/p:0"],
            &["/p", "abc"],
            &["--font"],
            &["--bilinmeyen"],
        ] {
            assert!(parse(args).is_err());
        }
    }

    /// xscreensaver seçenekleri: on altılı kimlik ve ortam değişkeniyle `-root`.
    #[test]
    fn xscreensaver_options() {
        assert_eq!(
            parse(&["-window-id", "0x2a00007"]).unwrap().launch,
            Launch::Embedded(0x2a00007)
        );
        let root = parse_args(["-root".to_string()], Some("0x1c00003".to_string()));
        assert_eq!(root.unwrap().launch, Launch::Embedded(0x1c00003));
        assert!(parse(&["-root"]).is_err());
    }

    /// Yazı tipi dosyası iki biçimde de verilebilmeli.
    #[test]
    fn font_option() {
        let options = parse(&["--font", "yazi.otf", "/w"]).unwrap();
        assert_eq!(options.font, Some(PathBuf::from("yazi.otf")));
        assert_eq!(options.launch, Launch::Window);
    }
}
