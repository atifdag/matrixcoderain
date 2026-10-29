package com.atifdag.matrixcoderain

import java.nio.ByteBuffer

/**
 * Rust kitaplığındaki çizim motoruna erişim.
 *
 * Bir motorun bütün çağrıları onu oluşturan iş parçacığından yapılmalıdır.
 * Sonuç kodları: 0 başarılı, 1 geçersiz girdi, 2 geçersiz motor, 3 kısa görüntü
 * alanı, 4 çizim durdurulmuş, 5 yanlış iş parçacığı, 6 beklenmeyen hata.
 */
internal object MatrixNative {
    init {
        System.loadLibrary("matrix_core")
    }

    /** Durdurulmuş bir motor oluşturur. Başarıda kimliği (sıfırdan büyük), hatada sonuç kodunun eksi değerini döndürür. */
    external fun create(width: Int, height: Int, seed: Long): Long

    /** Çizimi başlatır veya durdurur. */
    external fun setActive(handle: Long, active: Boolean): Int

    /** Geçen süreyi mikrosaniye olarak ve ekranda gösterilecek saati verir. */
    external fun update(handle: Long, elapsedMicros: Long, hour: Int, minute: Int, second: Int): Int

    /** Görüntüyü doğrudan bellekli alana her piksel için kırmızı, yeşil, mavi, 255 sırasıyla yazar. */
    external fun render(handle: Long, target: ByteBuffer, stride: Int): Int

    /** Sayaçları ve son karelerin çizim sürelerini Türkçe tek satır olarak verir; hata olursa null. */
    external fun statsText(handle: Long): String?

    /** Motoru kapatır. */
    external fun destroy(handle: Long): Int
}
