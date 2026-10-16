package cloud.deltasync.capture

import cloud.deltasync.core.KeystoreDeviceSigner
import java.io.File
import uniffi.delta_sync_core.signCaptureFile

/**
 * Signs a capture with the Delta Sync core. The Rust core builds the C2PA
 * manifest; the Keystore key signs through [KeystoreDeviceSigner].
 */
object ManifestSigner {
    fun signJpeg(jpeg: ByteArray, signer: KeystoreDeviceSigner, destination: File): File {
        val input = File.createTempFile("deltasync-in", ".jpg", destination.parentFile)
        try {
            input.writeBytes(jpeg)
            signCaptureFile(input.path, destination.path, signer)
        } finally {
            input.delete()
        }
        return destination
    }
}
