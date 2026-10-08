package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ComposerTest {
    @Test
    fun sendStaysOffUntilConfirm() {
        val composer = loadComposer(readDescription("composer.desc"))
        assertEquals("ada@example.com", composer.recipient)
        assertEquals("Hello", composer.subject)
        assertEquals("See you at the dock", composer.body)
        assertFalse(composer.send)
        assertTrue(composer.confirm)
    }
}
