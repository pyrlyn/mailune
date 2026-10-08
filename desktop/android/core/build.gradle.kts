// `core`: the Kotlin side of mailune-ffi. A plain JVM library until the
// Android SDK exists, so its tests run on any machine with a JDK.
plugins {
    kotlin("jvm") version "2.4.20"
}

group = "dev.mailune"
version = "0.1.0"

// Bytecode for Java 17, the level Android's D8 desugars, whatever JDK runs
// Gradle. Java and Kotlin must agree or the Kotlin plugin stops the build.
java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
}
