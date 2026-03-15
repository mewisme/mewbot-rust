use anyhow::{Context, Result};
use std::path::Path;

const DATA_DIR: &str = "data";

fn data_path(filename: &str) -> Result<std::path::PathBuf> {
    if filename.contains("..") {
        anyhow::bail!("Filename must not contain '..'");
    }
    Ok(Path::new(DATA_DIR).join(filename))
}

pub async fn ensure_data_dir() -> Result<()> {
    tokio::fs::create_dir_all(DATA_DIR)
        .await
        .context("create data dir")
}

pub async fn load(filename: &str) -> Result<String> {
    let path = data_path(filename)?;
    if !path.exists() {
        return Ok(String::new());
    }
    tokio::fs::read_to_string(&path).await.context("read data file")
}

pub async fn save(filename: &str, contents: &str) -> Result<()> {
    let path = data_path(filename)?;
    ensure_data_dir().await?;
    if let Some(parent) = path.parent() {
        if parent != Path::new(DATA_DIR) {
            tokio::fs::create_dir_all(parent)
                .await
                .context("create data subdir")?;
        }
    }
    tokio::fs::write(&path, contents).await.context("write data file")
}
