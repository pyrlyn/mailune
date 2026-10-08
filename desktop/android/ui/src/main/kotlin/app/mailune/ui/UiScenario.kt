// UI scenario. The emulator is absent, so this only names a fixture subject.
// Screenshot capture stays off.

package app.mailune.ui

/** A scenario the UI would walk if an emulator were present. */
class UiScenario(
    val subject: String,
    val screenshots: Boolean,
)

/** Reads the fixture subject and the screenshot switch from [text]. */
fun loadUiScenario(text: String): UiScenario {
    val entries = descriptionEntries(text)
    val subject = entries["subject"]?.singleOrNull()
    val screenshots = entries["screenshots"]?.singleOrNull()
    if (subject.isNullOrEmpty() || screenshots == null) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return UiScenario(subject, screenshots == "on")
}
