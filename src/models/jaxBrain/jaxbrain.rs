use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Deserialize)]
pub struct Overview {
    pub collections: Vec<CollectionMeta>,
}

#[derive(Deserialize, Clone)]
pub struct CollectionMeta {
    pub id: String,
    pub title: String,
    pub template: String, // "dictionary", "timeline", etc.
    pub file: String,
    pub tags: Vec<String>,
}

pub async fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} when fetching {}", resp.status(), url));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
