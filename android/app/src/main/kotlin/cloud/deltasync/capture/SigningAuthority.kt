package cloud.deltasync.capture

import android.util.Base64
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

data class AttestationOutcome(
    val mode: String,
    val verified: Boolean,
    val detail: String,
)

data class IssuedCertificate(
    val certificateChainPem: String,
    val expiresAt: String,
    val attestation: AttestationOutcome,
)

class SigningAuthority(private val baseUrl: String) {
    suspend fun challenge(): String = withContext(Dispatchers.IO) {
        val response = post("/v1/challenge", JSONObject().put("platform", "android"))
        response.getString("challenge")
    }

    suspend fun issue(
        challenge: String,
        csrPem: String,
        packageName: String,
        integrityToken: String?,
    ): IssuedCertificate = withContext(Dispatchers.IO) {
        val body = JSONObject()
            .put("platform", "android")
            .put("challenge", challenge)
            .put("csr_pem", csrPem)
            .put("package_name", packageName)
        if (!integrityToken.isNullOrBlank()) {
            body.put("integrity_token", integrityToken)
        }
        val response = post("/v1/device-certificate", body)
        val attestation = response.getJSONObject("attestation")
        IssuedCertificate(
            certificateChainPem = response.getString("certificate_chain_pem"),
            expiresAt = response.getString("expires_at"),
            attestation = AttestationOutcome(
                mode = attestation.getString("mode"),
                verified = attestation.getBoolean("verified"),
                detail = attestation.getString("detail"),
            ),
        )
    }

    private fun post(path: String, body: JSONObject): JSONObject {
        val url = URL(baseUrl.trimEnd('/') + path)
        val connection = (url.openConnection() as HttpURLConnection).apply {
            requestMethod = "POST"
            setRequestProperty("Content-Type", "application/json")
            doOutput = true
            connectTimeout = 20_000
            readTimeout = 20_000
        }
        connection.outputStream.use { it.write(body.toString().toByteArray()) }
        val status = connection.responseCode
        val stream = if (status in 200..299) connection.inputStream else connection.errorStream
        val text = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
        if (status !in 200..299) {
            val message = runCatching { JSONObject(text).optString("error") }.getOrNull()
                ?.takeIf { it.isNotEmpty() }
                ?: text
            error("Signing authority returned $status: $message")
        }
        return JSONObject(text)
    }
}

fun integrityNonce(challenge: String, uncompressedPoint: ByteArray): String {
    val digest = MessageDigest.getInstance("SHA-256")
    digest.update(challenge.toByteArray(Charsets.UTF_8))
    digest.update(byteArrayOf(0))
    digest.update(uncompressedPoint)
    return Base64.encodeToString(digest.digest(), Base64.URL_SAFE or Base64.NO_PADDING or Base64.NO_WRAP)
}
