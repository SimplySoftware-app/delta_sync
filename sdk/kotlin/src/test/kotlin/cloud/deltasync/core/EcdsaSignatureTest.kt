package cloud.deltasync.core

import java.security.KeyPairGenerator
import java.security.Signature
import java.security.spec.ECGenParameterSpec
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class EcdsaSignatureTest {
    @Test
    fun padsShortIntegersAndStripsSignExtension() {
        val r = ByteArray(32) { 0xFF.toByte() }
        val s = byteArrayOf(0x01, 0x02)
        val body = byteArrayOf(0x02, 33, 0x00) + r + byteArrayOf(0x02, s.size.toByte()) + s
        val der = byteArrayOf(0x30, body.size.toByte()) + body
        val raw = EcdsaSignature.derToRaw(der)
        assertEquals(64, raw.size)
        assertArrayEquals(r, raw.copyOfRange(0, 32))
        assertArrayEquals(ByteArray(30) + s, raw.copyOfRange(32, 64))
    }

    @Test
    fun convertsRealJcaSignaturesOfEveryLength() {
        val pair = KeyPairGenerator.getInstance("EC").run {
            initialize(ECGenParameterSpec("secp256r1"))
            generateKeyPair()
        }
        repeat(200) { n ->
            val sig = Signature.getInstance("SHA256withECDSA").run {
                initSign(pair.private)
                update("message $n".toByteArray())
                sign()
            }
            val raw = EcdsaSignature.derToRaw(sig)
            assertEquals(64, raw.size)
        }
    }

    @Test
    fun rejectsMalformedInput() {
        assertThrows(IllegalArgumentException::class.java) { EcdsaSignature.derToRaw(byteArrayOf(1, 2)) }
        assertThrows(IllegalArgumentException::class.java) {
            EcdsaSignature.derToRaw(byteArrayOf(0x30, 0x03, 0x02, 0x01, 0x01))
        }
        assertThrows(IllegalArgumentException::class.java) { EcdsaSignature.derToRaw(ByteArray(64)) }
    }
}
