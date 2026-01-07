use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    SerdeJson(serde_json::Error),
    InvalidPath(String),
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::SerdeJson(e)
    }
}

pub type StoreResult<T> = Result<T, StoreError>;

fn ensure_parent_dir(path: &Path) -> StoreResult<()> {
    let Some(parent) = path.parent() else { return Ok(()); };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)?;
    Ok(())
}

/// Load JSON from disk into any serde-deserializable type.
pub fn load_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> StoreResult<T> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(StoreError::InvalidPath("Empty path".into()));
    }
    let bytes = fs::read(path)?;
    let value = serde_json::from_slice::<T>(&bytes)?;
    Ok(value)
}

/// Save JSON to disk (pretty) with an atomic write pattern.
pub fn save_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> StoreResult<()> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(StoreError::InvalidPath("Empty path".into()));
    }

    ensure_parent_dir(path)?;

    let tmp_path: PathBuf = {
        let mut p = path.to_path_buf();
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        // file.json -> file.json.tmp
        let new_ext = if ext.is_empty() { "tmp".to_string() } else { format!("{ext}.tmp") };
        p.set_extension(new_ext);
        p
    };

    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)?; // atomic-ish on same filesystem
    Ok(())
}

/// Optional: map errors to strings for your `status: Signal<Option<String>>`
pub fn err_to_string(e: StoreError) -> String {
    match e {
        StoreError::Io(ioe) => format!("IO error: {ioe}"),
        StoreError::SerdeJson(se) => format!("JSON error: {se}"),
        StoreError::InvalidPath(s) => format!("Invalid path: {s}"),
    }
}
