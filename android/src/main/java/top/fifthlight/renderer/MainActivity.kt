package top.fifthlight.renderer

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.util.Log
import android.view.MotionEvent
import android.view.View
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsAnimationCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.androidgamesdk.GameActivity

class MainActivity: GameActivity() {
    init {
        System.loadLibrary("renderer_android")
    }

    @JvmField
    var imeInsets = Insets.NONE

    val contentView: View
        get() {
            return window.decorView.findViewById(android.R.id.content)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        WindowCompat.getInsetsController(window, window.decorView).apply {
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            hide(WindowInsetsCompat.Type.systemBars())
        }

        ViewCompat.setWindowInsetsAnimationCallback(
            this.contentView,
            object : WindowInsetsAnimationCompat.Callback(DISPATCH_MODE_STOP) {
                override fun onProgress(
                    insets: WindowInsetsCompat,
                    runningAnimations: List<WindowInsetsAnimationCompat?>
                ): WindowInsetsCompat {
                    imeInsets = insets.getInsets(WindowInsetsCompat.Type.ime())
                    return insets
                }
            }
        )
    }

    @Suppress("unused")
    fun enablePointerLock() {
        Log.d("MainActivity", "enablePointerLock")
        contentView.requestPointerCapture()
    }

    @Suppress("unused")
    fun disablePointerLock() {
        Log.d("MainActivity", "disablePointerLock")
        contentView.releasePointerCapture()
    }

    @Suppress("unused")
    fun openUrl(url: String) {
        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
    }
}