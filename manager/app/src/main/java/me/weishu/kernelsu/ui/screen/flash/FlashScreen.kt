package me.weishu.kernelsu.ui.screen.flash

import android.widget.Toast
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.dropUnlessResumed
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.Natives
import me.weishu.kernelsu.R
import me.weishu.kernelsu.data.repository.isSoftRebootPreferred
import me.weishu.kernelsu.ui.LocalUiMode
import me.weishu.kernelsu.ui.UiMode
import me.weishu.kernelsu.ui.navigation3.LocalNavigator
import me.weishu.kernelsu.ui.util.reboot
import me.weishu.kernelsu.data.repository.ModuleRepositoryImpl
import me.weishu.kernelsu.ui.util.module.ModuleDependencies
import me.weishu.kernelsu.ui.util.module.ModuleDependencyApproval

@Composable
fun FlashScreen(flashIt: FlashIt) {
    val navigator = LocalNavigator.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var text by rememberSaveable { mutableStateOf("") }
    val logContent = remember { StringBuilder() }
    var showRebootAction by rememberSaveable { mutableStateOf(false) }
    var flashingStatus by rememberSaveable { mutableStateOf(FlashingStatus.FLASHING) }
    val needJailbreakWarning = flashIt is FlashIt.FlashBoot && Natives.isLateLoadMode
    // Soft reboot keeps the jailbreak and still applies modules
    val softReboot = flashIt is FlashIt.FlashModules && isSoftRebootPreferred()
    var flashingEnabled by rememberSaveable { mutableStateOf(!needJailbreakWarning) }
    var dependenciesReviewed by remember { mutableStateOf(flashIt !is FlashIt.FlashModules) }
    var dependencyPrompt by remember { mutableStateOf(false) }
    var approvals by remember { mutableStateOf(emptyMap<String, ModuleDependencyApproval>()) }
    val uiMode = LocalUiMode.current
    val snackbarHost = remember { SnackbarHostState() }

    LaunchedEffect(flashIt) {
        if (flashIt !is FlashIt.FlashModules || text.isNotEmpty()) return@LaunchedEffect
        try {
            val modules = ModuleRepositoryImpl().getModules().getOrThrow()
            approvals = withContext(Dispatchers.IO) {
                flashIt.uris.associate { uri -> uri.toString() to ModuleDependencies.inspect(uri, modules.map { it.id }.toSet()) }
            }
            dependencyPrompt = approvals.values.any { it.remove.isNotEmpty() || it.disable.isNotEmpty() }
            dependenciesReviewed = !dependencyPrompt
        } catch (error: CancellationException) {
            throw error
        } catch (error: Exception) {
            text = context.getString(R.string.gxr_dependency_failure, error.message.orEmpty())
            logContent.append(text).append('\n')
            flashingStatus = FlashingStatus.FAILED
        }
    }

    if (dependencyPrompt) {
        AlertDialog(
            onDismissRequest = { navigator.pop() },
            title = { Text(context.getString(R.string.gxr_dependency_title)) },
            text = { Text(buildString {
                approvals.values.forEach { plan ->
                    if (plan.remove.isNotEmpty() || plan.disable.isNotEmpty()) {
                        append(plan.moduleId).append('\n')
                        if (plan.remove.isNotEmpty()) append(context.getString(R.string.gxr_dependency_remove, plan.remove.joinToString(", "))).append('\n')
                        if (plan.disable.isNotEmpty()) append(context.getString(R.string.gxr_dependency_disable, plan.disable.joinToString(", "))).append('\n')
                    }
                }
                append('\n').append(context.getString(R.string.gxr_dependency_details))
            }) },
            confirmButton = { TextButton(onClick = { dependencyPrompt = false; dependenciesReviewed = true }) { Text(context.getString(R.string.gxr_dependency_confirm)) } },
            dismissButton = { TextButton(onClick = { navigator.pop() }) { Text(context.getString(R.string.gxr_dependency_cancel)) } },
        )
    }

    fun showMessage(message: String) {
        scope.launch {
            if (uiMode == UiMode.Material) {
                snackbarHost.showSnackbar(message)
            } else {
                Toast.makeText(context, message, Toast.LENGTH_SHORT).show()
            }
        }
    }

    FlashEffect(
        flashIt = flashIt,
        text = text,
        logContent = logContent,
        onTextUpdate = { text = it },
        onShowRebootChange = { showRebootAction = it },
        onFlashingStatusChange = { flashingStatus = it },
        enabled = flashingEnabled && dependenciesReviewed,
        approvals = approvals,
    )

    val state = FlashUiState(
        text = text,
        showRebootAction = showRebootAction,
        flashingStatus = flashingStatus,
        showJailbreakWarning = needJailbreakWarning && !flashingEnabled,
        rebootLabelRes = if (softReboot) R.string.reboot_soft else R.string.reboot,
    )
    val actions = FlashScreenActions(
        onBack = dropUnlessResumed { navigator.pop() },
        onSaveLog = saveLog(logContent, scope) { showMessage(it) },
        onReboot = {
            scope.launch {
                withContext(Dispatchers.IO) {
                    reboot(if (softReboot) "soft_reboot" else "")
                }
            }
        },
        onConfirmJailbreakWarning = { flashingEnabled = true },
        onDismissJailbreakWarning = dropUnlessResumed { navigator.pop() },
    )

    when (LocalUiMode.current) {
        UiMode.Miuix -> FlashScreenMiuix(state, actions)
        UiMode.Material -> FlashScreenMaterial(state, actions, snackbarHost)
    }
}
