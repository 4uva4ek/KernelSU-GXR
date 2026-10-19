package me.weishu.kernelsu.data.model

import androidx.compose.runtime.Immutable

@Immutable
data class ModuleUpdateInfo(
    val downloadUrl: String,
    val version: String,
    val changelog: String,
    val deprecatedBy: String? = null,
    // Non-null when the feed retires the module without a successor.
    val retiredReason: String? = null,
) {
    companion object {
        val Empty = ModuleUpdateInfo("", "", "")
    }
}
