// Not applied. settings.gradle.kts does not include this file, and no build
// script calls apply() on it. The Android Gradle plugin is not in the offline
// cache, and the Android SDK is not installed. Applying this file would ask
// Gradle to resolve that plugin. The JVM modules stay as they are.

// Future :app module, once the plugin and the SDK are present:
//
// plugins {
//     id("com.android.application")
// }
//
// android {
//     namespace = "app.mailune"
//     compileSdk = 36
//     defaultConfig {
//         applicationId = "app.mailune"
//         minSdk = 26
//         targetSdk = 36
//     }
//     buildTypes {
//         release {
//             isMinifyEnabled = false
//         }
//     }
// }
//
// :app:bundleRelease writes
// app/build/outputs/bundle/release/app-release.aab
// There is no Play upload task.

val mailuneAab = mapOf(
    "plugin" to "com.android.application",
    "namespace" to "app.mailune",
    "applicationId" to "app.mailune",
    "minSdk" to "26",
    "compileSdk" to "36",
    "targetSdk" to "36",
    "task" to ":app:bundleRelease",
    "artifact" to "app/build/outputs/bundle/release/app-release.aab",
    "upload" to "none",
)
