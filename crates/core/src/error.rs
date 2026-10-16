use thiserror::Error;

#[derive(Debug, Error)]
#[cfg_attr(feature = "ffi", derive(uniffi::Error))]
pub enum CoreError {
    // The field is not called `message`: in the generated Kotlin it would clash
    // with `Throwable.message`.
    #[error("{reason}")]
    Message { reason: String },
}

impl CoreError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message {
            reason: message.into(),
        }
    }
}
