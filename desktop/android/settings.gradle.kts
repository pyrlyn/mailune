// The Android shell's Gradle build. Only a JVM module today: the Android SDK
// and cargo-ndk arrive with D1, and this module's code moves under them.
pluginManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositories {
        mavenCentral()
    }
}

rootProject.name = "mailune-android"
include(":core")
