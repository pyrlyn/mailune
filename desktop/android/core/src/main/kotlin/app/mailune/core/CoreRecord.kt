package app.mailune.core

/**
 * One core record the JVM test can encode and decode.
 *
 * UniFFI is not on this branch, so this is a plain Kotlin type.
 */
data class CoreRecord(
    val ok: Boolean,
    val message: String,
) {
    /** Line-oriented form the test reads back. */
    fun encode(): String = "ok $ok\nmessage $message\n"

    companion object {
        /** Rebuilds a record from [encode]. */
        fun decode(text: String): CoreRecord {
            var ok: Boolean? = null
            var message: String? = null
            for (line in text.lines()) {
                val trimmed = line.trim()
                if (trimmed.isEmpty()) {
                    continue
                }
                val space = trimmed.indexOf(' ')
                if (space <= 0) {
                    continue
                }
                val key = trimmed.substring(0, space)
                val value = trimmed.substring(space + 1)
                when (key) {
                    "ok" -> ok = value == "true"
                    "message" -> if (value.isNotEmpty()) message = value
                }
            }
            val ready = ok ?: throw IllegalArgumentException("record is incomplete")
            val textMessage = message ?: throw IllegalArgumentException("record is incomplete")
            return CoreRecord(ready, textMessage)
        }
    }
}
