plugins {
    alias(libs.plugins.agp.app) apply false
    alias(libs.plugins.kotlin) apply false
    alias(libs.plugins.compose.compiler) apply false
}

extra["androidMinSdkVersion"] = 31
extra["androidTargetSdkVersion"] = 37
extra["androidCompileSdkVersion"] = 37
extra["androidCompileSdkVersionMinor"] = 0
extra["androidBuildToolsVersion"] = "37.0.0"
extra["androidCompileNdkVersion"] = libs.versions.ndk.get()
extra["androidSourceCompatibility"] = JavaVersion.VERSION_21
extra["androidTargetCompatibility"] = JavaVersion.VERSION_21
extra["managerVersionCode"] = providers.gradleProperty("KSU_VERSION_CODE").orNull?.toInt() ?: getVersionCode()
extra["managerVersionName"] = formatGxrVersion(providers.gradleProperty("KSU_VERSION_NAME").getOrElse(getVersionName()))

fun formatGxrVersion(version: String): String = when {
    version.startsWith("samsung-gxr-v") -> version
    version.startsWith("gxr-v") -> "samsung-$version"
    version.startsWith("gxr-") -> "samsung-gxr-v${version.removePrefix("gxr-")}"
    else -> "samsung-gxr-vdev-$version"
}

fun getGitCommitCount(): Int {
    val process = Runtime.getRuntime().exec(arrayOf("git", "rev-list", "--count", "HEAD"))
    return process.inputStream.bufferedReader().use { it.readText().trim().toInt() }
}

fun getGitDescribe(): String {
    val process = Runtime.getRuntime().exec(arrayOf("git", "describe", "--tags", "--match", "gxr-*", "--always"))
    val version = process.inputStream.bufferedReader().use { it.readText().trim() }
    check(process.waitFor() == 0 && version.isNotEmpty()) { "Cannot determine GXR version" }
    return if (version.startsWith("gxr-")) version else "gxr-dev-$version"
}

fun getVersionCode(): Int {
    val commitCount = getGitCommitCount()
    return 30000 + commitCount
}

fun getVersionName(): String {
    return getGitDescribe()
}
