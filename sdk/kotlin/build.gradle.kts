import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

// Delta Sync core for Android: the Rust core (crates/core) built with
// cargo-ndk, the UniFFI Kotlin bindings, and a Keystore-backed DeviceSigner.
//
// Requirements on the build machine:
//   rustup target add aarch64-linux-android x86_64-linux-android
//   cargo install cargo-ndk --locked
//   sdkmanager "ndk;27.2.12479018"
//
//   ./gradlew :sdk:assembleRelease      # AAR in sdk/kotlin/build/outputs/aar
//   ./gradlew :sdk:testDebugUnitTest    # JVM tests against the host build of the core

val repoRoot: File = projectDir.resolve("../..").canonicalFile
val jniLibsDir: Provider<Directory> = layout.buildDirectory.dir("generated/jniLibs")
val uniffiDir: Provider<Directory> = layout.buildDirectory.dir("generated/uniffi")
val androidAbis = listOf("arm64-v8a", "x86_64")
val androidMinSdk = 26

android {
    namespace = "cloud.deltasync.core"
    compileSdk = 36
    ndkVersion = "27.2.12479018"

    defaultConfig {
        minSdk = androidMinSdk
        consumerProguardFiles("consumer-rules.pro")
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += androidAbis }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets["main"].jniLibs.srcDir(jniLibsDir)
    sourceSets["main"].java.srcDir(uniffiDir)
    // Generated test identity and the fixtures, for the on-device signing test.
    sourceSets["androidTest"].assets.srcDir(layout.buildDirectory.dir("generated/androidTestAssets"))

    publishing {
        singleVariant("release") { withSourcesJar() }
    }
}

kotlin {
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
}

dependencies {
    // UniFFI's Kotlin bindings call the native library through JNA.
    api("net.java.dev.jna:jna:5.17.0@aar")
    testImplementation("net.java.dev.jna:jna:5.17.0")
    testImplementation(libs.junit)
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.1")
    androidTestImplementation("org.json:json:20240303")
    // Android's org.json is a stub on the JVM.
    testImplementation("org.json:json:20240303")
}

fun cargoCommand(): String {
    val home = System.getProperty("user.home")
    val candidate = File(home, ".cargo/bin/cargo")
    return if (candidate.exists()) candidate.absolutePath else "cargo"
}

fun Exec.useCargoEnv() {
    val cargoBin = File(System.getProperty("user.home"), ".cargo/bin").absolutePath
    environment("PATH", "$cargoBin${File.pathSeparator}${System.getenv("PATH") ?: ""}")
}

// Native libraries for the Android ABIs, into build/generated/jniLibs/<abi>/.
val cargoNdkBuild = tasks.register<Exec>("cargoNdkBuild") {
    group = "build"
    description = "Build libdelta_sync_core.so for Android with cargo-ndk"
    workingDir = repoRoot
    useCargoEnv()
    val ndkDir = android.ndkDirectory.absolutePath
    environment("ANDROID_NDK_HOME", ndkDir)
    val abiArgs = androidAbis.flatMap { listOf("-t", it) }
    commandLine(
        listOf(cargoCommand(), "ndk") + abiArgs + listOf(
            "--platform", androidMinSdk.toString(),
            "-o", jniLibsDir.get().asFile.absolutePath,
            "build", "--release", "-p", "delta_sync_core", "--locked",
        ),
    )
    inputs.files(
        fileTree(repoRoot.resolve("crates/core")) { exclude("target") },
        repoRoot.resolve("Cargo.toml"),
        repoRoot.resolve("Cargo.lock"),
    )
    outputs.dir(jniLibsDir)
}

// Kotlin bindings, generated from the arm64 library with the workspace's own
// uniffi-bindgen so the binding version always matches the core.
val uniffiBindings = tasks.register<Exec>("uniffiKotlinBindings") {
    group = "build"
    description = "Generate the UniFFI Kotlin bindings"
    dependsOn(cargoNdkBuild)
    workingDir = repoRoot
    useCargoEnv()
    val library = repoRoot.resolve("target/aarch64-linux-android/release/libdelta_sync_core.so")
    commandLine(
        cargoCommand(), "run", "-q", "-p", "delta_sync_core", "--features", "bindgen",
        "--bin", "uniffi-bindgen", "--locked", "--",
        "generate", "--library", library.absolutePath, "--language", "kotlin",
        "--no-format", "--out-dir", uniffiDir.get().asFile.absolutePath,
    )
    inputs.files(cargoNdkBuild)
    inputs.file(library)
    outputs.dir(uniffiDir)
}

tasks.matching { it.name == "preBuild" }.configureEach { dependsOn(uniffiBindings) }

// Host build of the core for the JVM unit tests.
val cargoHostBuild = tasks.register<Exec>("cargoHostBuild") {
    workingDir = repoRoot
    useCargoEnv()
    commandLine(cargoCommand(), "build", "-p", "delta_sync_core", "--locked")
}

val testIdentity = tasks.register<Exec>("testIdentity") {
    description = "Throwaway signing identity for the unit tests (generated, never committed)"
    workingDir = repoRoot
    useCargoEnv()
    val out = layout.buildDirectory.file("test-identity.json")
    outputs.file(out)
    outputs.upToDateWhen { false }
    commandLine(
        cargoCommand(), "run", "-q", "-p", "delta_sync_core", "--example", "test_identity", "--locked",
    )
    doFirst { standardOutput = out.get().asFile.also { it.parentFile.mkdirs() }.outputStream() }
}

tasks.withType<Test>().configureEach {
    dependsOn(cargoHostBuild, testIdentity)
    systemProperty("jna.library.path", repoRoot.resolve("target/debug").absolutePath)
    systemProperty("delta.sync.test.identity", layout.buildDirectory.file("test-identity.json").get().asFile.absolutePath)
    systemProperty("delta.sync.test.csr.out", layout.buildDirectory.file("test-csr.pem").get().asFile.absolutePath)
    systemProperty("delta.sync.test.fixtures", repoRoot.resolve("crates/core/tests/fixtures").absolutePath)
}

// On-device test inputs: a throwaway identity (never committed) and the fixtures.
val androidTestAssets = tasks.register<Copy>("androidTestAssets") {
    dependsOn(testIdentity)
    from(layout.buildDirectory.file("test-identity.json"))
    from(repoRoot.resolve("crates/core/tests/fixtures")) { include("sample.*") }
    into(layout.buildDirectory.dir("generated/androidTestAssets"))
}
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("AndroidTestAssets") }
    .configureEach { dependsOn(androidTestAssets) }
