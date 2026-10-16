# DeltaSyncCore (Swift)

Swift package over the Delta Sync Rust core (`crates/core`) through UniFFI.

- `SecureEnclaveDeviceSigner`: a `DeviceSigner` whose P-256 key is created in the
  Secure Enclave and never exported. It returns raw `r || s` signatures, builds
  the PKCS#10 request for `/v1/device-certificate` and holds the chain the
  authority returns.
- Generated bindings: `signCaptureFile(inPath:outPath:signer:)` (JPEG, MOV, MP4,
  streamed from disk), `verifyFileJson(path:caPem:selfDeclaredAi:)` and the rest
  of the core.

```sh
sdk/swift/scripts/build-xcframework.sh   # DeltaSyncCoreFFI.xcframework + generated Swift (git-ignored)
sdk/swift/scripts/test.sh                # builds, then runs the tests on an iOS simulator
```

Needs the rustup targets `aarch64-apple-ios` and `aarch64-apple-ios-sim`
(`x86_64-apple-ios` with `INCLUDE_X86_64_SIM=1`).

The simulator has no Secure Enclave, so the tests sign with a software P-256 key
generated for the run (`crates/core/examples/test_identity.rs`). They cover the
Swift to Rust bridge, the foreign-trait callback, DER to `r || s` and the
verification of a signed JPEG, MOV and MP4. Creating and using a real Secure
Enclave key can only be tested on a physical device.
