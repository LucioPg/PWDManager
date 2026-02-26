//! Avatar utilities for user profile images.
//!
//! These functions handle avatar loading, scaling, and formatting.
//! They remain in the PWDManager project (not extracted to pwd-crypto).

use pwd_crypto::base64_encode;
use custom_errors::GeneralError;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// Returns user avatar with fallback to default.
///
/// If `avatar_from_db` is `None` or empty, returns the default avatar.
pub fn get_user_avatar_with_default(avatar_from_db: Option<Vec<u8>>) -> String {
    let avatar: Vec<u8> = match avatar_from_db {
        Some(avatar_) if !avatar_.is_empty() => avatar_,
        _ => include_bytes!("../../assets/default_avatar.png").to_vec(),
    };
    format_avatar_url(base64_encode(&avatar))
}

/// Formats avatar bytes as data URL.
pub fn format_avatar_url(avatar_b64: String) -> String {
    format!("data:image/png;base64,{}", avatar_b64)
}

/// Scales avatar to 128x128 pixels.
pub fn scale_avatar(bytes: &[u8]) -> Result<Vec<u8>, GeneralError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| GeneralError::new_scaling_error(e.to_string()))?;
    image_to_vec(&img.thumbnail(128, 128))
}

fn image_to_vec(img: &DynamicImage) -> Result<Vec<u8>, GeneralError> {
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| GeneralError::new_encode_error(e.to_string()))?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_expected_default_avatar() -> String {
        let default_bytes = include_bytes!("../../assets/default_avatar.png");
        format!("data:image/png;base64,{}", base64_encode(default_bytes))
    }

    #[test]
    fn test_avatar_present() {
        let data = Some(vec![1, 2, 3]);
        let result = get_user_avatar_with_default(data);
        assert_eq!(result, "data:image/png;base64,AQID");
    }

    #[test]
    fn test_avatar_empty() {
        let data = Some(vec![]);
        let result = get_user_avatar_with_default(data);
        let expected = get_expected_default_avatar();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_avatar_none() {
        let result = get_user_avatar_with_default(None);
        let expected = get_expected_default_avatar();
        assert_eq!(result, expected);
    }
}
