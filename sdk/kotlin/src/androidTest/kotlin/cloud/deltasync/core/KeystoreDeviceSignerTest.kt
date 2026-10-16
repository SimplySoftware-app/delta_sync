package cloud.deltasync.core

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.security.PrivateKey
import java.security.spec.PKCS8EncodedKeySpec
import java.util.Base64
import org.json.JSONObject
import uniffi.delta_sync_core.DeviceSigner
import uniffi.delta_sync_core.coreVersion
import uniffi.delta_sync_core.signCaptureFile
import uniffi.delta_sync_core.verifyFileJson
import java.math.BigInteger
import java.security.KeyFactory
import java.security.Signature
import java.security.spec.ECPoint
import java.security.spec.ECPublicKeySpec
import java.security.AlgorithmParameters
import java.security.spec.ECGenParameterSpec
import java.security.spec.ECParameterSpec
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.delta_sync_core.CoreException
import uniffi.delta_sync_core.SignerAlg

/**
 * Runs on a device or emulator. It exercises the real Android Keystore path:
 * key creation with StrongBox-then-TEE fallback, attestation challenge, raw
 * r||s output and the CSR. An emulator has no StrongBox and no hardware TEE,
 * so this does not prove hardware backing: that needs a physical phone.
 */
@RunWith(AndroidJUnit4::class)
class KeystoreDeviceSignerTest {
    private val alias = "cloud.deltasync.core.test"

    @After
    fun cleanUp() = KeystoreDeviceSigner.delete(alias)

    private fun toRawVerifies(point: ByteArray, data: ByteArray, raw: ByteArray): Boolean {
        val params = AlgorithmParameters.getInstance("EC").run {
            init(ECGenParameterSpec("secp256r1"))
            getParameterSpec(ECParameterSpec::class.java)
        }
        val x = BigInteger(1, point.copyOfRange(1, 33))
        val y = BigInteger(1, point.copyOfRange(33, 65))
        val key = KeyFactory.getInstance("EC").generatePublic(ECPublicKeySpec(ECPoint(x, y), params))
        val r = BigInteger(1, raw.copyOfRange(0, 32))
        val s = BigInteger(1, raw.copyOfRange(32, 64))
        // Re-encode as DER so the JCA verifier can check the P1363 output.
        fun der(v: BigInteger) = v.toByteArray().let { byteArrayOf(0x02, it.size.toByte()) + it }
        val body = der(r) + der(s)
        val sig = byteArrayOf(0x30, body.size.toByte()) + body
        return Signature.getInstance("SHA256withECDSA").run {
            initVerify(key)
            update(data)
            verify(sig)
        }
    }

    @Test
    fun createsAKeySignsRawAndBuildsACsr() {
        val info = KeystoreDeviceSigner.createKey(alias, "challenge-from-authority".toByteArray())
        assertTrue(KeystoreDeviceSigner.exists(alias))
        println("strongBox=${info.strongBox} attestationCerts=${info.attestationChainPem.split("BEGIN CERTIFICATE").size - 1}")

        val signer = KeystoreDeviceSigner(alias, "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n")
        assertEquals(SignerAlg.ES256, signer.alg())
        val data = "claim bytes".toByteArray()
        val raw = signer.sign(data)
        assertEquals(64, raw.size)
        assertTrue("the Keystore signature must verify against its public key", toRawVerifies(signer.publicPoint(), data, raw))

        assertTrue(signer.certificateRequest("Delta Sync device").contains("BEGIN CERTIFICATE REQUEST"))
    }

    @Test
    fun refusesToSignBeforeEnrolment() {
        KeystoreDeviceSigner.createKey(alias, ByteArray(16))
        val signer = KeystoreDeviceSigner(alias)
        val error = runCatching { signer.certChain() }.exceptionOrNull()
        assertTrue("$error", error is CoreException)
    }

    /**
     * The native core on Android (arm64 on this emulator): a DeviceSigner with a
     * software key signs a JPEG, MOV and MP4 and each verifies as HM. The
     * identity and fixtures are test assets generated at build time.
     */
    @Test
    fun nativeCoreSignsJpegMovAndMp4OnDevice() {
        println("core ${coreVersion()}")
        val context = InstrumentationRegistry.getInstrumentation().context
        val identity = JSONObject(context.assets.open("test-identity.json").bufferedReader().readText())
        val key: PrivateKey = KeyFactory.getInstance("EC").generatePrivate(
            PKCS8EncodedKeySpec(Base64.getDecoder().decode(identity.getString("pkcs8_der_base64"))),
        )
        val signer = object : DeviceSigner {
            override fun alg() = SignerAlg.ES256
            override fun certChain() = identity.getString("chain_pem")
            override fun sign(`data`: ByteArray): ByteArray = EcdsaSignature.derToRaw(
                Signature.getInstance("SHA256withECDSA").run { initSign(key); update(data); sign() },
            )
        }
        val dir = File(InstrumentationRegistry.getInstrumentation().targetContext.cacheDir, "sign-test").apply { mkdirs() }
        for (name in listOf("sample.jpg", "sample.mov", "sample.mp4")) {
            val input = File(dir, "in-$name")
            context.assets.open(name).use { src -> input.outputStream().use { src.copyTo(it) } }
            val output = File(dir, "out-$name")
            signCaptureFile(input.path, output.path, signer)
            val verdict = JSONObject(verifyFileJson(output.path, identity.getString("ca_pem"), false))
            assertEquals("$name: $verdict", "HM", verdict.getString("label"))
        }
        dir.deleteRecursively()
    }
}
