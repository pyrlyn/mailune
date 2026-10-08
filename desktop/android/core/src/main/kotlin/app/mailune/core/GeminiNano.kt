// Gemini Nano availability. The Android SDK is absent, so this only records
// whether the caller says the model is present and falls back when it is not.

package app.mailune.core

/** Text from either Gemini Nano or the fallback. */
class Completion(
    val provider: String,
    val text: String,
)

/**
 * Platform model. [available] is supplied by the caller because the SDK is absent.
 * A missing model uses the fallback and does not download anything.
 */
class GeminiNano(
    private val available: Boolean,
) {
    /** Whether Gemini Nano can take the prompt. */
    fun isAvailable(): Boolean = available

    /** Uses Nano when it is present. Otherwise returns the fallback text. */
    fun complete(prompt: String): Completion {
        if (prompt.isEmpty()) {
            throw IllegalArgumentException("empty prompt")
        }
        return if (available) {
            Completion(provider = "gemini-nano", text = prompt)
        } else {
            Completion(provider = "fallback", text = prompt)
        }
    }

    override fun toString(): String = "GeminiNano"
}
