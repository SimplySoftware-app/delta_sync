package cloud.deltasync.capture

import org.contentauth.c2pa.Builder
import org.contentauth.c2pa.ByteArrayStream
import org.contentauth.c2pa.DataStream
import org.contentauth.c2pa.KeyStoreSigner
import org.contentauth.c2pa.SigningAlgorithm
import org.contentauth.c2pa.StrongBoxSigner
import java.io.File

object ManifestSigner {
    fun signJpeg(jpeg: ByteArray, certificateChainPem: String, alias: String, destination: File): File {
        val signer = if (runCatching { StrongBoxSigner.isKeyStrongBoxBacked(alias) }.getOrDefault(false)) {
            StrongBoxSigner.createSigner(
                SigningAlgorithm.ES256,
                certificateChainPem,
                StrongBoxSigner.Config(keyTag = alias, requireUserAuthentication = false),
                null,
            )
        } else {
            KeyStoreSigner.createSigner(
                SigningAlgorithm.ES256,
                certificateChainPem,
                alias,
                null,
            )
        }
        val builder = Builder.fromJson(MANIFEST)
        val source = DataStream(jpeg)
        val output = ByteArrayStream()
        try {
            builder.sign("image/jpeg", source, output, signer)
            destination.writeBytes(output.getData())
        } finally {
            builder.close()
            source.close()
            output.close()
            signer.close()
        }
        return destination
    }

    private val MANIFEST = """
        {
          "claim_generator": "DeltaSync Android/0.1.0",
          "title": "DeltaSync capture",
          "format": "image/jpeg",
          "assertions": [
            {
              "label": "c2pa.actions",
              "data": {
                "actions": [
                  {
                    "action": "c2pa.created",
                    "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
                  }
                ]
              }
            }
          ]
        }
    """.trimIndent()
}
