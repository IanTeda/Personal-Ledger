#![allow(unused)] // Allow unused code during development

/// Telemetry-specific error types for logging and tracing operations.
///
/// This enum provides detailed error classification for telemetry-related failures,
/// allowing for precise error handling and debugging of logging infrastructure issues.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Generic telemetry errors.
    ///
    /// This covers generic telemetry issues that don't fit into the more specific
    /// error categories above.
    #[error("Telemetry generic error: {0}")]
    Generic(String),
}

/// Helper methods for creating `TelemetryError` variants.
///
/// These methods provide convenient constructors for common error scenarios,
/// allowing for more ergonomic error creation throughout the telemetry module.
impl Error {
    /// Creates a new generic configuration error.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of the configuration issue
    ///
    /// # Examples
    ///
    /// ```rust
    /// let error = lib_tracing::TelemetryError::generic("There is a non-defined error in the telemetry");
    /// ```
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TelemetryResult;
    use fake::Fake;
    use fake::faker::lorem::en::Sentence;

    #[test]
    fn test_generic_error_with_string() {
        let message = Sentence(3..8).fake::<String>();
        let error = Error::generic(message.clone());

        match error {
            Error::Generic(msg) => assert_eq!(msg, message),
        }
    }

    #[test]
    fn test_generic_error_with_str() {
        let message = Sentence(3..8).fake::<String>();
        let error = Error::generic(message.as_str());

        match error {
            Error::Generic(msg) => assert_eq!(msg, message),
        }
    }

    #[test]
    fn test_generic_error_display() {
        let message = Sentence(3..8).fake::<String>();
        let error = Error::generic(message.clone());
        let display_msg = format!("{}", error);

        assert_eq!(display_msg, format!("Telemetry generic error: {}", message));
    }

    #[test]
    fn test_generic_error_debug() {
        let message = Sentence(3..8).fake::<String>();
        let error = Error::generic(message.clone());
        let debug_msg = format!("{:?}", error);

        // The debug output should contain the variant name and message
        assert!(debug_msg.contains("Generic"));
        assert!(debug_msg.contains(&message));
    }

    #[test]
    fn test_telemetry_result_type_alias() {
        // Test that TelemetryResult works as expected
        let result: TelemetryResult<i32> = Ok(42);
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value, 42);
        }

        let error_message = Sentence(2..5).fake::<String>();
        let error_result: TelemetryResult<i32> = Err(Error::generic(error_message));
        assert!(error_result.is_err());
    }
}
