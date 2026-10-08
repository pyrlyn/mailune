// Reads the fixture subject. Screenshot capture stays off.

package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class UiScenarioTest {
    @Test
    fun theScenarioNamesAFixtureSubject() {
        val scenario = loadUiScenario(readDescription("ui-scenario.desc"))
        assertEquals("See you at the dock", scenario.subject)
        assertFalse(scenario.screenshots)
    }
}
