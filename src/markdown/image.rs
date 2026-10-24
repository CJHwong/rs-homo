use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use log::warn;
use std::path::{Path, PathBuf};

/// Directory that relative image paths resolve against: the folder of the
/// opened file, or the working directory for piped input.
pub fn base_dir_for(file_path: Option<&str>) -> PathBuf {
    file_path
        .and_then(|path| Path::new(path).parent())
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn mime_for(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

/// Map a local image reference to a `data:` URI.
///
/// The WebView loads the page from a string with no base URL, so it can
/// neither resolve relative paths nor read `file://`. Embedding the bytes
/// is the only way a local image reaches the page.
///
/// Returns `None` for anything that is not a readable local image (remote
/// URLs, existing `data:` URIs, missing files); the caller keeps the
/// original `src` and the WebView falls back to the alt text.
pub fn embed_local_image(src: &str, base_dir: &Path) -> Option<String> {
    if src.contains("://") || src.starts_with("data:") || src.starts_with('#') {
        return None;
    }
    let raw_path = src.split(['?', '#']).next().unwrap_or(src);
    let path = Path::new(raw_path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };
    let mime = mime_for(&resolved)?;
    match std::fs::read(&resolved) {
        Ok(bytes) => Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes))),
        Err(error) => {
            warn!("Cannot read image {}: {error}", resolved.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_dir_for_file_uses_parent() {
        assert_eq!(base_dir_for(Some("docs/a.md")), PathBuf::from("docs"));
    }

    #[test]
    fn test_base_dir_for_bare_file_or_pipe_uses_cwd() {
        assert_eq!(base_dir_for(Some("a.md")), PathBuf::from("."));
        assert_eq!(base_dir_for(None), PathBuf::from("."));
    }

    #[test]
    fn test_remote_and_data_urls_are_left_alone() {
        let dir = Path::new(".");
        assert_eq!(embed_local_image("https://x.com/a.png", dir), None);
        assert_eq!(embed_local_image("data:image/png;base64,AA==", dir), None);
    }

    #[test]
    fn test_non_image_extension_is_not_read() {
        assert_eq!(embed_local_image("Cargo.toml", Path::new(".")), None);
    }
}
