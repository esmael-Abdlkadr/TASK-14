use sha2::{Sha256, Digest};
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::db::knowledge_base as kb_db;
use crate::errors::AppError;
use crate::models::KbImage;

const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024; // 5 MB
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png"];
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];
const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47];

/// Validate image bytes: check magic bytes, size, and mime type.
pub fn validate_image(data: &[u8], claimed_mime: &str) -> Result<String, AppError> {
    // Size check
    if data.len() > MAX_IMAGE_SIZE {
        return Err(AppError::BadRequest(format!(
            "Image exceeds maximum size of {} MB",
            MAX_IMAGE_SIZE / (1024 * 1024)
        )));
    }

    if data.len() < 4 {
        return Err(AppError::BadRequest("File too small to be a valid image".to_string()));
    }

    // Detect actual type from magic bytes
    let detected_mime = if data.starts_with(JPEG_MAGIC) {
        "image/jpeg"
    } else if data.starts_with(PNG_MAGIC) {
        "image/png"
    } else {
        return Err(AppError::BadRequest(
            "Invalid image format. Only JPEG and PNG are supported".to_string(),
        ));
    };

    // Verify claimed mime matches detected
    if !ALLOWED_MIME_TYPES.contains(&claimed_mime) {
        return Err(AppError::BadRequest(format!(
            "Unsupported image type '{}'. Only JPEG and PNG are allowed",
            claimed_mime
        )));
    }

    if claimed_mime != detected_mime {
        return Err(AppError::BadRequest(format!(
            "Image content does not match claimed type '{}' (detected '{}')",
            claimed_mime, detected_mime
        )));
    }

    Ok(detected_mime.to_string())
}

/// Compute SHA-256 fingerprint of image data for deduplication
pub fn compute_fingerprint(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Get the storage directory for images
fn get_storage_dir() -> PathBuf {
    let dir = std::env::var("CIVICSORT_IMAGE_DIR")
        .unwrap_or_else(|_| "./data/images".to_string());
    PathBuf::from(dir)
}

/// Store an image on disk and record in DB. Returns existing image if duplicate.
pub async fn store_image(
    pool: &PgPool,
    file_name: &str,
    data: &[u8],
    claimed_mime: &str,
    uploaded_by: Option<Uuid>,
) -> Result<KbImage, AppError> {
    // Validate
    let mime_type = validate_image(data, claimed_mime)?;

    // Compute fingerprint
    let fingerprint = compute_fingerprint(data);

    // Check for duplicate
    if let Some(existing) = kb_db::find_image_by_hash(pool, &fingerprint)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
    {
        log::info!(
            "Duplicate image detected (hash={}), reusing image {}",
            fingerprint,
            existing.id
        );
        return Ok(existing);
    }

    // Determine file extension from mime
    let ext = match mime_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        _ => "bin",
    };

    // Generate unique file path: {storage_dir}/{first2hash}/{hash}.{ext}
    let subdir = &fingerprint[..2];
    let storage_dir = get_storage_dir();
    let dir_path = storage_dir.join(subdir);

    tokio::fs::create_dir_all(&dir_path)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to create image directory: {}", e)))?;

    let file_path = dir_path.join(format!("{}.{}", fingerprint, ext));
    let file_path_str = file_path.to_string_lossy().to_string();

    // Write file
    tokio::fs::write(&file_path, data)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to write image file: {}", e)))?;

    // Insert DB record
    let image = kb_db::insert_image(
        pool,
        file_name,
        &file_path_str,
        data.len() as i64,
        &mime_type,
        &fingerprint,
        None, // width - could parse from image headers
        None, // height
        uploaded_by,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(image)
}

/// Read an image from disk by its DB record
pub async fn read_image(pool: &PgPool, image_id: Uuid) -> Result<(Vec<u8>, String), AppError> {
    let image = kb_db::get_image(pool, image_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Image not found".to_string()))?;

    let data = tokio::fs::read(&image.file_path)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to read image file: {}", e)))?;

    Ok((data, image.mime_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_jpeg() {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        data.extend_from_slice(&[0u8; 100]);
        assert!(validate_image(&data, "image/jpeg").is_ok());
    }

    #[test]
    fn test_validate_png() {
        let mut data = vec![0x89, 0x50, 0x4E, 0x47];
        data.extend_from_slice(&[0u8; 100]);
        assert!(validate_image(&data, "image/png").is_ok());
    }

    #[test]
    fn test_reject_wrong_mime() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0];
        assert!(validate_image(&data, "image/png").is_err());
    }

    #[test]
    fn test_reject_too_large() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let mut large = data;
        large.extend_from_slice(&vec![0u8; MAX_IMAGE_SIZE + 1]);
        assert!(validate_image(&large, "image/jpeg").is_err());
    }

    #[test]
    fn test_reject_invalid_magic() {
        let data = vec![0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(validate_image(&data, "image/jpeg").is_err());
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let data = b"test image data";
        let h1 = compute_fingerprint(data);
        let h2 = compute_fingerprint(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fingerprint_different() {
        let h1 = compute_fingerprint(b"image A");
        let h2 = compute_fingerprint(b"image B");
        assert_ne!(h1, h2);
    }
}
