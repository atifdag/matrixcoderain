package com.atifdag.matrixcoderain

import android.graphics.Bitmap
import android.os.Handler
import android.os.HandlerThread
import android.os.SystemClock
import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView
import java.nio.ByteBuffer
import java.time.LocalTime
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.math.max

/**
 * Yağmuru ayrı bir iş parçacığında çizer.
 *
 * Çizim yalnız ekran koruyucu çalışırken, ekran açıkken ve çizim yüzeyi hazırken
 * yapılır. Bu koşullardan biri kalkınca bekleyen çizim işi kaldırılır; yeni iş
 * zamanlanmaz. Motor, görüntü alanı ve yüzey yalnız çizim iş parçacığında kullanılır.
 *
 * Pil, ısı ve bellek için yüzey ekranın yarı boyutunda istenir; görüntü yüzeye
 * bire bir çizilir ve ekrana büyütmeyi ekran donanımı yapar.
 */
internal class MatrixRenderer(view: SurfaceView) : SurfaceHolder.Callback {
    private val holder = view.holder
    private val thread = HandlerThread("MatrixKodYagmuru").apply { start() }
    private val handler = Handler(thread.looper)
    private val drawTask = Runnable { drawFrame() }

    // Aşağıdaki alanlara yalnız çizim iş parçacığından erişilir.
    private var engine = 0L
    private var bitmap: Bitmap? = null
    private var pixels: ByteBuffer? = null
    private var surfaceReady = false
    private var dreaming = false
    private var displayOn = true
    private var drawing = false
    private var lastTick = 0L
    private var lastReport = 0L

    init {
        holder.addCallback(this)
        // Görünüm her yerleştiğinde yüzey, görünüm boyutunun yarısı olarak istenir.
        // Boyut değişmemişse sistem bu isteği yok sayar.
        view.addOnLayoutChangeListener { _, left, top, right, bottom, _, _, _, _ ->
            holder.setFixedSize(
                max(1, ((right - left) * RENDER_SCALE).toInt()),
                max(1, ((bottom - top) * RENDER_SCALE).toInt()),
            )
        }
    }

    /** Sistem ekran koruyucuyu başlattı veya durdurdu. */
    fun setDreaming(value: Boolean) = onRenderThread {
        dreaming = value
        refresh()
    }

    /** Ekran açıldı veya kapandı. */
    fun setDisplayOn(value: Boolean) = onRenderThread {
        displayOn = value
        refresh()
    }

