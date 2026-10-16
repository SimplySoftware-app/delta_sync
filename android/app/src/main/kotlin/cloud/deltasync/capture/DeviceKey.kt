package cloud.deltasync.capture

import cloud.deltasync.core.KeySecurityLevel
import cloud.deltasync.core.KeystoreDeviceSigner
import java.security.SecureRandom

data class PreparedKey(
    val signer: KeystoreDeviceSigner,
    val backing: String,
    val csrPem: String,
    val uncompressedPoint: ByteArray,
)

object DeviceKey {
    const val ALIAS = "cloud.deltasync.capture.signing"

    /**
     * The signing key lives in the Android Keystore (StrongBox when the phone
     * has it, the TEE otherwise) and is never exported. The Delta Sync core
     * asks this signer for signatures; it never sees the key.
     */
    fun loadOrCreate(): PreparedKey {
        if (!KeystoreDeviceSigner.exists(ALIAS)) {
            // The authority attests the app with Play Integrity; the key
            // attestation challenge only has to be unpredictable here.
            val challenge = ByteArray(32).also { SecureRandom().nextBytes(it) }
            KeystoreDeviceSigner.createKey(ALIAS, challenge)
        }
        val signer = KeystoreDeviceSigner(ALIAS)
        val backing = when (KeystoreDeviceSigner.securityLevel(ALIAS)) {
            KeySecurityLevel.STRONG_BOX -> "StrongBox"
            KeySecurityLevel.TRUSTED_ENVIRONMENT -> "Android Keystore (hardware)"
            KeySecurityLevel.SOFTWARE -> "Android Keystore (software)"
        }
        return PreparedKey(
            signer = signer,
            backing = backing,
            csrPem = signer.certificateRequest("DeltaSync device"),
            uncompressedPoint = signer.publicPoint(),
        )
    }
}
