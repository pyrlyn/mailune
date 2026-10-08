package app.mailune.ui

/** List and detail panes, plus the two widths the shell switches on. */
class Shell(
    val panes: List<String>,
    val phoneWidth: Int,
    val tabletWidth: Int,
)

/** Reads the panes and widths from [text]. */
fun loadShell(text: String): Shell {
    val entries = descriptionEntries(text)
    val panes = entries["pane"] ?: emptyList()
    val phone = entries["phone"]?.singleOrNull()?.toIntOrNull()
    val tablet = entries["tablet"]?.singleOrNull()?.toIntOrNull()
    if (panes != listOf("list", "detail") || phone == null || tablet == null) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return Shell(panes, phone, tablet)
}
