package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ShellTest {
    @Test
    fun theShellNamesListDetailPhoneAndTablet() {
        val shell = loadShell(readDescription("shell.desc"))
        assertEquals(listOf("list", "detail"), shell.panes)
        assertEquals(600, shell.phoneWidth)
        assertEquals(840, shell.tabletWidth)
    }
}
