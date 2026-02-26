//! Base64 encoding utilities.

use base64::{Engine, prelude::BASE64_STANDARD};
use crate::error::CryptoError;

/// Encodes bytes to a base64 string.
pub fn base64_encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

/// Decodes a base64 string to bytes.
///
/// # Errors
///
/// Returns `CryptoError::DecryptionError` if the input is not valid base64.
pub fn base64_decode(encoded: &str) -> Result<Vec<u8>, CryptoError> {
    BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_empty() {
        let result = base64_encode(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_base64_encode_simple() {
        let result = base64_encode(&[1, 2, 3]);
        assert_eq!(result, "AQID");
    }

    #[test]
    fn test_base64_encode_hello() {
        let result = base64_encode(b"Hello");
        assert_eq!(result, "SGVsbG8=");
    }

    #[test]
    fn test_base64_decode_empty() {
        let result = base64_decode("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_base64_decode_simple() {
        let result = base64_decode("AQID").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_base64_decode_hello() {
        let result = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_base64_decode_invalid() {
        let result = base64_decode("!!invalid!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip() {
        let original = b"This is a test string with various bytes: \x00\x01\x02\xff";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded.as_slice(), original.as_slice());
    }
}
