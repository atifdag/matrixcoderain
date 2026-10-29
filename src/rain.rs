//! Akan harflerin benzetimi.
//!
//! Bu dosya ekrana hiçbir şey çizmez; yalnızca hangi hücrede hangi harfin
//! bulunduğunu ve damlaların nerede olduğunu hesaplar. İşletim sisteminden
//! tamamen bağımsızdır.

use crate::rng::Rng;

/// Harf paletinden hangi gruplardan harf seçileceğini tanımlar.
///
/// Her grup `(başlangıç, adet)` çiftidir ve palet dizisindeki bir aralığı gösterir.
/// Oranlar filmdeki görünüme yakındır: %70 katakana, %20 rakam, %10 Latin harfi.
#[derive(Clone, Copy)]
pub struct Picker {
    /// Katakana harfleri.
    pub kana: (u16, u16),
    /// 0-9 rakamları.
    pub digits: (u16, u16),
    /// A-Z büyük Latin harfleri.
    pub letters: (u16, u16),
}

impl Picker {
    /// Rastgele bir palet dizini döndürür.
    fn pick(&self, rng: &mut Rng) -> u16 {
        let chosen = match rng.range(0, 10) {
            0..=6 => self.kana,
            7..=8 => self.digits,
            _ => self.letters,
        };
        // Seçilen grup boşsa (örneğin yazı tipinde katakana yoksa) dolu olan ilk grubu kullan.
        let (start, count) = if chosen.1 == 0 {
            [self.kana, self.digits, self.letters]
                .into_iter()
                .find(|g| g.1 > 0)
                .unwrap_or((0, 1))
        } else {
            chosen
        };
        start + rng.range(0, count as i32) as u16
    }
}

/// Yukarıdan aşağıya akan tek bir damla.
pub struct Streak {
    /// Sütun numarası.
    pub col: i32,
    /// Baş harfin bulunduğu satır; eksi değer henüz görünmediği anlamına gelir.
    pub head: f32,
    /// Her 40 milisaniyelik benzetim adımında kaç satır ilerlediği.
    pub speed: f32,
    /// İzin uzunluğu, satır olarak.
    pub len: i32,
}

/// Tüm yağmurun durumu: hücrelerdeki harfler ve damlalar.
pub struct Rain {
    /// Ekrana sığan sütun sayısı.
    cols: i32,
    /// Ekrana sığan satır sayısı.
    rows: i32,
    /// Her hücre için palet dizini; `cols * rows` adet, sütun sütun sıralı.
    cells: Vec<u16>,
    /// Ekranda akan damlalar.
    drops: Vec<Streak>,
    /// Harf ve damla seçimlerinin tek rastgele sayı kaynağı.
    rng: Rng,
    /// Rastgele harf seçerken kullanılan gruplar.
    picker: Picker,
}

impl Rain {
    /// Aynı boyut ve başlangıç sayısı aynı ilk yağmur durumunu oluşturur.
    pub fn new(cols: i32, rows: i32, picker: Picker, seed: u64) -> Rain {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let mut rng = Rng::new(seed);
        let cells = (0..cols * rows).map(|_| picker.pick(&mut rng)).collect();

        // Sık bir yağmur için her sütuna yaklaşık 1,5 damla.
        let drop_count = cols + cols / 2;
        let mut drops = Vec::with_capacity(drop_count as usize);
        for i in 0..drop_count {
            let col = if i < cols { i } else { rng.range(0, cols) };
            drops.push(Self::spawn_drop(&mut rng, col, rows, true));
        }

        Rain {
            cols,
            rows,
            cells,
            drops,
            rng,
            picker,
        }
    }

    /// Görüntüye sığan hücre satırı sayısını verir.
    pub fn rows(&self) -> i32 {
        self.rows
    }

    /// Hücrelerdeki harf seçimlerini kopyalamadan okur.
    pub fn cells(&self) -> &[u16] {
        &self.cells
    }

    /// Çiziciye damlaların konum ve uzunluklarını ödünç verir.
    pub fn drops(&self) -> &[Streak] {
        &self.drops
    }

    /// Yeni bir damla üretir. İlk damlalar geniş bir aralığa dağıtılır ki ekran
    /// tek bir dalga halinde bir anda dolmasın.
    fn spawn_drop(rng: &mut Rng, col: i32, rows: i32, initial: bool) -> Streak {
        let max_wait = if initial { rows * 2 } else { rows };
        Streak {
            col,
            head: -(rng.range(0, max_wait.max(1)) as f32),
            speed: 0.3 + rng.unit() * 0.7,
            len: rng.range(4, (rows * 3 / 4).max(6)),
        }
    }

    /// Benzetimi bir sabit zaman adımı ilerletir.
    pub fn step(&mut self) {
        // Titreşim etkisi: her benzetim adımında hücrelerin küçük bir kısmındaki harfi değiştir.
        let mutations = (self.cells.len() / 60).max(1);
        for _ in 0..mutations {
            let i = self.rng.range(0, self.cells.len() as i32) as usize;
            self.cells[i] = self.picker.pick(&mut self.rng);
        }

        let rows = self.rows;
        let cols = self.cols;
        for d in &mut self.drops {
            d.head += d.speed;
            // Damla izinin tamamı ekranın altından çıktıysa yeniden başlat.
            if d.head - d.len as f32 > rows as f32 + 1.0 {
                let col = if self.rng.range(0, 3) == 0 {
                    self.rng.range(0, cols)
                } else {
                    d.col
                };
                *d = Self::spawn_drop(&mut self.rng, col, rows, false);
            }
        }
    }
}
