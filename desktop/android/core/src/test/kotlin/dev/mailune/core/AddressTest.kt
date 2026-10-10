// Round-trips the `Address` record through the RustBuffer layout, the way a
// call into mailune-ffi and back would carry it.

package dev.mailune.core

import java.nio.ByteBuffer
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class AddressTest {
    @Test
    fun aNamedAddressRoundTrips() {
        val address = Address("Ada Lovelace", "ada@example.com")
        assertEquals(address, FfiConverterAddress.lift(FfiConverterAddress.lower(address)))
    }

    @Test
    fun aBareAddressRoundTripsWithANoneTag() {
        val address = Address(null, "ana@acme.io")
        val buf = FfiConverterAddress.lower(address)
        assertEquals(0, buf.get(0).toInt())
        assertEquals(address, FfiConverterAddress.lift(buf))
    }

    @Test
    fun theLayoutIsTheOneRustWrites() {
        val buf = FfiConverterAddress.lower(Address("É", "a@b"))
        val bytes = ByteArray(buf.remaining()).also { buf.get(it) }
        // Some tag, len 2 + "É" in UTF-8, len 3 + "a@b".
        val expected = byteArrayOf(1, 0, 0, 0, 2, 0xC3.toByte(), 0x89.toByte(), 0, 0, 0, 3, 0x61, 0x40, 0x62)
        assertContentEquals(expected, bytes)
    }

    @Test
    fun trailingBytesAreRejected() {
        val buf = ByteBuffer.allocate(FfiConverterAddress.allocationSize(Address(null, "a")) + 1)
        FfiConverterAddress.write(Address(null, "a"), buf)
        buf.flip()
        buf.limit(buf.limit() + 1)
        assertFailsWith<IllegalArgumentException> { FfiConverterAddress.lift(buf) }
    }
}
