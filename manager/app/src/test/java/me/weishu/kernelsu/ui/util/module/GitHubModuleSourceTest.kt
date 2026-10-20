package me.weishu.kernelsu.ui.util.module

import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

class GitHubModuleSourceTest {
    @Test fun repositoryLinksAreCanonicalAndBounded() {
        assertEquals("owner/repo", GitHubModuleSource.repository("https://github.com/Owner/Repo.git/"))
        assertEquals("owner/repo", GitHubModuleSource.repository("owner/repo"))
        for (url in listOf("https://evil.test/o/r", "https://github.com/o/r/tree/main", "o/../r", "https://user@github.com/o/r", "https://github.com/o/r?x=1")) {
            assertThrows(IllegalArgumentException::class.java) { GitHubModuleSource.repository(url) }
        }
    }

    private val catalogue = """{"schemaVersion":1,"modules":[{"id":"test_module","name":"Test","updateJson":"https://example.com/update.json"}]}"""
    private val feed = """{"version":"1.2.3","versionCode":123,"zipUrl":"https://example.com/module.zip","sha256":"${"a".repeat(64)}"}"""

    @Test fun experimentalIsExplicitAndDefaultsToFalse() {
        assertFalse(GitHubModuleSource.parseCatalogue(catalogue) { feed }.single().experimental)
        for (flag in listOf(true, false)) {
            val text = catalogue.replace("\"name\":\"Test\"", "\"name\":\"Test\",\"experimental\":$flag")
            assertEquals(flag, GitHubModuleSource.parseCatalogue(text) { feed }.single().experimental)
        }
        for (invalid in listOf("\"true\"", "1", "null")) {
            val text = catalogue.replace("\"name\":\"Test\"", "\"name\":\"Test\",\"experimental\":$invalid")
            assertThrows(IllegalArgumentException::class.java) {
                GitHubModuleSource.parseCatalogue(text) { feed }
            }
        }
    }

    @Test fun catalogueUsesPublishedVersionAndRejectsUnsafeMetadata() {
        val modules = GitHubModuleSource.parseCatalogue(catalogue) { feed }
        assertEquals("test_module", modules.single().id)
        assertEquals(123, modules.single().versionCode)
        assertThrows(IllegalArgumentException::class.java) {
            GitHubModuleSource.parseCatalogue(catalogue.replace("https://", "file://")) { feed }
        }
        assertThrows(IllegalArgumentException::class.java) {
            GitHubModuleSource.parseCatalogue(catalogue) { feed.replace("https://", "http://") }
        }
        assertThrows(IllegalArgumentException::class.java) {
            GitHubModuleSource.parseCatalogue(catalogue) { feed.replace("123,", "0,") }
        }
        assertThrows(IllegalArgumentException::class.java) {
            GitHubModuleSource.parseCatalogue(catalogue.replace("schemaVersion\":1", "schemaVersion\":2")) { feed }
        }
    }

    @Test fun selectedZipMustMatchCatalogueBeforeAnyInstallation() {
        val file = File.createTempFile("gxr-source-test", ".zip")
        try {
            ZipOutputStream(file.outputStream()).use { zip ->
                zip.putNextEntry(ZipEntry("module.prop"))
                zip.write("id=test_module\nversionCode=123\n".toByteArray())
                zip.closeEntry()
            }
            val module = GitHubModuleSource.parseCatalogue(catalogue) { feed }.single()
            assertEquals("test_module" to 123, GitHubModuleSource.inspectPackage(file, module))
            assertThrows(IllegalArgumentException::class.java) {
                GitHubModuleSource.inspectPackage(file, module.copy(id = "wrong_module"))
            }
            assertThrows(IllegalArgumentException::class.java) {
                GitHubModuleSource.inspectPackage(file, module.copy(versionCode = 124))
            }
        } finally { file.delete() }
    }

    @Test fun repositoryCanSupplyMoreThanFivePreviouslyUnknownModules() {
        val entries = (1..12).joinToString(",") { index ->
            """{"id":"gxr_external_$index","name":"External $index","updateJson":"https://example.com/$index.json"}"""
        }
        val text = """{"schemaVersion":1,"modules":[$entries]}"""
        val fetched = mutableListOf<String>()
        val modules = GitHubModuleSource.parseCatalogue(text) { url -> fetched += url; feed }
        assertEquals(12, modules.size)
        assertEquals((1..12).map { "gxr_external_$it" }, modules.map { it.id })
        assertEquals((1..12).map { "https://example.com/$it.json" }, fetched)
        assertEquals("https://example.com/12.json", GitHubModuleSource.catalogueFeeds(text)["gxr_external_12"])
    }

    @Test fun legacyUpdateLookupUsesSourcesAndRejectsConflictingOwnership() {
        val feeds = ModuleSourceRegistry.uniqueFeeds(listOf(
            mapOf("gxr_new_module" to "https://a.test/new.json", "other" to "https://a.test/other.json"),
            mapOf("gxr_second" to "https://b.test/new.json", "other" to "https://b.test/other.json"),
        ))
        assertEquals("https://a.test/new.json", feeds["gxr_new_module"])
        assertEquals("https://b.test/new.json", feeds["gxr_second"])
        assertNull(feeds["other"])
    }
}
