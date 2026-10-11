package me.weishu.kernelsu.ui.util

import android.os.SystemClock
import android.util.Log
import android.widget.Toast
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.R
import me.weishu.kernelsu.ksuApp
import java.util.concurrent.atomic.AtomicBoolean

// Application lifetime: removing the menu/composable must not cancel an
// already submitted restart. All entry points (Home, Modules, Flash) use this.
private val rebootScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
private val rebootPending = AtomicBoolean(false)

fun reboot(reason: String = "") {
    if (!rebootPending.compareAndSet(false, true)) return
    rebootScope.launch {
        val started = SystemClock.elapsedRealtime()
        Toast.makeText(ksuApp, R.string.reboot_request_starting, Toast.LENGTH_SHORT).show()
        Log.i("RebootRequest", "Submitting restart request: $reason")
        try {
            val accepted = withContext(Dispatchers.IO) { executeReboot(reason) }
            Log.i("RebootRequest", "Restart request accepted=$accepted in ${SystemClock.elapsedRealtime() - started}ms")
            Toast.makeText(
                ksuApp,
                if (accepted) R.string.reboot_request_accepted else R.string.reboot_request_failed,
                Toast.LENGTH_LONG
            ).show()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e("RebootRequest", "Restart request failed", e)
            Toast.makeText(ksuApp, R.string.reboot_request_failed, Toast.LENGTH_LONG).show()
        } finally {
            rebootPending.set(false)
        }
    }
}
