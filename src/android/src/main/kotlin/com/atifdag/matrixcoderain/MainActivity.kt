package com.atifdag.matrixcoderain

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

/**
 * Uygulama simgesinden açılan bilgi ekranı.
 *
 * Animasyonu kendisi göstermez; kullanıcıyı sistemin ekran koruyucu ayarlarına
 * yönlendirir. Ekran koruyucuyu yalnız sistem başlatır.
 */
class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val padding = (24 * resources.displayMetrics.density).toInt()
        val content = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding, padding, padding)
            addView(TextView(context).apply {
                setText(R.string.info)
                textSize = 16f
            })
            addView(Button(context).apply {
                setText(R.string.open_settings)
                setOnClickListener { openDreamSettings() }
            })
        }
        // Dış kap, durum ve gezinme çubuklarının altında kalmamak için kenar boşluğu alır.
        setContentView(FrameLayout(this).apply {
            fitsSystemWindows = true
            addView(content)
        })
    }

    /** Ekran koruyucu ayarlarını açar; cihazda bu sayfa yoksa ekran ayarlarına gider. */
    private fun openDreamSettings() {
        for (page in listOf(Settings.ACTION_DREAM_SETTINGS, Settings.ACTION_DISPLAY_SETTINGS)) {
            try {
                startActivity(Intent(page))
                return
            } catch (e: ActivityNotFoundException) {
                // Bu sayfa cihazda yok; sıradakine geçilir.
            }
        }
    }
}
