use dashmap::DashMap;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use tokio::fs::{read_to_string, write};

static SETTINGS: OnceCell<Arc<JsonSettings>> = OnceCell::new();
static DATA_SOURCE: OnceCell<String> = OnceCell::new();

pub fn data_source() -> &'static str {
    DATA_SOURCE.get_or_init(|| {
        env::var("DATA")
            .ok()
            .unwrap_or_else(|| "data.json".to_string())
    })
}

pub async fn load_json_settings() -> Arc<JsonSettings> {
    match SETTINGS.get() {
        None => {
            let data = load_data(data_source()).await;
            match SETTINGS.try_insert(data.clone()) {
                Ok(_) => data,
                Err((actual, _)) => actual.clone(),
            }
        }
        Some(data) => data.clone(),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonSettings {
    pub redirects: DashMap<String, String>,
    pub last_symbol: Mutex<String>,
}

async fn load_data_result(source: &str) -> Result<JsonSettings, String> {
    let contents = read_to_string(source).await.map_err(|e| format!("{}", e))?;
    let deserialized: JsonSettings =
        serde_json::from_str(&contents).map_err(|e| format!("{}", e))?;
    Ok(deserialized)
}

pub async fn load_data(source: &str) -> Arc<JsonSettings> {
    match load_data_result(source).await {
        Ok(settings) => Arc::new(settings),
        Err(e) => {
            println!("Could not load settings: {}", e);
            let redirects: DashMap<String, String> = DashMap::with_capacity(80);
            let json = JsonSettings {
                redirects,
                last_symbol: Mutex::new("".to_string()),
            };

            Arc::new(json)
        }
    }
}

pub async fn save_data(settings: &JsonSettings, source: &str) -> Result<(), String> {
    let serialized = serde_json::to_string(&settings).map_err(|e| format!("{}", e))?;
    write(source, serialized)
        .await
        .map_err(|e| format!("{}", e))?;
    Ok(())
}

pub fn add_redirect(settings: &JsonSettings, key: &str, value: &str) {
    println!("Adding redirect from {} to {}", key, value);
    settings
        .redirects
        .insert(key.to_string(), value.to_string());
}
