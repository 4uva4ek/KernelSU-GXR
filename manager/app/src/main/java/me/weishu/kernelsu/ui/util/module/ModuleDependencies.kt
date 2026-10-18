package me.weishu.kernelsu.ui.util.module

import android.net.Uri
import me.weishu.kernelsu.ksuApp
import java.io.File
import java.security.MessageDigest
import java.util.zip.ZipFile

data class ModuleDependencyApproval(
    val sha256: String,
    val moduleId: String,
    val remove: List<String>,
    val disable: List<String>,
)

object ModuleDependencies {
    private val idPattern = Regex("gxr_[A-Za-z0-9_-]+")

    fun ids(properties: Map<String, String>, key: String): List<String> {
        val value = properties[key] ?: return emptyList()
        val ids = value.split(',')
        require(ids.size in 1..16 && ids.distinct().size == ids.size) { "Invalid dependency count or duplicates" }
        require(ids.all { idPattern.matches(it) && it != properties["id"] }) { "Invalid dependency ID" }
        return ids
    }

    fun sha256(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(64 * 1024)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                digest.update(buffer, 0, count)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it.toInt() and 255) }
    }

    fun inspect(uri: Uri, installed: Set<String>): ModuleDependencyApproval {
        val file = File.createTempFile("dependency-", ".zip", ksuApp.cacheDir)
        try {
            val input = ksuApp.contentResolver.openInputStream(uri) ?: error("Cannot open module ZIP")
            input.use { stream -> file.outputStream().use { stream.copyTo(it) } }
            val properties = ZipFile(file).use { zip ->
                val entry = zip.getEntry("module.prop") ?: error("Missing module.prop")
                require(entry.size in 1..16384) { "Invalid module.prop size" }
                zip.getInputStream(entry).bufferedReader().use { reader ->
                    reader.readText().lineSequence().filter { '=' in it && !it.startsWith('#') }
                        .associate { it.substringBefore('=').trim() to it.substringAfter('=').trim() }
                }
            }
            val id = properties["id"] ?: error("Missing module ID")
            val remove = ids(properties, "gxrReplaces")
            val disable = ids(properties, "gxrRequiresDisabled")
            require(remove.intersect(disable.toSet()).isEmpty()) { "Ambiguous module dependencies" }
            return ModuleDependencyApproval(sha256(file), id, remove.filter { it in installed }, disable.filter { it in installed })
        } finally {
            file.delete()
        }
    }
}
