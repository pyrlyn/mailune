// Reads one fixture subject from the widget, the share target, and mailto.

package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SurfacesTest {
    @Test
    fun eachSurfaceNamesOneFixtureSubject() {
        val surfaces = loadSurfaces(readDescription("surfaces.desc"))
        assertEquals("InboxWidget", surfaces.widgetName)
        assertEquals("See you at the dock", surfaces.widgetSubject)
        assertEquals("app.mailune.share.Send", surfaces.shareTarget)
        assertEquals("See you at the dock", surfaces.shareSubject)
        assertEquals("mailto", surfaces.mailtoFilter)
        assertEquals("See you at the dock", surfaces.mailtoSubject)
    }
}
