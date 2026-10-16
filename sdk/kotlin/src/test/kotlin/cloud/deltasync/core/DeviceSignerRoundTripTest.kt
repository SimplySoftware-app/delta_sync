package cloud.deltasync.core

import java.io.File
import java.security.KeyFactory
import java.security.PrivateKey
import java.security.Signature
import java.security.spec.PKCS8EncodedKeySpec
import java.util.Base64
import java.util.UUID
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import uniffi.delta_sync_core.CoreException
import uniffi.delta_sync_core.DeviceSigner
import uniffi.delta_sync_core.SignerAlg
import uniffi.delta_sync_core.signCaptureFile
import uniffi.delta_sync_core.verifyFileJson

/**
 * Kotlin to Rust round trip on the JVM: a [DeviceSigner] implemented in Kotlin
 * signs a JPEG, MOV and MP4 through the real native core, and the result
 * verifies as HM. The key is a software P-256 key generated for this run; the
 * Android Keystore (StrongBox/TEE) only exists on a device or emulator.
 */
class DeviceSignerRoundTripTest {
    private val identity: JSONObject? =
        System.getProperty("delta.sync.test.identity")?.let(::File)?.takeIf { it.exists() }
            ?.let { JSONObject(it.readText()) }
    private val fixtures = File(System.getProperty("delta.sync.test.fixtures") ?: ".")

    private class SoftwareSigner(private val key: PrivateKey, private val chain: String) : DeviceSigner {
        override fun alg() = SignerAlg.ES256
        override fun certChain() = chain
        override fun sign(`data`: ByteArray): ByteArray {
            val der = Signature.getInstance("SHA256withECDSA").run {
                initSign(key)
                update(data)
                sign()
            }
            return EcdsaSignature.derToRaw(der)
        }
    }

    private fun signer(): Pair<SoftwareSigner, String> {
        val id = identity!!
        val pkcs8 = Base64.getDecoder().decode(id.getString("pkcs8_der_base64"))
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(pkcs8))
        return SoftwareSigner(key, id.getString("chain_pem")) to id.getString("ca_pem")
    }

    private fun verdict(path: File, ca: String) = JSONObject(verifyFileJson(path.path, ca, false))

    @Test
    fun signsJpegMovAndMp4ThroughTheForeignSigner() {
        assumeTrue("test identity not generated", identity != null)
        val (signer, ca) = signer()
        val dir = File(System.getProperty("java.io.tmpdir"), "delta-sync-${UUID.randomUUID()}").apply { mkdirs() }
        try {
            for (name in listOf("sample.jpg", "sample.mov", "sample.mp4")) {
                val out = File(dir, name)
                signCaptureFile(File(fixtures, name).path, out.path, signer)
                val result = verdict(out, ca)
                assertEquals("$name: $result", "HM", result.getString("label"))
                assertTrue("$name: $result", result.getBoolean("trusted"))
            }
        } finally {
            dir.deleteRecursively()
        }
    }

    @Test
    fun aSignerThatReturnsDerIsRejected() {
        assumeTrue("test identity not generated", identity != null)
        val (good, _) = signer()
        val der = object : DeviceSigner {
            override fun alg() = SignerAlg.ES256
            override fun certChain() = good.certChain()
            override fun sign(`data`: ByteArray): ByteArray {
                // Same key, but hand back the DER form.
                val key = KeyFactory.getInstance("EC").generatePrivate(
                    PKCS8EncodedKeySpec(Base64.getDecoder().decode(identity!!.getString("pkcs8_der_base64"))),
                )
                return Signature.getInstance("SHA256withECDSA").run { initSign(key); update(data); sign() }
            }
        }
        val out = File(System.getProperty("java.io.tmpdir"), "delta-sync-${UUID.randomUUID()}.jpg")
        val error = runCatching { signCaptureFile(File(fixtures, "sample.jpg").path, out.path, der) }
            .exceptionOrNull()
        assertTrue("$error", error is CoreException && error.message!!.contains("raw r||s"))
        assertFalse(out.exists())
    }

    @Test
    fun certificateRequestIsWellFormed() {
        val pair = java.security.KeyPairGenerator.getInstance("EC").run {
            initialize(java.security.spec.ECGenParameterSpec("secp256r1"))
            generateKeyPair()
        }
        val w = (pair.public as java.security.interfaces.ECPublicKey).w
        fun fixed(v: java.math.BigInteger) = v.toByteArray().let { b ->
            val t = if (b.size > 32) b.copyOfRange(b.size - 32, b.size) else b
            ByteArray(32 - t.size) + t
        }
        val point = byteArrayOf(0x04) + fixed(w.affineX) + fixed(w.affineY)
        val pem = CertificateRequest.build("Delta Sync device", point) {
            Signature.getInstance("SHA256withECDSA").run { initSign(pair.private); update(it); sign() }
        }
        assertTrue(pem.startsWith("-----BEGIN CERTIFICATE REQUEST-----"))
        System.getProperty("delta.sync.test.csr.out")?.let { File(it).writeText(pem) }
    }
}
