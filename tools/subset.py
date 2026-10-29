# Matrix - Kod Yağmuru: gömülü yazı tiplerini yalnız kullanılan harflere küçültür.
#
# Programın kullandığı yazı tipi kitaplığı, bir yazı tipindeki bütün harfleri
# baştan belleğe açar. Bu yüzden programa yalnız gereken harfler gömülür.
#
# Özgün dosyalar: Noto CJK Sans 2.004 sürümündeki NotoSansJP-Regular.otf ve
# Source Sans 3.052R sürümündeki TTF/SourceSans3-Medium.ttf.
#
# Kullanım:
#   uvx --from fonttools==4.60.1 python tools/subset.py NotoSansJP-Regular.otf SourceSans3-Medium.ttf
#
# Çıktılar depodaki assets/fonts klasörüne yazılır:
#   NotoSansJP-Regular-Subset.otf  Yağmur: yarım genişlikli katakana, rakamlar, büyük Latin harfleri.
#   MatrixImza-Medium.ttf          İmza: Türkçe Q klavyedeki harfler, rakamlar, işaretler, boşluk ve ©.
#
# Source Sans 3'ün lisansı "Source" adını korur; değiştirilmiş bir kopya bu adı
# taşıyamaz. Bu yüzden imza yazı tipinin adı "Matrix Imza" yapılır. Telif ve
# lisans satırları olduğu gibi kalır. Noto Sans JP'nin adında korunan ad
# geçmediği için onun adı değişmez.
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

# Çıktı klasörü: depodaki assets/fonts. Betik tools/ altındadır.
FONTS = Path(__file__).resolve().parent.parent / "assets" / "fonts"

# Yağmurda kullanılan karakterler: 0-9, A-Z ve yarım genişlikli katakana.
RAIN_CHARACTERS = [*range(0x30, 0x3A), *range(0x41, 0x5B), *range(0xFF66, 0xFF9E)]

# İmza yazı tipinde Türkçe Q klavyeyle yazılabilen her şey bulunur; böylece
# imza metni değiştirildiğinde yazı tipini yeniden üretmek gerekmez:
# boşluk, rakamlar, büyük ve küçük Latin harfleri ve ASCII işaretleri
# (U+0020-U+007E), büyük ve küçük Türkçe harfler, AltGr ile yazılan
# işaretler ve imzadaki ©.
ASCII_PRINTABLE = "".join(chr(c) for c in range(0x20, 0x7F))
TURKISH_LETTERS = "ÇçĞğİıÖöŞşÜü"
ALTGR_SIGNS = "€£½ÉéÆæß´¨"
CREDIT_CHARACTERS = [ord(c) for c in ASCII_PRINTABLE + TURKISH_LETTERS + ALTGR_SIGNS + "©"]

# İmza yazı tipinin yeni adı ve biçimi.
CREDIT_FAMILY = "Matrix Imza"
CREDIT_STYLE = "Medium"


def make_subset(source: Path, characters: list[int]) -> TTFont:
    """Yalnız verilen karakterleri tutar. Programın kullanmadığı ekran ipuçlarını
    (hinting) ve dizgi kurallarını atar; harflerin çizgileri ve ölçüleri değişmez."""
    options = subset.Options()
    options.layout_features = []
    options.hinting = False
    options.desubroutinize = True
    options.name_IDs = ["*"]
    options.name_legacy = True
    options.name_languages = ["*"]
    # Özgün tarih korunur; böylece betik her çalıştırmada aynı dosyayı üretir.
    font = TTFont(source, recalcTimestamp=False)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=characters)
    subsetter.subset(font)
    missing = [hex(c) for c in characters if c not in font.getBestCmap()]
    if missing:
        sys.exit(f"{source.name} içinde eksik karakterler: {missing}")
    return font


def rename(font: TTFont, family: str, style: str) -> None:
    """Yazı tipinin görünen adlarını değiştirir. Telif (0), marka (7) ve
    lisans (13, 14) satırlarına dokunulmaz."""
    full_name = f"{family} {style}"
    postscript_name = f"{family}-{style}".replace(" ", "")
    version = font["head"].fontRevision
    names = {
        1: full_name,
        3: f"{version:.3f};{postscript_name}",
        4: full_name,
        6: postscript_name,
        16: family,
        17: style,
    }
    table = font["name"]
    for name_id, value in names.items():
        table.removeNames(nameID=name_id)
        table.setName(value, name_id, 3, 1, 0x409)
        table.setName(value, name_id, 1, 0, 0)


def save(font: TTFont, file_name: str) -> None:
    """Yazı tipini kaydeder ve boyutunu ve harf sayısını yazar."""
    target = FONTS / file_name
    font.save(target)
    print(f"{file_name}: {target.stat().st_size} bayt, {len(font.getGlyphOrder())} harf")


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("Kullanım: subset.py <NotoSansJP-Regular.otf> <SourceSans3-Medium.ttf>")
    rain_source, credit_source = map(Path, sys.argv[1:])

    save(make_subset(rain_source, RAIN_CHARACTERS), "NotoSansJP-Regular-Subset.otf")

    credit = make_subset(credit_source, CREDIT_CHARACTERS)
    rename(credit, CREDIT_FAMILY, CREDIT_STYLE)
    save(credit, "MatrixImza-Medium.ttf")


if __name__ == "__main__":
    main()
