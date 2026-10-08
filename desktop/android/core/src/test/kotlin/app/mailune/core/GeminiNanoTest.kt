// JVM coverage for Gemini Nano availability and the fallback.

package app.mailune.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GeminiNanoTest {
    @Test
    fun nanoIsUsedWhenItIsAvailable() {
        val model = GeminiNano(available = true)
        assertTrue(model.isAvailable())
        val completion = model.complete("See you at the dock")
        assertEquals("gemini-nano", completion.provider)
        assertEquals("See you at the dock", completion.text)
    }

    @Test
    fun aMissingNanoFallsBack() {
        val model = GeminiNano(available = false)
        assertFalse(model.isAvailable())
        val completion = model.complete("See you at the dock")
        assertEquals("fallback", completion.provider)
        assertEquals("See you at the dock", completion.text)
        assertFalse(model.toString().contains("See you at the dock"))
    }
}
