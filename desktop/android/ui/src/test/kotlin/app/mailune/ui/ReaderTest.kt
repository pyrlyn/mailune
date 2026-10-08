package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class ReaderTest {
    @Test
    fun theReaderBlocksRemoteContentAndJavaScript() {
        val reader = loadReader(readDescription("reader.desc"))
        assertEquals("See you at the dock", reader.body)
        assertFalse(reader.javascript)
        assertFalse(reader.remote)
    }

    @Test
    fun theReaderIncludesASummaryAndOneReplyChip() {
        val reader = loadReader(readDescription("reader.desc"))
        assertEquals("A short note about the dock", reader.summary)
        assertEquals("Thanks", reader.replyChip)
    }
}
