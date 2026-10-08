package app.mailune.ui

/** One row in the thread list. */
class ThreadRow(val subject: String)

/** Reads the fixture rows from [text]. */
fun loadThreadList(text: String): List<ThreadRow> {
    val subjects = descriptionEntries(text)["subject"] ?: emptyList()
    if (subjects.isEmpty()) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return subjects.map { ThreadRow(it) }
}
