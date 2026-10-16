package cloud.deltasync.core

import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.Signature
import java.security.cert.Certificate
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec
import java.util.Base64
import uniffi.delta_sync_core.CoreException
import uniffi.delta_sync_core.DeviceSigner
import uniffi.delta_sync_core.SignerAlg

/** Where the private key lives, as reported by the Keystore. */
enum class KeySecurityLevel { STRONG_BOX, TRUSTED_ENVIRONMENT, SOFTWARE }

/** What a freshly created device key looks like. */
data class DeviceKeyInfo(
    /** True when the key lives in StrongBox, false for the TEE (or software) fallback. */
    val strongBox: Boolean,
    /**
     * Key attestation chain, leaf first, as PEM. Send it with the challenge
     * nonce to the authority, which checks it against Google's roots.
     * Empty when the device cannot attest.
     */
    val attestationChainPem: String,
)

/**
 * A [DeviceSigner] backed by an EC P-256 key in the Android Keystore.
 *
 * The key is generated inside the secure hardware and is not exportable.
 * StrongBox is preferred; devices without it fall back to the TEE.
 * The Rust core builds the C2PA claim and calls [sign] with the bytes to
 * sign; this class returns the raw `r || s` signature COSE expects.
 *
 * The certificate chain is only known after enrolment: create the key with
 * [createKey] (passing the authority's challenge as the attestation
 * challenge), build a request with [certificateRequest], then call
 * [setCertificateChain] with the PEM the authority returned.
 */
class KeystoreDeviceSigner(
    private val alias: String,
    certificateChainPem: String = "",
) : DeviceSigner {
    @Volatile
    private var chainPem: String = certificateChainPem

    fun setCertificateChain(pem: String) {
        chainPem = pem
    }

    /** Uncompressed X9.63 public point, `04 || X || Y`. */
    fun publicPoint(): ByteArray {
        val key = keyStore().getCertificate(alias)?.publicKey as? ECPublicKey
            ?: throw IllegalStateException("No device key under alias $alias")
        return byteArrayOf(0x04) + fixed32(key.w.affineX.toByteArray()) + fixed32(key.w.affineY.toByteArray())
    }

    /** PKCS#10 request for `/v1/device-certificate`, signed by the device key. */
    fun certificateRequest(commonName: String): String =
        CertificateRequest.build(commonName, publicPoint()) { derSign(it) }

    // DeviceSigner

    override fun alg(): SignerAlg = SignerAlg.ES256

    override fun certChain(): String {
        val pem = chainPem
        if (!pem.contains("BEGIN CERTIFICATE")) {
            throw CoreException.Message("The device has no certificate yet: enrol before signing.")
        }
        return pem
    }

    override fun sign(`data`: ByteArray): ByteArray {
        try {
            return EcdsaSignature.derToRaw(derSign(data))
        } catch (e: CoreException) {
            throw e
        } catch (e: Exception) {
            throw CoreException.Message("The device key could not sign: ${e.message}")
        }
    }

    private fun derSign(data: ByteArray): ByteArray {
        val key = keyStore().getKey(alias, null) as? PrivateKey
            ?: throw IllegalStateException("No device key under alias $alias")
        return Signature.getInstance("SHA256withECDSA").run {
            initSign(key)
            update(data)
            sign()
        }
    }

    companion object {
        private const val PROVIDER = "AndroidKeyStore"

        fun exists(alias: String): Boolean = keyStore().containsAlias(alias)

        /** Security level the Keystore reports for the key under [alias]. */
        fun securityLevel(alias: String): KeySecurityLevel {
            val key = keyStore().getKey(alias, null) as? PrivateKey
                ?: throw IllegalStateException("No device key under alias $alias")
            val info = KeyFactory.getInstance(key.algorithm, PROVIDER)
                .getKeySpec(key, KeyInfo::class.java)
            return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                when (info.securityLevel) {
                    android.security.keystore.KeyProperties.SECURITY_LEVEL_STRONGBOX -> KeySecurityLevel.STRONG_BOX
                    android.security.keystore.KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT ->
                        KeySecurityLevel.TRUSTED_ENVIRONMENT
                    else -> KeySecurityLevel.SOFTWARE
                }
            } else if (info.isInsideSecureHardware) {
                KeySecurityLevel.TRUSTED_ENVIRONMENT
            } else {
                KeySecurityLevel.SOFTWARE
            }
        }

        fun delete(alias: String) {
            keyStore().deleteEntry(alias)
        }

        /**
         * Create the signing key, preferring StrongBox and falling back to the
         * TEE, with key attestation bound to [attestationChallenge].
         */
        fun createKey(
            alias: String,
            attestationChallenge: ByteArray,
            preferStrongBox: Boolean = true,
        ): DeviceKeyInfo {
            if (preferStrongBox) {
                try {
                    generate(alias, attestationChallenge, strongBox = true)
                    return info(alias, strongBox = true)
                } catch (_: StrongBoxUnavailableException) {
                    delete(alias)
                }
            }
            generate(alias, attestationChallenge, strongBox = false)
            return info(alias, strongBox = false)
        }

        private fun generate(alias: String, challenge: ByteArray, strongBox: Boolean) {
            val spec = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_SIGN)
                .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
                .setDigests(KeyProperties.DIGEST_SHA256)
                .setUserAuthenticationRequired(false)
                .setAttestationChallenge(challenge)
                .setIsStrongBoxBacked(strongBox)
                .build()
            KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, PROVIDER).run {
                initialize(spec)
                generateKeyPair()
            }
        }

        private fun info(alias: String, strongBox: Boolean): DeviceKeyInfo {
            val chain: Array<Certificate> = keyStore().getCertificateChain(alias) ?: emptyArray()
            return DeviceKeyInfo(strongBox, chain.joinToString("") { toPem(it.encoded) })
        }

        private fun toPem(der: ByteArray): String {
            val body = Base64.getMimeEncoder(64, "\n".toByteArray()).encodeToString(der)
            return "-----BEGIN CERTIFICATE-----\n$body\n-----END CERTIFICATE-----\n"
        }

        private fun keyStore(): KeyStore = KeyStore.getInstance(PROVIDER).apply { load(null) }

        private fun fixed32(value: ByteArray): ByteArray {
            val trimmed = if (value.size > 32) value.copyOfRange(value.size - 32, value.size) else value
            return ByteArray(32 - trimmed.size) + trimmed
        }
    }
}
