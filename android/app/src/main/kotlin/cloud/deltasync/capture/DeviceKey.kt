package cloud.deltasync.capture

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import org.contentauth.c2pa.CertificateManager
import org.contentauth.c2pa.StrongBoxSigner
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec

data class PreparedKey(
    val alias: String,
    val backing: String,
    val csrPem: String,
    val uncompressedPoint: ByteArray,
)

object DeviceKey {
    const val ALIAS = "cloud.deltasync.capture.signing"

    fun loadOrCreate(): PreparedKey {
        val created = ensureKey()
        val csr = CertificateManager.createCSR(
            ALIAS,
            CertificateManager.CertificateConfig(commonName = "DeltaSync device"),
        )
        return PreparedKey(
            alias = ALIAS,
            backing = created,
            csrPem = csr,
            uncompressedPoint = publicPoint(),
        )
    }

    private fun ensureKey(): String {
        val store = keyStore()
        if (!store.containsAlias(ALIAS)) {
            try {
                StrongBoxSigner.createKey(
                    StrongBoxSigner.Config(
                        keyTag = ALIAS,
                        requireUserAuthentication = false,
                    ),
                )
            } catch (_: Exception) {
                generate(strongBox = false)
            }
        }
        return when {
            runCatching { StrongBoxSigner.isKeyStrongBoxBacked(ALIAS) }.getOrDefault(false) ->
                "StrongBox"
            runCatching { org.contentauth.c2pa.KeyStoreSigner.isKeyHardwareBacked(ALIAS) }.getOrDefault(false) ->
                "Android Keystore (hardware)"
            else -> "Android Keystore (software)"
        }
    }

    private fun generate(strongBox: Boolean) {
        val builder = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_SIGN)
            .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
            .setDigests(KeyProperties.DIGEST_SHA256)
            .setUserAuthenticationRequired(false)
        if (strongBox) {
            builder.setIsStrongBoxBacked(true)
        }
        val generator = KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, "AndroidKeyStore")
        generator.initialize(builder.build())
        generator.generateKeyPair()
    }

    private fun publicPoint(): ByteArray {
        val certificate = keyStore().getCertificate(ALIAS)
        val publicKey = certificate.publicKey as ECPublicKey
        val x = fixed32(publicKey.w.affineX.toByteArray())
        val y = fixed32(publicKey.w.affineY.toByteArray())
        return byteArrayOf(0x04) + x + y
    }

    private fun fixed32(value: ByteArray): ByteArray {
        val trimmed = if (value.size > 32) value.copyOfRange(value.size - 32, value.size) else value
        return ByteArray(32 - trimmed.size) + trimmed
    }

    private fun keyStore(): KeyStore =
        KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
}
