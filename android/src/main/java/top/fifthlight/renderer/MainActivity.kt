package top.fifthlight.renderer

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.util.Log
import android.view.View
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.lifecycle.lifecycleScope
import com.google.androidgamesdk.GameActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity: GameActivity() {
    init {
        System.loadLibrary("renderer_android")
    }

    companion object {
        private val REQUEST_CODE_OPEN_FILE = 1
    }

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