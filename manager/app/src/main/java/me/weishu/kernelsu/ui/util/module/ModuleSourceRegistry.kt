package me.weishu.kernelsu.ui.util.module

import android.content.Context
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.ksuApp
import org.json.JSONArray

/** Installed packages own their updateJson; older packages use configured Git catalogues. */
object ModuleSourceRegistry {
    @Volatile private var feeds: Map<String, String> = emptyMap()

    fun feed(id: String): String? = feeds[id]

    suspend fun refresh() = withContext(Dispatchers.IO) {
        val prefs = ksuApp.getSharedPreferences("module-sources", Context.MODE_PRIVATE)
        val repositories = if (!prefs.contains("repositories")) listOf(GitHubModuleSource.DEFAULT) else {
            runCatching {
                val saved = JSONArray(prefs.getString("repositories", "[]"))
                (0 until saved.length()).map { GitHubModuleSource.repository(saved.getString(it)) }.distinct()
            }.getOrDefault(emptyList())
        }
        val catalogues = repositories.mapNotNull { repository ->
            try {
                GitHubModuleSource.catalogue(repository)?.let(GitHubModuleSource::catalogueFeeds)
            } catch (error: Exception) {
                if (error is CancellationException) throw error
                null // An unavailable source must not suppress other sources/package-owned feeds.
            }
        }
        feeds = uniqueFeeds(catalogues)
    }

    internal fun uniqueFeeds(catalogues: List<Map<String, String>>): Map<String, String> =
        catalogues.flatMap { it.entries }.groupBy({ it.key }, { it.value })
            .mapNotNull { (id, urls) -> urls.distinct().singleOrNull()?.let { id to it } }.toMap()
}
