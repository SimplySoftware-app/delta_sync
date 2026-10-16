use thiserror::Error;

#[derive(Debug, Error)]
#[cfg_attr(feature = "ffi", derive(uniffi::Error))]
pub enum CoreError {
    #[error("{message}")]
    Message { message: String },
}

impl CoreError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}
