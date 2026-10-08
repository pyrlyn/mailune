package app.mailune.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class AndroidHostTest {
    @Test
    fun aSecretStaysInMemory() {
        val host = AndroidHost()
        host.putSecret("ada", "token")
        assertEquals("token", host.secret("ada"))
        assertFalse(host.toString().contains("token"))
    }

    @Test
    fun aSyncRequestRecordsTheAccount() {
        val host = AndroidHost()
        host.requestSync("ada")
        assertEquals(listOf("ada"), host.syncRequests())
    }

    @Test
    fun theNotificationChannelIsNamed() {
        assertEquals("mailune.sync", AndroidHost().notificationChannel())
    }

    @Test
    fun theOauthRedirectCarriesTheCode() {
        assertEquals("mailune://callback?code=abc", AndroidHost().oauthRedirect("abc"))
    }
}
