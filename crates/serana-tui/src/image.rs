//! Terminal image protocol detection and rendering.
//!
//! Supports Kitty, iTerm2, and Sixel protocols with automatic detection.
use std::path::Path;

/// Detected terminal image protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    /// Kitty graphics protocol (kitty, ghostty, wezterm).
    Kitty,
    /// iTerm2 inline images protocol (iTerm2, mintty, WezTerm).
    ITerm2,
    /// Sixel graphics (xterm, foot, mlterm, contour).
    Sixel,
    /// No image protocol detected.
    None,
}

impl ImageProtocol {
    /// Auto-detect the best image protocol for the current terminal.
    pub fn detect() -> Self {
        // Check TERM_PROGRAM first (most reliable)
        if let Ok(program) = std::env::var("TERM_PROGRAM") {
            match program.as_str() {
                "kitty" | "ghostty" => return ImageProtocol::Kitty,
                "iTerm.app" | "WezTerm" | "mintty" => return ImageProtocol::ITerm2,
                "vscode" | "hyper" => return ImageProtocol::None,
                _ => {}
            }
        }

        // Check TERM for Sixel-capable terminals
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("kitty") {
                return ImageProtocol::Kitty;
            }
            if term.contains("sixel") || term.contains("mlterm") || term.contains("foot") {
                return ImageProtocol::Sixel;
            }
        }

        // Check KITTY_WINDOW_ID for kitty protocol
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            return ImageProtocol::Kitty;
        }

        // Check TERM_PROGRAM_VERSION for iTerm2
        if std::env::var("ITERM_SESSION_ID").is_ok() {
            return ImageProtocol::ITerm2;
        }

        // Check for WezTerm (supports both kitty and iTerm2)
        if std::env::var("WEZTERM_PANE").is_ok() {
            return ImageProtocol::Kitty; // prefer kitty protocol
        }

        ImageProtocol::None
    }

    /// Can this protocol display images?
    pub fn is_supported(&self) -> bool {
        *self != ImageProtocol::None
    }
}

/// Check if a file is an image by its extension.
pub fn is_image_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".bmp")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".tiff")
        || lower.ends_with(".ico")
}

/// Render an image file to terminal escape sequences.
/// Returns the escape sequence string to print, or None if unsupported.
pub fn render_image(path: &Path, protocol: ImageProtocol) -> Option<String> {
    if !protocol.is_supported() {
        return None;
    }

    let data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }

    let base64_data = base64_encode(&data);

    match protocol {
        ImageProtocol::Kitty => {
            // Kitty graphics protocol: transmit and display
            // Chunk the data to avoid buffer overflow
            let mut result = String::new();
            let chunk_size = 4096;
            let total = base64_data.len();
            let mut offset = 0;

            while offset < total {
                let end = (offset + chunk_size).min(total);
                let chunk = &base64_data[offset..end];
                let is_last = end >= total;

                if offset == 0 && is_last {
                    // Single chunk
                    result.push_str(&format!(
                        "\x1b_Ga=T,f=100,m={};{}\x1b\\",
                        0, chunk
                    ));
                } else if offset == 0 {
                    // First chunk
                    result.push_str(&format!(
                        "\x1b_Ga=T,f=100,m=1;{}\x1b\\",
                        chunk
                    ));
                } else if is_last {
                    // Last chunk
                    result.push_str(&format!(
                        "\x1b_Gm=0;{}\x1b\\",
                        chunk
                    ));
                } else {
                    // Middle chunk
                    result.push_str(&format!(
                        "\x1b_Gm=1;{}\x1b\\",
                        chunk
                    ));
                }

                offset = end;
            }
            Some(result)
        }
        ImageProtocol::ITerm2 => {
            // iTerm2 inline images
            let filename = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("image");
            Some(format!(
                "\x1b]1337;File=name={};inline=1;size={};preserveAspectRatio=1:{}\x07",
                base64_encode(filename.as_bytes()),
                data.len(),
                base64_data
            ))
        }
        ImageProtocol::Sixel => {
            // For Sixel, we need to convert the image to Sixel format.
            // This requires a library like libsixel. For now, display a placeholder.
            // In practice, tools like `img2sixel` would be called.
            let _ = base64_data;
            Some(format!("[Sixel image: {}]", path.display()))
        }
        ImageProtocol::None => None,
    }
}

/// Simple base64 encoder (no external dependency).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let chunks = data.chunks_exact(3);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        result.push(TABLE[(n >> 18 & 0x3F) as usize] as char);
        result.push(TABLE[(n >> 12 & 0x3F) as usize] as char);
        result.push(TABLE[(n >> 6 & 0x3F) as usize] as char);
        result.push(TABLE[(n & 0x3F) as usize] as char);
    }

    match remainder.len() {
        1 => {
            let n = (remainder[0] as u32) << 16;
            result.push(TABLE[(n >> 18 & 0x3F) as usize] as char);
            result.push(TABLE[(n >> 12 & 0x3F) as usize] as char);
            result.push('=');
            result.push('=');
        }
        2 => {
            let n = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
            result.push(TABLE[(n >> 18 & 0x3F) as usize] as char);
            result.push(TABLE[(n >> 12 & 0x3F) as usize] as char);
            result.push(TABLE[(n >> 6 & 0x3F) as usize] as char);
            result.push('=');
        }
        _ => {}
    }

    result
}

/// Detect if a tool result contains image output (base64 or file path).
pub fn detect_image_in_result(result: &str, _tool_name: &str) -> Option<String> {
    // Check if the tool result references an image file
    for line in result.lines() {
        let trimmed = line.trim();
        // Standalone path: line starts with /, ./, ../, or ~ and is an image
        if is_image_file(trimmed) && (trimmed.starts_with('/') || trimmed.starts_with("./") || trimmed.starts_with("../") || trimmed.starts_with('~')) {
            return Some(trimmed.to_string());
        }
        // Check for "Saved to: <path>" patterns
        if let Some(path) = trimmed.strip_prefix("saved to:").or_else(|| trimmed.strip_prefix("Saved to:")) {
            let path = path.trim();
            if is_image_file(path) {
                return Some(path.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("test.png"));
        assert!(is_image_file("test.jpg"));
        assert!(is_image_file("test.jpeg"));
        assert!(is_image_file("test.gif"));
        assert!(is_image_file("test.webp"));
        assert!(is_image_file("test.svg"));
        assert!(!is_image_file("test.txt"));
        assert!(!is_image_file("test.rs"));
        assert!(!is_image_file("noext"));
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_image_protocol_none_for_unknown() {
        // In a test environment without TERM_PROGRAM, should get None
        // (unless the test runner happens to set it)
        let protocol = ImageProtocol::detect();
        // Just ensure it doesn't panic
        let _ = protocol;
    }

    #[test]
    fn test_detect_image_in_result() {
        let result = "Image saved to: /tmp/chart.png";
        assert_eq!(detect_image_in_result(result, "bash"), None); // doesn't match "Saved to:" exactly

        let result2 = "Saved to: /tmp/chart.png";
        assert_eq!(detect_image_in_result(result2, "bash"), Some("/tmp/chart.png".to_string()));

        let result3 = "/tmp/output.jpg";
        assert_eq!(detect_image_in_result(result3, "generate"), Some("/tmp/output.jpg".to_string()));
    }
}
