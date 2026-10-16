package cloud.deltasync.core

import java.io.ByteArrayOutputStream
import java.util.Base64

/**
 * PKCS#10 certificate request for `/v1/device-certificate`, signed by a P-256
 * key that may live in hardware: the key only has to be able to sign.
 */
object CertificateRequest {
    /**
     * @param publicPoint uncompressed X9.63 point, `04 || X || Y` (65 bytes)
     * @param sign signs the DER `CertificationRequestInfo` with SHA256withECDSA
     *             and returns the DER signature as the platform produces it
     */
    fun build(commonName: String, publicPoint: ByteArray, sign: (ByteArray) -> ByteArray): String {
        require(publicPoint.size == 65 && publicPoint[0].toInt() == 0x04) {
            "Expected an uncompressed P-256 point"
        }
        val spki = sequence(
            sequence(oid(EC_PUBLIC_KEY), oid(PRIME256V1)),
            bitString(publicPoint),
        )
        val name = sequence(set(sequence(oid(COMMON_NAME), utf8(commonName))))
        val info = sequence(integerZero(), name, spki, byteArrayOf(0xA0.toByte(), 0x00))
        val signature = sign(info)
        val request = sequence(
            info,
            sequence(oid(ECDSA_WITH_SHA256)),
            bitString(signature),
        )
        val body = Base64.getMimeEncoder(64, "\n".toByteArray()).encodeToString(request)
        return "-----BEGIN CERTIFICATE REQUEST-----\n$body\n-----END CERTIFICATE REQUEST-----\n"
    }

    private val EC_PUBLIC_KEY = byteArrayOf(0x2A, 0x86.toByte(), 0x48, 0xCE.toByte(), 0x3D, 0x02, 0x01)
    private val PRIME256V1 = byteArrayOf(0x2A, 0x86.toByte(), 0x48, 0xCE.toByte(), 0x3D, 0x03, 0x01, 0x07)
    private val ECDSA_WITH_SHA256 = byteArrayOf(0x2A, 0x86.toByte(), 0x48, 0xCE.toByte(), 0x3D, 0x04, 0x03, 0x02)
    private val COMMON_NAME = byteArrayOf(0x55, 0x04, 0x03)

    private fun sequence(vararg parts: ByteArray) = tag(0x30, concat(parts))
    private fun set(vararg parts: ByteArray) = tag(0x31, concat(parts))
    private fun integerZero() = byteArrayOf(0x02, 0x01, 0x00)
    private fun utf8(text: String) = tag(0x0C, text.toByteArray(Charsets.UTF_8))
    private fun oid(body: ByteArray) = tag(0x06, body)
    private fun bitString(payload: ByteArray) = tag(0x03, byteArrayOf(0x00) + payload)

    private fun concat(parts: Array<out ByteArray>): ByteArray {
        val out = ByteArrayOutputStream()
        parts.forEach { out.write(it) }
        return out.toByteArray()
    }

    private fun tag(tag: Int, body: ByteArray): ByteArray {
        val out = ByteArrayOutputStream()
        out.write(tag)
        when {
            body.size < 0x80 -> out.write(body.size)
            body.size < 0x100 -> { out.write(0x81); out.write(body.size) }
            else -> { out.write(0x82); out.write(body.size shr 8); out.write(body.size and 0xFF) }
        }
        out.write(body)
        return out.toByteArray()
    }
}
