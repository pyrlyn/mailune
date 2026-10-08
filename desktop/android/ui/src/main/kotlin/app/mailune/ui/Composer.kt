package app.mailune.ui

/** A draft. Send stays closed until confirm is required. */
class Composer(
    val recipient: String,
    val subject: String,
    val body: String,
    val send: Boolean,
    val confirm: Boolean,
)

/** Reads the draft fields and the send gate from [text]. */
fun loadComposer(text: String): Composer {
    val entries = descriptionEntries(text)
    val recipient = entries["recipient"]?.singleOrNull()
    val subject = entries["subject"]?.singleOrNull()
    val body = entries["body"]?.singleOrNull()
    val send = entries["send"]?.singleOrNull()
    val confirm = entries["confirm"]?.singleOrNull()
    if (recipient.isNullOrEmpty() || subject.isNullOrEmpty() || body.isNullOrEmpty() || send == null || confirm == null) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    val sendOn = send == "on"
    val confirmRequired = confirm == "required"
    if (sendOn && !confirmRequired) {
        throw IllegalArgumentException("ui description is incomplete")
    }
    return Composer(recipient, subject, body, sendOn, confirmRequired)
}
