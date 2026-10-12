package me.weishu.kernelsu.ui.util

import android.net.Uri
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.ksuApp
import me.weishu.kernelsu.ui.util.module.LatestVersionInfo
import okhttp3.Request

/**
 * @author weishu
 * @date 2023/6/22.
 */
suspend fun download(
    url: String,
    fileName: String,
    onDownloaded: (Uri) -> Unit = {},
    onDownloading: () -> Unit = {},
    onProgress: (Int) -> Unit = {}
) {
    onDownloading()

    val downloadId = DownloadManager.enqueue(
        context = ksuApp,
        url = url,
        fileName = fileName,
        onCompleted = onDownloaded,
    )

    DownloadManager.downloads
        .onEach { map -> map[downloadId]?.let { onProgress(it.progress) } }
        .first { map ->
            val status = map[downloadId]?.status
            status == DownloadManager.Status.COMPLETED ||
                status == DownloadManager.Status.FAILED
        }
}

internal suspend fun isDownloadAvailable(uri: Uri): Boolean = withContext(Dispatchers.IO) {
    runCatching {
        ksuApp.contentResolver.openFileDescriptor(uri, "r").use { it != null }
    }.getOrDefault(false)
}

private const val GXR_LATEST_RELEASE =
    "https://api.github.com/repos/4uva4ek/KernelSU-GXR/releases/latest"

/**
 * Latest published GXR Manager release. Releases ship `manager-update.json`
 * (versionCode, versionName, apk, sha256, certSha256); older releases are read
 * from `BUILD-INFO.txt` and `SHA256SUMS.txt`. Drafts and prereleases are ignored.
 */
fun checkNewVersion(): LatestVersionInfo {
    val none = LatestVersionInfo()
    if (!isNetworkAvailable(ksuApp)) return none
    return runCatching {
        val release = org.json.JSONObject(fetchText(GXR_LATEST_RELEASE) ?: return none)
        if (release.optBoolean("draft") || release.optBoolean("prerelease")) return none
        val changelog = release.optString("body")
        val assets = release.getJSONArray("assets")
        val urls = (0 until assets.length()).map { assets.getJSONObject(it) }
            .associate { it.getString("name") to it.getString("browser_download_url") }

        var versionCode = 0
        var versionName = ""
        var apk = ""
        var sha256 = ""
        var cert = ""
        val manifest = urls["manager-update.json"]?.let { fetchText(it) }?.let { org.json.JSONObject(it) }
        if (manifest != null) {
            versionCode = manifest.optInt("versionCode")
            versionName = manifest.optString("versionName")
            apk = manifest.optString("apk")
            sha256 = manifest.optString("sha256")
            cert = manifest.optString("certSha256")
        } else {
            val info = urls["BUILD-INFO.txt"]?.let { fetchText(it) } ?: ""
            Regex("""Manager:\s*(\S+)\s*\((\d+)\)""").find(info)?.let {
                versionName = it.groupValues[1]
                versionCode = it.groupValues[2].toInt()
            }
            Regex("""Certificate SHA256:\s*([0-9a-fA-F]{64})""").find(info)?.let { cert = it.groupValues[1] }
            apk = urls.keys.firstOrNull { it.endsWith(".apk") } ?: ""
            val sums = urls["SHA256SUMS.txt"]?.let { fetchText(it) } ?: ""
            sha256 = sums.lines().map { it.trim() }
                .firstOrNull { it.endsWith("  $apk") }?.substringBefore(' ') ?: ""
        }
        val downloadUrl = urls[apk] ?: return none
        if (versionCode <= 0) return none
        LatestVersionInfo(versionCode, downloadUrl, changelog, versionName, apk, sha256, cert)
    }.getOrDefault(none)
}

private fun fetchText(url: String): String? =
    ksuApp.okhttpClient.newCall(Request.Builder().url(url).build()).execute().use { response ->
        if (response.isSuccessful) response.body.string() else null
    }
