package cloud.deltasync.capture

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.core.content.FileProvider
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import cloud.deltasync.core.KeystoreDeviceSigner
import com.google.android.play.core.integrity.IntegrityManagerFactory
import com.google.android.play.core.integrity.IntegrityTokenRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.tasks.await
import kotlinx.coroutines.withContext
import java.io.File

data class CaptureUiState(
    val serverUrl: String = "http://10.0.2.2:8080",
    val cloudProjectNumber: String = "",
    val allowMissingIntegrity: Boolean = false,
    val status: String = "Not prepared.",
    val backing: String = "",
    val busy: Boolean = false,
    val signedFile: File? = null,
)

class CaptureViewModel(application: Application) : AndroidViewModel(application) {
    private val preferences = application.getSharedPreferences("deltasync", Context.MODE_PRIVATE)
    private val _state = MutableStateFlow(
        CaptureUiState(
            serverUrl = preferences.getString("serverUrl", "http://10.0.2.2:8080")!!,
            cloudProjectNumber = preferences.getString("cloudProjectNumber", "")!!,
            allowMissingIntegrity = preferences.getBoolean("allowMissingIntegrity", false),
        ),
    )
    val state: StateFlow<CaptureUiState> = _state.asStateFlow()

    private var signer: KeystoreDeviceSigner? = null

    fun updateServer(value: String) = update("serverUrl", value) { copy(serverUrl = value) }
    fun updateProject(value: String) = update("cloudProjectNumber", value) { copy(cloudProjectNumber = value) }
    fun updateAllowMissing(value: Boolean) {
        preferences.edit().putBoolean("allowMissingIntegrity", value).apply()
        _state.update { it.copy(allowMissingIntegrity = value) }
    }

    fun prepare() {
        launch("Preparing the device key…") {
            val key = withContext(Dispatchers.IO) { DeviceKey.loadOrCreate() }
            val authority = SigningAuthority(_state.value.serverUrl)
            val challenge = authority.challenge()
            val token = requestIntegrity(challenge, key.uncompressedPoint)
            val issued = authority.issue(
                challenge = challenge,
                csrPem = key.csrPem,
                packageName = getApplication<Application>().packageName,
                integrityToken = token,
            )
            key.signer.setCertificateChain(issued.certificateChainPem)
            signer = key.signer
            _state.update {
                it.copy(
                    backing = key.backing,
                    status = "${key.backing}. Certificate expires ${issued.expiresAt}. ${issued.attestation.detail}",
                )
            }
        }
    }

    fun report(message: String) {
        _state.update { it.copy(status = message, busy = false) }
    }

    fun sign(jpeg: ByteArray) {
        launch("Signing the capture…") {
            val deviceSigner = signer ?: error("Prepare the device before capturing.")
            val file = File(getApplication<Application>().cacheDir, "deltasync-signed.jpg")
            withContext(Dispatchers.IO) {
                ManifestSigner.signJpeg(jpeg, deviceSigner, file)
            }
            _state.update {
                it.copy(
                    signedFile = file,
                    status = "Signed ${file.length()} bytes with the ${it.backing.ifBlank { "device" }} key.",
                )
            }
        }
    }

    fun shareUri(): Uri? {
        val file = _state.value.signedFile ?: return null
        val context = getApplication<Application>()
        return FileProvider.getUriForFile(context, "${context.packageName}.files", file)
    }

    private suspend fun requestIntegrity(challenge: String, point: ByteArray): String? {
        val nonce = integrityNonce(challenge, point)
        val project = _state.value.cloudProjectNumber.toLongOrNull()
        return try {
            val manager = IntegrityManagerFactory.create(getApplication())
            val request = IntegrityTokenRequest.builder()
                .setNonce(nonce)
                .apply { if (project != null) setCloudProjectNumber(project) }
                .build()
            manager.requestIntegrityToken(request).await().token()
        } catch (error: Exception) {
            if (_state.value.allowMissingIntegrity) {
                null
            } else {
                throw IllegalStateException(
                    "Play Integrity did not return a token (${error.message}). " +
                        "For a stub server, turn on “Continue without Play Integrity”.",
                    error,
                )
            }
        }
    }

    private fun launch(start: String, work: suspend () -> Unit) {
        viewModelScope.launch {
            _state.update { it.copy(busy = true, status = start) }
            try {
                work()
            } catch (error: Exception) {
                _state.update { it.copy(status = error.message ?: "Something went wrong.") }
            } finally {
                _state.update { it.copy(busy = false) }
            }
        }
    }

    private fun update(key: String, value: String, reduce: CaptureUiState.() -> CaptureUiState) {
        preferences.edit().putString(key, value).apply()
        _state.update(reduce)
    }
}
