package cloud.deltasync.core

/** Conversions between the two encodings of an ECDSA signature. */
object EcdsaSignature {
    /**
     * Java's `SHA256withECDSA` returns ASN.1 DER (`SEQUENCE { INTEGER r, INTEGER s }`).
     * COSE (and so C2PA) wants the fixed-width `r || s` form, 64 bytes for P-256.
     */
    fun derToRaw(der: ByteArray, componentLength: Int = 32): ByteArray {
        var index = 0

        fun need(count: Int) {
            require(index + count <= der.size) { "Truncated ECDSA signature" }
        }

        fun readLength(): Int {
            need(1)
            val first = der[index++].toInt() and 0xFF
            if (first < 0x80) return first
            val count = first and 0x7F
            require(count in 1..2) { "Bad ECDSA length encoding" }
            need(count)
            var value = 0
            repeat(count) { value = (value shl 8) or (der[index++].toInt() and 0xFF) }
            return value
        }

        fun readInteger(): ByteArray {
            need(1)
            require(der[index++].toInt() == 0x02) { "ECDSA signature is not DER" }
            val length = readLength()
            require(length > 0) { "Empty ECDSA integer" }
            need(length)
            var start = index
            val end = index + length
            index = end
            // Drop the sign-extension zero(s), keep at least one byte.
            while (end - start > 1 && der[start].toInt() == 0) start++
            val size = end - start
            require(size <= componentLength) { "ECDSA integer too large for this curve" }
            return ByteArray(componentLength - size) + der.copyOfRange(start, end)
        }

        need(1)
        require(der[index++].toInt() == 0x30) { "ECDSA signature is not DER" }
        val sequenceLength = readLength()
        require(index + sequenceLength == der.size) { "Bad ECDSA sequence length" }
        val r = readInteger()
        val s = readInteger()
        require(index == der.size) { "Trailing bytes after ECDSA signature" }
        return r + s
    }
}
