package me.weishu.kernelsu.ui.viewmodel

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.ksuApp
import me.weishu.kernelsu.ui.util.module.GitHubModuleSource
import me.weishu.kernelsu.ui.util.module.SourcePackage
import okhttp3.Request
import org.json.JSONArray
import java.io.File
import java.security.MessageDigest

data class ModuleSourcesState(
    val sources: List<String> = emptyList(),
    val active: String = GitHubModuleSource.DEFAULT,
    val packages: List<SourcePackage> = emptyList(),
    val selected: Set<String> = emptySet(),
    val busy: Boolean = false,
    val progress: String? = null,
    val error: String? = null,
)

class ModuleSourcesViewModel : ViewModel() {
    private val prefs = ksuApp.getSharedPreferences("module-sources", Context.MODE_PRIVATE)
    private val stored = runCatching {
        val array = JSONArray(prefs.getString("repositories", "[]"))
        (0 until array.length()).map { GitHubModuleSource.repository(array.getString(it)) }.distinct()
    }.getOrDefault(emptyList())
    private val sources = if (prefs.contains("repositories")) stored else listOf(GitHubModuleSource.DEFAULT)
    private val _state = MutableStateFlow(ModuleSourcesState(sources = sources,
        active = prefs.getString("active", null)?.takeIf { it == "official" || it in sources } ?: sources.firstOrNull() ?: "official"))
    val state = _state.asStateFlow()
    private val _install = Channel<List<Uri>>(Channel.BUFFERED)
    val install = _install.receiveAsFlow()

    private fun save() {
        prefs.edit().putString("repositories", JSONArray(_state.value.sources).toString())
            .putString("active", _state.value.active).apply()
    }

    fun select(source: String) {
        if (_state.value.busy) return
        _state.update { it.copy(active = source, packages = emptyList(), selected = emptySet(), error = null) }
        save()
        refresh()
    }

    fun add(input: String): Boolean {
        if (_state.value.busy) return false
        return runCatching {
            val source = GitHubModuleSource.repository(input)
            _state.update { it.copy(sources = (it.sources + source).distinct()) }
            select(source)
            true
        }.getOrElse { error -> _state.update { it.copy(error = error.message) }; false }
    }

    fun remove(source: String) {
        if (_state.value.busy) return
        _state.update { it.copy(sources = it.sources - source) }
        if (_state.value.active == source) select(_state.value.sources.firstOrNull() ?: "official")
        save()
    }

    fun refresh() {
        val source = _state.value.active
        if (_state.value.busy || source == "official") return
        _state.update { it.copy(busy = true, error = null, progress = "Loading $source") }
        viewModelScope.launch {
            try {
                val packages = withContext(Dispatchers.IO) { GitHubModuleSource.load(source) }
                _state.update { it.copy(packages = packages, selected = emptySet()) }
            } catch (error: Exception) {
                if (error is CancellationException) throw error
                _state.update { it.copy(error = error.message ?: "Cannot load source", packages = emptyList(), selected = emptySet()) }
            } finally {
                _state.update { it.copy(busy = false, progress = null) }
            }
        }
    }

    fun toggle(key: String) {
        if (_state.value.busy) return
        _state.update { it.copy(selected = if (key in it.selected) it.selected - key else it.selected + key) }
    }

    fun installSelected(installed: Map<String, Int>) {
        val state = _state.value
        if (state.busy) return
        val packages = state.packages.filter { it.key in state.selected }
        if (packages.isEmpty()) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            val files = mutableListOf<File>()
            try {
                withContext(Dispatchers.IO) {
                    val ids = mutableSetOf<String>()
                    for ((index, item) in packages.withIndex()) {
                        _state.update { it.copy(progress = "Downloading ${index + 1}/${packages.size}: ${item.name}") }
                        val file = File.createTempFile("source-module-", ".zip", ksuApp.cacheDir)
                        files += file
                        val digest = MessageDigest.getInstance("SHA-256")
                        ksuApp.okhttpClient.newCall(Request.Builder().url(item.zipUrl).build()).execute().use { response ->
                            check(response.isSuccessful) { "Download failed: HTTP ${response.code}" }
                            response.body.byteStream().use { input -> file.outputStream().use { output ->
                                val buffer = ByteArray(64 * 1024)
                                var total = 0L
                                while (true) {
                                    val n = input.read(buffer)
                                    if (n < 0) break
                                    total += n
                                    require(total <= 128 * 1024 * 1024) { "Package exceeds 128 MiB" }
                                    output.write(buffer, 0, n)
                                    digest.update(buffer, 0, n)
                                }
                            } }
                        }
                        val hash = digest.digest().joinToString("") { "%02x".format(it) }
                        require(item.sha256 == null || hash == item.sha256) { "SHA-256 mismatch: ${item.name}" }
                        val (id, code) = GitHubModuleSource.inspectPackage(file, item)
                        require(ids.add(id)) { "Two selected packages contain the same module: $id" }
                        require(installed[id]?.let { code > it } != false) { "$id: this version or a newer version is already installed" }
                    }
                }
                _state.update { it.copy(selected = emptySet()) }
                _install.send(files.map(Uri::fromFile))
                // Flash screen receives app-private file URIs. The cache files
                // remain available across navigation/recreation; Android may
                // clear the cache later. No module is installed during download.
            } catch (error: Exception) {
                files.forEach { it.delete() }
                if (error is CancellationException) throw error
                _state.update { it.copy(error = error.message ?: "Download verification failed") }
            } finally {
                _state.update { it.copy(busy = false, progress = null) }
            }
        }
    }
}
