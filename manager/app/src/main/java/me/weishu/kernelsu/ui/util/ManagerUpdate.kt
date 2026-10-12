package me.weishu.kernelsu.ui.util

import android.content.pm.PackageManager
import android.net.Uri
import android.util.Log
import androidx.core.content.pm.PackageInfoCompat
import com.topjohnwu.superuser.ShellUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.ksuApp
import me.weishu.kernelsu.ui.util.module.LatestVersionInfo
import java.io.File
import java.io.InputStream
import java.security.MessageDigest

private const val TAG = "ManagerUpdate"

/** World-readable staging path for `pm install`; removed after the attempt. */
private const val STAGED_APK = "/data/local/tmp/ksu-gxr-manager-update.apk"

/**
 * Download the published Manager APK, verify it against the release manifest and the
 * installed Manager's signing certificate, then install it through root.
 *
 * Nothing is installed unless the SHA-256 from the release matches, the package name
 * is this Manager, every signer certificate equals the installed Manager's and the
 * version code increases. The Manager process is replaced by the installation.
 */
suspend fun installManagerUpdate(
    info: LatestVersionInfo,
    onProgress: (Int) -> Unit = {},
): Result<Unit> = withContext(Dispatchers.IO) {
    runCatching {
        require(info.downloadUrl.isNotEmpty()) { "The release has no Manager APK" }
        require(info.sha256.length == 64) { "The release has no SHA-256 for the APK" }

        var downloaded: Uri? = null
        download(
            url = info.downloadUrl,
            fileName = info.apkName.ifEmpty { "KernelSU-GXR-update.apk" },
            onDownloaded = { downloaded = it },
            onProgress = onProgress,
        )
        val uri = downloaded ?: error("Download failed")

        val file = File(ksuApp.cacheDir, "manager-update.apk")
        try {
            ksuApp.contentResolver.openInputStream(uri)?.use { input ->
                file.outputStream().use { input.copyTo(it) }
            } ?: error("Cannot read the downloaded file")

            val digest = file.inputStream().use { sha256Hex(it) }
            check(digest.equals(info.sha256, ignoreCase = true)) { "SHA-256 mismatch: $digest" }

            val pm = ksuApp.packageManager
            val flags = PackageManager.GET_SIGNING_CERTIFICATES
            val archive = pm.getPackageArchiveInfo(file.path, flags) ?: error("Not an APK")
            check(archive.packageName == ksuApp.packageName) { "Different package: ${archive.packageName}" }
            val installed = pm.getPackageInfo(ksuApp.packageName, flags)
            val ownSigners = installed.signingInfo?.apkContentsSigners?.map { sha256Hex(it.toByteArray()) }.orEmpty()
            val newSigners = archive.signingInfo?.apkContentsSigners?.map { sha256Hex(it.toByteArray()) }.orEmpty()
            check(ownSigners.isNotEmpty() && newSigners == ownSigners) {
                "Signing certificate differs from the installed Manager"
            }
            if (info.certSha256.isNotEmpty()) {
                check(newSigners.any { it.equals(info.certSha256, ignoreCase = true) }) {
                    "Certificate does not match the release manifest"
                }
            }
            val newCode = PackageInfoCompat.getLongVersionCode(archive)
            val oldCode = PackageInfoCompat.getLongVersionCode(installed)
            check(newCode > oldCode) { "Version $newCode is not newer than $oldCode" }

            val command = "cp -f '${file.path}' $STAGED_APK && chmod 644 $STAGED_APK && " +
                "pm install -r --user 0 $STAGED_APK; r=\$?; rm -f $STAGED_APK; exit \$r"
            Log.i(TAG, "Installing Manager ${info.versionName} ($newCode) over $oldCode")
            val ok = withNewRootShell { ShellUtils.fastCmdResult(this, command) }
            check(ok) { "pm install failed" }
        } finally {
            file.delete()
        }
    }.onFailure { Log.w(TAG, "Manager update not installed", it) }
}

private fun sha256Hex(input: InputStream): String {
    val digest = MessageDigest.getInstance("SHA-256")
    val buffer = ByteArray(64 * 1024)
    while (true) {
        val read = input.read(buffer)
        if (read < 0) break
        digest.update(buffer, 0, read)
    }
    return digest.digest().joinToString("") { "%02x".format(it) }
}

private fun sha256Hex(bytes: ByteArray): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
