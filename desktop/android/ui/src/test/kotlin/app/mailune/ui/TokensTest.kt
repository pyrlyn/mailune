package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class TokensTest {
    @Test
    fun coloursTypeSpacingAndALocalIconName() {
        assertEquals("window_bg", Tokens.windowBackground)
        assertEquals("accent", Tokens.accent)
        assertEquals("foreground", Tokens.foreground)
        assertEquals("sans", Tokens.fontFamily)
        assertEquals(16, Tokens.fontSizeSp)
        assertEquals(16, Tokens.spaceDp)
        assertEquals("mail", Tokens.iconName)
        assertFalse(Tokens.iconName.contains("http"))
    }
}
