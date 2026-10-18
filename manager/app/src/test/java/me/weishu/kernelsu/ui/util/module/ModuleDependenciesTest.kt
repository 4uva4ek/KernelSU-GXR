package me.weishu.kernelsu.ui.util.module

import org.junit.Assert.*
import org.junit.Test

class ModuleDependenciesTest {
    @Test fun requirementsComeFromPackageMetadataAndKeepOrder() {
        val props = mapOf("id" to "gxr_new", "gxrReplaces" to "gxr_dependent,gxr_provider", "gxrRequiresDisabled" to "gxr_optional")
        assertEquals(listOf("gxr_dependent", "gxr_provider"), ModuleDependencies.ids(props, "gxrReplaces"))
        assertEquals(listOf("gxr_optional"), ModuleDependencies.ids(props, "gxrRequiresDisabled"))
        assertEquals(emptyList<String>(), ModuleDependencies.ids(mapOf("id" to "gxr_other"), "gxrReplaces"))
    }

    @Test fun invalidDependencyMetadataCannotBecomeShellArguments() {
        for (value in listOf("", "gxr_new", "gxr_a,gxr_a", "../gxr_a", "gxr_a;id", "gxr_a, gxr_b", "gxr_a\nfoo")) {
            assertThrows(IllegalArgumentException::class.java) {
                ModuleDependencies.ids(mapOf("id" to "gxr_new", "gxrReplaces" to value), "gxrReplaces")
            }
        }
    }
}
