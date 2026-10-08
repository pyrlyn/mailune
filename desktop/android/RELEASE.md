# Android release

The modules under `desktop/android` stay JVM projects. `settings.gradle.kts`
includes `core`, `ui`, and `app`. `gradle/kotlin-jvm.gradle.kts` compiles
Kotlin with the jars inside the Gradle install, because the Kotlin Gradle
plugin marker is not cached. Gradle stays offline (`org.gradle.offline=true`).
The Android Gradle plugin is not on that cache, and this note does not
install the Android SDK.

## F-Droid

Checked 2026-10-08 against the inclusion policy:
<https://f-droid.org/docs/Inclusion_Policy/>.

F-Droid requires a FLOSS license, for example GPL, and compiles the
application from public source with a FLOSS toolchain. Proprietary tracking
libraries, including Google Play Services, are forbidden.

Mailune is `GPL-3.0-or-later OR LicenseRef-Mailune-Royalty-Free`. A store
build can ship as the `GPL-3.0-or-later` side. That is the kind of license
the policy names. The client is local-first, so a build can omit Play
services.

F-Droid's builders compile the binary. This tree is still the JVM modules
above, so it has no Android application plugin and no APK. `aab.gradle.kts`
describes the Play app bundle and is not applied. The script has no upload
task.
