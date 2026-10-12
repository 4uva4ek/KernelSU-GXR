package me.weishu.kernelsu.ui.util.module

import me.weishu.kernelsu.ksuApp
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject
import java.net.URI
import java.io.File
import java.util.zip.ZipFile

data class SourcePackage(
    val key: String,
    val id: String?,
    val name: String,
    val version: String,
    val versionCode: Int?,
    val description: String,
    val zipUrl: String,
    val sha256: String?,
)

object GitHubModuleSource {
    const val DEFAULT = "4uva4ek/galaxyxr-modules"

    fun repository(input: String): String {
        val text = input.trim().removeSuffix("/").removeSuffix(".git")
        val path = if (text.startsWith("https://", true)) {
            val uri = URI(text)
            require(uri.host.equals("github.com", true) && uri.userInfo == null && uri.port == -1 &&
                uri.query == null && uri.fragment == null) { "Paste a public GitHub repository URL" }
            uri.path.removePrefix("/")
        } else text
        require(Regex("[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+").matches(path) &&
            path.split('/').none { it == "." || it == ".." }) { "Use https://github.com/owner/repository" }
        return path.lowercase(java.util.Locale.ROOT)
    }

    private fun https(value: String): String {
        val uri = URI(value)
        require(uri.scheme == "https" && !uri.host.isNullOrBlank() && uri.userInfo == null) { "Source contains a non-HTTPS URL" }
        return value
    }

    private fun json(url: String, allowMissing: Boolean = false): String? {
        val request = Request.Builder().url(https(url)).header("Accept", "application/json").build()
        return ksuApp.okhttpClient.newCall(request).execute().use { response ->
            if (allowMissing && response.code == 404) return@use null
            check(response.isSuccessful) {
                if (response.code == 403 || response.code == 429) "GitHub request limit reached; retry later"
                else "Source returned HTTP ${response.code}"
            }
            val bytes = response.body.byteStream().use { it.readNBytes(2 * 1024 * 1024 + 1) }
            require(bytes.size <= 2 * 1024 * 1024) { "Source catalogue is too large" }
            bytes.toString(Charsets.UTF_8)
        }
    }

    fun catalogue(repo: String): String? {
        val safeRepo = repository(repo)
        val info = JSONObject(requireNotNull(json("https://api.github.com/repos/$safeRepo")))
        val branch = info.getString("default_branch")
        require(Regex("[A-Za-z0-9_./-]+").matches(branch) && !branch.contains("..")) { "Unsupported default branch name" }
        return json("https://raw.githubusercontent.com/$safeRepo/$branch/modules.json", true)
    }

    fun load(repo: String): List<SourcePackage> {
        val safeRepo = repository(repo)
        val catalogue = catalogue(safeRepo)
        if (catalogue != null) return parseCatalogue(catalogue) { requireNotNull(json(it)) }
        // Repositories without a catalogue expose their stable release ZIPs.
        // A module/vX tag defines an independent release line, as in GXR.
        val releases = JSONArray()
        var page = 1
        do {
            val batch = JSONArray(requireNotNull(json("https://api.github.com/repos/$safeRepo/releases?per_page=100&page=$page")))
            for (index in 0 until batch.length()) releases.put(batch.getJSONObject(index))
            page++
        } while (batch.length() == 100)
        val lines = mutableSetOf<String>()
        val packages = mutableListOf<SourcePackage>()
        for (i in 0 until releases.length()) {
            val release = releases.getJSONObject(i)
            if (release.optBoolean("draft") || release.optBoolean("prerelease")) continue
            val tag = release.getString("tag_name")
            val line = if (tag.contains("/v")) tag.substringBeforeLast("/v") else "default"
            if (line in lines) continue
            val assets = release.getJSONArray("assets")
            val found = mutableListOf<SourcePackage>()
            for (a in 0 until assets.length()) {
                val asset = assets.getJSONObject(a)
                val name = asset.getString("name")
                if (!name.endsWith(".zip", true)) continue
                val url = https(asset.getString("browser_download_url"))
                val hash = asset.optString("digest").removePrefix("sha256:")
                    .takeIf { Regex("[0-9a-f]{64}").matches(it) }
                found += SourcePackage(url, null, name, tag, null,
                    "Release package. Module identity is checked before installation.", url, hash)
            }
            if (found.isNotEmpty()) { lines += line; packages += found }
        }
        return packages
    }

    internal fun parseCatalogue(text: String, fetch: (String) -> String): List<SourcePackage> {
        val feeds = catalogueFeeds(text)
        val root = JSONObject(text)
        require(root.getInt("schemaVersion") == 1) { "Unsupported catalogue version" }
        val modules = root.getJSONArray("modules")
        val ids = mutableSetOf<String>()
        return (0 until modules.length()).map { index ->
            val module = modules.getJSONObject(index)
            val id = module.getString("id")
            require(Regex("[a-zA-Z][a-zA-Z0-9._-]+").matches(id) && ids.add(id)) { "Invalid or duplicate module ID" }
            val update = JSONObject(fetch(feeds.getValue(id)))
            val code = update.getInt("versionCode")
            require(code > 0) { "Invalid module versionCode" }
            val hash = update.optString("sha256").takeIf { it.isNotEmpty() }
            require(hash == null || Regex("[0-9a-f]{64}").matches(hash)) { "Invalid SHA-256" }
            SourcePackage(id, id, module.getString("name"), update.getString("version"), code,
                module.optString("description"), https(update.getString("zipUrl")), hash)
        }
    }

    internal fun catalogueFeeds(text: String): Map<String, String> {
        val root = JSONObject(text)
        require(root.getInt("schemaVersion") == 1) { "Unsupported catalogue version" }
        val modules = root.getJSONArray("modules")
        val feeds = linkedMapOf<String, String>()
        for (index in 0 until modules.length()) {
            val module = modules.getJSONObject(index)
            val id = module.getString("id")
            require(Regex("[a-zA-Z][a-zA-Z0-9._-]+").matches(id) && id !in feeds) { "Invalid or duplicate module ID" }
            feeds[id] = https(module.getString("updateJson"))
        }
        return feeds
    }

    internal fun inspectPackage(file: File, item: SourcePackage): Pair<String, Int> {
        val properties = ZipFile(file).use { zip ->
            require(zip.entries().asSequence().count { it.name == "module.prop" } == 1) { "ZIP must contain one module.prop" }
            val entry = zip.getEntry("module.prop") ?: error("Not a KernelSU module ZIP")
            require(entry.size in 1..16384) { "Invalid module.prop size" }
            zip.getInputStream(entry).bufferedReader().use { reader ->
                val props = mutableMapOf<String, String>()
                reader.readText().lineSequence().filter { '=' in it }.forEach { line ->
                    val pair = line.split('=', limit = 2)
                    require(props.put(pair[0], pair[1]) == null) { "Duplicate module property" }
                }
                props
            }
        }
        val id = properties["id"] ?: error("Module ID missing")
        val code = properties["versionCode"]?.toIntOrNull() ?: error("Module versionCode missing")
        require(Regex("[a-zA-Z][a-zA-Z0-9._-]+").matches(id) && code > 0) { "Invalid module identity/version" }
        require(item.id == null || item.id == id) { "Downloaded module ID differs from catalogue" }
        require(item.versionCode == null || item.versionCode == code) { "Downloaded version differs from catalogue" }
        return id to code
    }
}
