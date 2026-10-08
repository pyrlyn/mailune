package app.mailune.core

import org.junit.Assert.assertEquals
import org.junit.Test

class CoreRecordTest {
    @Test
    fun roundTripsOneRecord() {
        val record = CoreRecord(ok = true, message = "ready")
        val again = CoreRecord.decode(record.encode())
        assertEquals(record, again)
    }
}
