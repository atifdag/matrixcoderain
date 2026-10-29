package com.atifdag.matrixcoderain

import android.hardware.display.DisplayManager
import android.service.dreams.DreamService
import android.view.Display
import android.view.SurfaceView

/**
 * Android'in ekran koruyucu hizmeti.
 *
 * Sistem, telefon şarjdayken ve boşta kaldığında bu hizmeti başlatır. Dokunma
 * ekran koruyucuyu kapatır. Ekran kapanınca veya hizmet durunca çizim de durur;
 * kilit ekranı ve kimlik doğrulama sistemin kendisindedir.
 */
class MatrixDreamService : DreamService() {
    private var surface: SurfaceView? = null
    private var renderer: MatrixRenderer? = null

    /** Ekranın açılıp kapanmasını izler. */
    private val displayListener = object : DisplayManager.DisplayListener {
        override fun onDisplayAdded(displayId: Int) = Unit

        override fun onDisplayRemoved(displayId: Int) = Unit

        override fun onDisplayChanged(displayId: Int) {
            if (displayId == surface?.display?.displayId) updateDisplayState()
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        isInteractive = false
        isFullscreen = true
        val view = SurfaceView(this)
        surface = view
        renderer = MatrixRenderer(view)
        setContentView(view)
    }

    override fun onDreamingStarted() {
        super.onDreamingStarted()
        getSystemService(DisplayManager::class.java).registerDisplayListener(displayListener, null)
        updateDisplayState()
        renderer?.setDreaming(true)
    }

    override fun onDreamingStopped() {
        renderer?.setDreaming(false)
        getSystemService(DisplayManager::class.java).unregisterDisplayListener(displayListener)
        super.onDreamingStopped()
    }

    override fun onDetachedFromWindow() {
        renderer?.release()
        renderer = null
        surface = null
        super.onDetachedFromWindow()
    }

    /** Ekran yalnız tam açıkken çizim yapılır; kısılmış veya kapalı ekranda durur. */
    private fun updateDisplayState() {
        val state = surface?.display?.state ?: Display.STATE_ON
        renderer?.setDisplayOn(state == Display.STATE_ON)
    }
}
