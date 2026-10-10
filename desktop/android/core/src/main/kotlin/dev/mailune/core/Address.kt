// The `Address` record from mailune-ffi, with the converter UniFFI generates
// for it. uniffi-bindgen does not run here yet (no cargo-ndk, no Android
// SDK), so this file is written by hand in the generated shape. Once the
// bindings are generated, they replace it and the test keeps passing.

package dev.mailune.core

import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets

/** Someone a message is from, to or copied to. */
data class Address(
    /** Display name when the header had one. */
    var name: String?,
    /** Addr-spec (`ana@acme.io`). */
    var email: String,
)

/**
 * Reads and writes [Address] in the RustBuffer layout: fields in declaration
 * order, a string as a big-endian i32 byte length then UTF-8, an optional as
 * a 0 or 1 tag byte then the value.
 */
object FfiConverterAddress {
    fun read(buf: ByteBuffer): Address = Address(readOptionalString(buf), readString(buf))

    fun allocationSize(value: Address): Int =
        1 + (value.name?.let { 4 + utf8(it).size } ?: 0) + 4 + utf8(value.email).size

    fun write(value: Address, buf: ByteBuffer) {
        writeOptionalString(value.name, buf)
        writeString(value.email, buf)
    }

    /** One record in a fresh buffer, as a call into Rust would carry it. */
    fun lower(value: Address): ByteBuffer {
        val buf = ByteBuffer.allocate(allocationSize(value))
        write(value, buf)
        buf.flip()
        return buf
    }

    /** The record from a buffer Rust returned. Trailing bytes are a bug on one side. */
    fun lift(buf: ByteBuffer): Address {
        val value = read(buf)
        require(!buf.hasRemaining()) { "junk remaining in buffer after lifting an Address" }
        return value
    }

    private fun utf8(text: String): ByteArray = text.toByteArray(StandardCharsets.UTF_8)

    private fun readString(buf: ByteBuffer): String {
        val length = buf.getInt()
        require(length >= 0 && length <= buf.remaining()) { "string length out of range" }
        val bytes = ByteArray(length)
        buf.get(bytes)
        return String(bytes, StandardCharsets.UTF_8)
    }

    private fun writeString(text: String, buf: ByteBuffer) {
        val bytes = utf8(text)
        buf.putInt(bytes.size)
        buf.put(bytes)
    }

    private fun readOptionalString(buf: ByteBuffer): String? =
        when (buf.get().toInt()) {
            0 -> null
            1 -> readString(buf)
            else -> throw IllegalArgumentException("unexpected optional tag")
        }

    private fun writeOptionalString(text: String?, buf: ByteBuffer) {
        if (text == null) {
            buf.put(0)
        } else {
            buf.put(1)
            writeString(text, buf)
        }
    }
}
