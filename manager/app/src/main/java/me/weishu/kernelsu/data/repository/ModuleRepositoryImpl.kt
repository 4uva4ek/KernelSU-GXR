package me.weishu.kernelsu.data.repository

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import me.weishu.kernelsu.data.model.Module
import me.weishu.kernelsu.data.model.ModuleUpdateInfo
import me.weishu.kernelsu.ksuApp
import me.weishu.kernelsu.ui.util.isNetworkAvailable
import me.weishu.kernelsu.ui.util.listModules
import me.weishu.kernelsu.ui.util.module.sanitizeVersionString
import me.weishu.kernelsu.ui.util.module.ModuleSourceRegistry
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject

class ModuleRepositoryImpl : ModuleRepository {

    companion object {
        private const val TAG = "ModuleRepository"
    }

    override suspend fun getModules(): Result<List<Module>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = listModules()
            val array = JSONArray(result)
            (0 until array.length())
                .asSequence()
                .map { array.getJSONObject(it) }
                .map { obj ->
                    Module(
                        id = obj.getString("id"),
                        name = obj.optString("name"),
                        author = obj.optString("author", "Unknown"),
                        version = obj.optString("version", "Unknown"),
                        versionCode = obj.optInt("versionCode", 0),
                        description = obj.optString("description"),
                        enabled = obj.getBoolean("enabled"),
                        update = obj.optBoolean("update"),
                        remove = obj.getBoolean("remove"),
                        updateJson = obj.optString("updateJson"),
                        hasWebUi = obj.optBoolean("web"),
                        hasActionScript = obj.optBoolean("action"),
                        metamodule = (obj.optInt("metamodule") != 0) || obj.optBoolean("metamodule"),
                        actionIconPath = obj.optString("actionIcon").takeIf { it.isNotBlank() },
                        webUiIconPath = obj.optString("webuiIcon").takeIf { it.isNotBlank() },
                        deprecatedBy = obj.optString("gxrDeprecatedBy").takeIf { Regex("gxr_[A-Za-z0-9_-]+").matches(it) },
                        replaces = obj.optString("gxrReplaces").split(',').filter { Regex("gxr_[A-Za-z0-9_-]+").matches(it) }.take(16),
                    )
                }.toList().let { modules ->
                    modules.map { module ->
                        val replacements = modules.filter { !it.remove && module.id in it.replaces }.map { it.id }
                        module.copy(deprecatedBy = module.deprecatedBy ?: replacements.singleOrNull())
                    }
                }
        }
    }

    override suspend fun checkUpdate(module: Module): Result<ModuleUpdateInfo> = withContext(Dispatchers.IO) {
        runCatching {
            if (!isNetworkAvailable(ksuApp)) {
                return@runCatching ModuleUpdateInfo.Empty
            }
            val feed = module.updateJson.takeIf { it.isNotBlank() } ?: ModuleSourceRegistry.feed(module.id)
            if (feed == null || module.remove || module.update) {
                return@runCatching ModuleUpdateInfo.Empty
            }

            val url = feed
            val response = ksuApp.okhttpClient.newCall(
                Request.Builder().url(url).build()
            ).execute()

            val result = if (response.isSuccessful) {
                response.body.string()
            } else {
                ""
            }

            if (result.isEmpty()) {
                return@runCatching ModuleUpdateInfo.Empty
            }

            val updateJson = JSONObject(result)
            val deprecatedBy = updateJson.optString("deprecatedBy").takeIf { Regex("gxr_[A-Za-z0-9_-]+").matches(it) }
            if (deprecatedBy != null) return@runCatching ModuleUpdateInfo.Empty.copy(deprecatedBy = deprecatedBy)
            var version = updateJson.optString("version", "")
            version = sanitizeVersionString(version)
            val versionCode = updateJson.optInt("versionCode", 0)
            val zipUrl = updateJson.optString("zipUrl", "")
            val changelog = updateJson.optString("changelog", "")

            if (versionCode <= module.versionCode || zipUrl.isEmpty()) {
                ModuleUpdateInfo.Empty
            } else {
                ModuleUpdateInfo(zipUrl, version, changelog)
            }
        }
    }
}
