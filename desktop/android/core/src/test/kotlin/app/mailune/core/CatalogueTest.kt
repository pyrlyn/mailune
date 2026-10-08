package app.mailune.core

import org.junit.Assert.assertEquals
import org.junit.Test

class CatalogueTest {
    @Test
    fun aMissingKeyFallsBackToEnglish() {
        val german = loadCatalogue("de")
        assertEquals("Posteingang", german.text("inbox"))
        assertEquals("Compose", german.text("compose"))
        assertEquals("Inbox", loadCatalogue("en").text("inbox"))
    }
}
