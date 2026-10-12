package me.weishu.kernelsu.ui.util.module

data class LatestVersionInfo(
    val versionCode: Int = 0,
    val downloadUrl: String = "",
    val changelog: String = "",
    val versionName: String = "",
    val apkName: String = "",
    val sha256: String = "",
    val certSha256: String = "",
)
