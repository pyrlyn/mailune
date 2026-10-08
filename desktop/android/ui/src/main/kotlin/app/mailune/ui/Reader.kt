package app.mailune.ui

/**
 * The reader body, the two switches that stay closed, a fixture summary,
 * and one reply chip. No model is downloaded to produce them.
 */
class Reader(
    val body: String,
    val javascript: Boolean,
    val remote: Boolean,
    val summary: String,
    val replyChip: String,
)

/** Reads the body and the safety flags from [text]. */
fun loadReader(text: String): Reader {
    val entries = descriptionEntries(text)
    val body = entries["body"]?.singleOrNull()
    val javascript = entries["javascript"]?.singleOrNull()
    val remote = entries["remote"]?.singleOrNull()
    val summary = entries["summary"]?.singleOrNull()
    val replyChip = entries["reply-chip"]?.singleOrNull()
    if (body.isNullOrEmpty() || javascript == null || remote == null || summary.isNullOrEmpty() || replyChip.isNullOrEmpty()) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return Reader(body, javascript == "on", remote == "allowed", summary, replyChip)
}
