package cloud.deltasync.capture

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.view.CameraController
import androidx.camera.view.LifecycleCameraController
import androidx.camera.view.PreviewView
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import java.io.File
import java.util.concurrent.Executors

private val Paper = Color(0xFFEEE9DF)
private val Ink = Color(0xFF1C1C1E)
private val Navy = Color(0xFF2C3B4D)

@Composable
fun CaptureScreen(model: CaptureViewModel = viewModel()) {
    val state by model.state.collectAsState()
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val camera = remember {
        LifecycleCameraController(context).apply {
            setEnabledUseCases(CameraController.IMAGE_CAPTURE)
        }
    }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) camera.bindToLifecycle(lifecycleOwner)
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Paper)
            .verticalScroll(rememberScrollState())
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Image(painterResource(R.drawable.ic_shield), contentDescription = null)
            Spacer(Modifier.padding(6.dp))
            Column {
                Text("DeltaSync", color = Navy, fontSize = 24.sp, fontWeight = FontWeight.Bold)
                Text("Sign a photo as it is captured.", color = Ink.copy(alpha = 0.75f))
            }
        }
        OutlinedTextField(
            value = state.serverUrl,
            onValueChange = model::updateServer,
            label = { Text("Signing authority") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
        )
        OutlinedTextField(
            value = state.cloudProjectNumber,
            onValueChange = model::updateProject,
            label = { Text("Google Cloud project number") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Continue without Play Integrity", modifier = Modifier.weight(1f), color = Ink)
            Switch(checked = state.allowMissingIntegrity, onCheckedChange = model::updateAllowMissing)
        }
        Text(
            "Use that switch only with a stub server. A missing Play Integrity token is not a device proof.",
            color = Ink.copy(alpha = 0.7f),
            fontSize = 13.sp,
        )
        Text(
            state.status,
            modifier = Modifier
                .fillMaxWidth()
                .background(Color.White, RoundedCornerShape(12.dp))
                .padding(14.dp),
            color = Ink,
        )
        Button(
            onClick = model::prepare,
            enabled = !state.busy,
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.buttonColors(containerColor = Navy),
        ) {
            Image(painterResource(R.drawable.ic_shield), contentDescription = null)
            Text("  Prepare device")
        }
        AndroidView(
            factory = { ctx ->
                PreviewView(ctx).apply {
                    this.controller = camera
                    val allowed = ContextCompat.checkSelfPermission(ctx, Manifest.permission.CAMERA) ==
                        PackageManager.PERMISSION_GRANTED
                    if (allowed) camera.bindToLifecycle(lifecycleOwner) else permission.launch(Manifest.permission.CAMERA)
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(280.dp),
        )
        OutlinedButton(
            onClick = {
                val file = File(context.cacheDir, "deltasync-raw.jpg")
                val options = ImageCapture.OutputFileOptions.Builder(file).build()
                camera.takePicture(
                    options,
                    Executors.newSingleThreadExecutor(),
                    object : ImageCapture.OnImageSavedCallback {
                        override fun onImageSaved(outputFileResults: ImageCapture.OutputFileResults) {
                            model.sign(file.readBytes())
                        }

                        override fun onError(exception: ImageCaptureException) {
                            model.report(exception.message ?: "The camera did not return a photo.")
                        }
                    },
                )
            },
            enabled = !state.busy,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Image(painterResource(R.drawable.ic_camera), contentDescription = null)
            Text("  Capture and sign", color = Navy)
        }
        state.signedFile?.let {
            Button(
                onClick = {
                    val uri = model.shareUri() ?: return@Button
                    val share = Intent(Intent.ACTION_SEND).apply {
                        type = "image/jpeg"
                        putExtra(Intent.EXTRA_STREAM, uri)
                        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    }
                    context.startActivity(Intent.createChooser(share, "Signed capture"))
                },
                modifier = Modifier.fillMaxWidth(),
                colors = ButtonDefaults.buttonColors(containerColor = Navy),
            ) {
                Text("Share signed JPEG")
            }
        }
    }
}
