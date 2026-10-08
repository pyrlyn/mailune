// Widget, share target, and mailto intent filter.
// Glance stays uncompiled: these are names and fixture subjects, not composables.

package app.mailune.ui

/** A home-screen widget, a share target, and a mailto intent filter. */
class Surfaces(
    val widgetName: String,
    val widgetSubject: String,
    val shareTarget: String,
    val shareSubject: String,
    val mailtoFilter: String,
    val mailtoSubject: String,
)

/** Reads the three surfaces from [text]. */
fun loadSurfaces(text: String): Surfaces {
    val entries = descriptionEntries(text)
    val widgetName = entries["widget"]?.singleOrNull()
    val widgetSubject = entries["widget-subject"]?.singleOrNull()
    val shareTarget = entries["share"]?.singleOrNull()
    val shareSubject = entries["share-subject"]?.singleOrNull()
    val mailtoFilter = entries["mailto"]?.singleOrNull()
    val mailtoSubject = entries["mailto-subject"]?.singleOrNull()
    if (
        widgetName.isNullOrEmpty() ||
        widgetSubject.isNullOrEmpty() ||
        shareTarget.isNullOrEmpty() ||
        shareSubject.isNullOrEmpty() ||
        mailtoFilter.isNullOrEmpty() ||
        mailtoSubject.isNullOrEmpty()
    ) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return Surfaces(
        widgetName,
        widgetSubject,
        shareTarget,
        shareSubject,
        mailtoFilter,
        mailtoSubject,
    )
}
