package app.mailune.ui

/** The reader body and the two switches that stay closed. */
class Reader(
    val body: String,
    val javascript: Boolean,
    val remote: Boolean,
)

/** Reads the body and the safety flags from [text]. */
fun loadReader(text: String): Reader {
    val entries = descriptionEntries(text)
    val body = entries["body"]?.singleOrNull()
    val javascript = entries["javascript"]?.singleOrNull()
    val remote = entries["remote"]?.singleOrNull()
    if (body.isNullOrEmpty() || javascript == null || remote == null) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return Reader(body, javascript == "on", remote == "allowed")
}
