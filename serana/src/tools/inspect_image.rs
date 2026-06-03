//! Image inspection tool — extracts metadata and basic info from image files.
//!
//! Supports PNG, JPEG, GIF, BMP, WebP header parsing without external dependencies.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::{Result, Tool};

/// Tool that inspects image files and extracts metadata.
pub struct InspectImageTool;

#[async_trait]
impl Tool for InspectImageTool {
    fn name(&self) -> &'static str {
        "inspect_image"
    }

    fn description(&self) -> &'static str {
        "Inspect an image file to extract metadata (dimensions, format, file size). Input: {\"path\": \"/path/to/image.png\"}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the image file"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;

        let data = tokio::fs::read(path).await?;
        let file_size = data.len();

        if data.len() < 8 {
            return Err(anyhow::anyhow!("File too small to be a valid image"));
        }

        let (format, width, height) = detect_image_info(&data)?;

        Ok(json!({
            "path": path,
            "format": format,
            "width": width,
            "height": height,
            "file_size_bytes": file_size,
            "file_size_human": human_size(file_size),
        }))
    }
}

/// Detect image format and dimensions from raw bytes.
fn detect_image_info(data: &[u8]) -> Result<(String, u64, u64)> {
    // PNG: 8-byte signature, IHDR chunk at offset 8
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        if data.len() >= 24 {
            let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as u64;
            let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]) as u64;
            return Ok(("PNG".into(), w, h));
        }
        return Ok(("PNG".into(), 0, 0));
    }

    // JPEG: starts with FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let (w, h) = parse_jpeg_dimensions(data);
        return Ok(("JPEG".into(), w, h));
    }

    // GIF: starts with "GIF87a" or "GIF89a"
    if data.starts_with(b"GIF8") && data.len() >= 10 {
        let w = u16::from_le_bytes([data[6], data[7]]) as u64;
        let h = u16::from_le_bytes([data[8], data[9]]) as u64;
        return Ok(("GIF".into(), w, h));
    }

    // BMP: starts with "BM"
    if data.starts_with(b"BM") && data.len() >= 26 {
        let w = u32::from_le_bytes([data[18], data[19], data[20], data[21]]) as u64;
        let h = u32::from_le_bytes(
            [data[22], data[23], data[24], data[25]]
                .try_into()
                .unwrap_or([0; 4]),
        ) as u64;
        // BMP height can be negative (top-down), take absolute
        let h = if h > 0x7FFFFFFF { !h + 1 } else { h };
        return Ok(("BMP".into(), w, h));
    }

    // WebP: starts with "RIFF" + size + "WEBP"
    if data.starts_with(b"RIFF") && data.len() >= 16 && &data[8..12] == b"WEBP" {
        // VP8 chunk
        if data.len() >= 30 && &data[12..16] == b"VP8 " {
            let w = u16::from_le_bytes([data[26], data[27]]) as u64;
            let h = u16::from_le_bytes([data[28], data[29]]) as u64;
            return Ok(("WebP".into(), w, h));
        }
        // VP8L (lossless)
        if data.len() >= 25 && &data[12..16] == b"VP8L" {
            let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
            let w = ((bits & 0x3FFF) + 1) as u64;
            let h = (((bits >> 14) & 0x3FFF) + 1) as u64;
            return Ok(("WebP".into(), w, h));
        }
        return Ok(("WebP".into(), 0, 0));
    }

    Err(anyhow::anyhow!("Unsupported or unrecognized image format"))
}

/// Parse JPEG dimensions by scanning for SOF marker.
fn parse_jpeg_dimensions(data: &[u8]) -> (u64, u64) {
    let mut i = 2; // skip FF D8
    while i + 4 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        // SOF markers: C0-C3, C5-C7, C9-CB, CD-CF
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 {
            if i + 9 < data.len() {
                let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u64;
                let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u64;
                return (w, h);
            }
        }
        // Skip to next marker
        if i + 3 < data.len() {
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + len;
        } else {
            break;
        }
    }
    (0, 0)
}

fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_png() {
        let mut data = vec![0u8; 24];
        data[0..8].copy_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        // Width = 1920 (0x00000780), Height = 1080 (0x00000438)
        data[16..20].copy_from_slice(&[0x00, 0x00, 0x07, 0x80]);
        data[20..24].copy_from_slice(&[0x00, 0x00, 0x04, 0x38]);
        let (fmt, w, h) = detect_image_info(&data).unwrap();
        assert_eq!(fmt, "PNG");
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
    }

    #[test]
    fn detect_gif() {
        let mut data = vec![0u8; 10];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..8].copy_from_slice(&640u16.to_le_bytes());
        data[8..10].copy_from_slice(&480u16.to_le_bytes());
        let (fmt, w, h) = detect_image_info(&data).unwrap();
        assert_eq!(fmt, "GIF");
        assert_eq!(w, 640);
        assert_eq!(h, 480);
    }

    #[test]
    fn human_size_formatting() {
        assert_eq!(human_size(500), "500 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn too_small() {
        assert!(detect_image_info(&[0, 1, 2]).is_err());
    }
}
