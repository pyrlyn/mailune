package app.mailune.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ThreadListTest {
    @Test
    fun theListHasOneFixtureSubject() {
        val rows = loadThreadList(readDescription("thread-list.desc"))
        assertEquals(listOf("See you at the dock"), rows.map { it.subject })
    }
}