    override fun surfaceCreated(holder: SurfaceHolder) = Unit

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) =
        onRenderThread {
            surfaceReady = true
            prepare(width, height)
            refresh()
        }

    /** Yüzey kaybolmadan önce süren çizimin bitmesi beklenir; yüzeye geç erişim olmaz. */
    override fun surfaceDestroyed(holder: SurfaceHolder) = onRenderThreadAndWait {
        surfaceReady = false
        refresh()
    }

    /** Motoru ve iş parçacığını kapatır. Tekrar çağrılması güvenlidir. */
    fun release() {
        holder.removeCallback(this)
        if (!thread.isAlive) return
        onRenderThreadAndWait {
            surfaceReady = false
            refresh()
            destroyEngine()
        }
        thread.quitSafely()
    }

    private fun canDraw() = dreaming && displayOn && surfaceReady && engine > 0

    /** Koşullara göre çizimi başlatır ya da bekleyen bütün çizim işlerini kaldırır. */
    private fun refresh() {
        handler.removeCallbacks(drawTask)
        val active = canDraw()
        if (engine > 0) MatrixNative.setActive(engine, active)
        if (active != drawing) {
            drawing = active
            report(if (active) "Çizim başladı" else "Çizim durdu")
        }
        if (active) {
            // Duraklamada geçen süre yağmura eklenmez.
            lastTick = SystemClock.elapsedRealtimeNanos()
            handler.post(drawTask)
        }
    }

    /**
     * Sayaçları ve çizim sürelerini sistem kaydına yazar (`adb logcat -s MatrixKodYagmuru`).
     * Durunca ve yeniden başlayınca kare sayısının aynı kalması, aradaki sürede
     * çizim yapılmadığını gösterir.
     */
    private fun report(reason: String) {
        if (engine <= 0) return
        MatrixNative.statsText(engine)?.let { Log.i(TAG, "$reason: $it") }
        lastReport = SystemClock.elapsedRealtimeNanos()
    }

    /** Yüzey boyutunda motor ve görüntü alanı hazırlar; boyut aynıysa dokunmaz. */
    private fun prepare(w: Int, h: Int) {
        if (engine > 0 && bitmap?.width == w && bitmap?.height == h) return
        destroyEngine()
        val created = MatrixNative.create(w, h, System.nanoTime())
        if (created <= 0) {
            Log.w(TAG, "Çizim motoru oluşturulamadı, sonuç kodu: ${-created}")
            return
        }
        engine = created
        bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        pixels = ByteBuffer.allocateDirect(w * h * 4)
    }

    /** Motoru ve görüntü alanlarını bırakır. */
    private fun destroyEngine() {
        if (engine > 0) MatrixNative.destroy(engine)
        engine = 0
        bitmap?.recycle()
        bitmap = null
        pixels = null
    }

    /** Bir kare çizer ve koşullar sürüyorsa bir sonrakini zamanlar. */
    private fun drawFrame() {
        if (!canDraw()) return
        val bitmap = bitmap ?: return
        val pixels = pixels ?: return
        // Sonraki kare bu karenin başlangıcına göre zamanlanır; çizim süresi aralığa eklenmez.
        val nextFrame = SystemClock.uptimeMillis() + FRAME_MS
        val now = SystemClock.elapsedRealtimeNanos()
        val time = LocalTime.now()
        MatrixNative.update(engine, (now - lastTick) / 1_000, time.hour, time.minute, time.second)
        lastTick = now
        // ARGB_8888 resim bellekte kırmızı, yeşil, mavi, saydamlık sırasıyla durur;
        // Rust'ın yazdığı sırayla aynıdır, dönüştürme gerekmez.
        if (MatrixNative.render(engine, pixels, bitmap.width * 4) == 0) {
            bitmap.copyPixelsFromBuffer(pixels)
            pixels.rewind()
            present(bitmap)
        }
        if (now - lastReport >= REPORT_NS) report("Çalışıyor")
        handler.postAtTime(drawTask, nextFrame)
    }

    /** Görüntüyü yüzeye bire bir koyar. Yüzey kullanılamıyorsa kareyi atlar. */
    private fun present(bitmap: Bitmap) {
        val canvas = try {
            holder.lockHardwareCanvas()
        } catch (e: IllegalStateException) {
            Log.w(TAG, "Çizim yüzeyi kullanılamıyor.", e)
            null
        } ?: return
        try {
            canvas.drawBitmap(bitmap, 0f, 0f, null)
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    /** İşi çizim iş parçacığının sırasına ekler. */
    private fun onRenderThread(work: () -> Unit) {
        handler.post { work() }
    }

    /** İşi çizim iş parçacığında yapar ve bitmesini en çok bir saniye bekler. */
    private fun onRenderThreadAndWait(work: () -> Unit) {
        val done = CountDownLatch(1)
        handler.post {
            try {
                work()
            } finally {
                done.countDown()
            }
        }
        done.await(WAIT_MS, TimeUnit.MILLISECONDS)
    }

    private companion object {
        const val TAG = "MatrixKodYagmuru"

        /** İki kare arasındaki süre: saniyede yaklaşık 15 kare. */
        const val FRAME_MS = 66L

        /** Yüzey ve görüntü her iki eksende ekranın yarı boyutundadır. */
        const val RENDER_SCALE = 0.5f

        /** Yüzey kapanırken çizimin bitmesini bekleme sınırı. */
        const val WAIT_MS = 1_000L

        /** Çalışırken sayaçların kayda yazılma aralığı: bir dakika. */
        const val REPORT_NS = 60_000_000_000L
    }
}
