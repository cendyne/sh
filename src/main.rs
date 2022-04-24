use dashmap::DashMap;
use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::convert::Infallible;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::fs::{read_to_string, write};
use warp::{Filter, Rejection, Reply};

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn increment_string(alphabet: &str, symbols: &str) -> String {
    let mut result: Vec<char> = Vec::with_capacity(symbols.len() + 1);
    let mut carry = 1;
    let length = alphabet.len();
    symbols.chars().rev().for_each(|c| {
        if carry == 0 {
            result.push(c);
        } else if let Some(value) = alphabet.find(c) {
            if value == length - 1 {
                if let Some(first) = alphabet.chars().next() {
                    result.push(first);
                }
            } else if let Some(next) = alphabet.chars().nth(value + 1) {
                    result.push(next);
                    carry = 0;
            }
        } else {
            println!("Unsupported character {}, using last", c);
            if let Some(last) = alphabet.chars().last() {
                result.push(last);
            }
        }
    });

    if carry != 0 {
        if let Some(first) = alphabet.chars().next() {
            result.push(first);
        }
    }

    result.reverse();
    result.iter().collect()
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonSettings {
    redirects: DashMap<String, String>,
    last_symbol: Mutex<String>,
}

async fn load_data_result(source: &str) -> Result<JsonSettings, String> {
    let contents = read_to_string(source).await.map_err(|e| format!("{}", e))?;
    let deserialized: JsonSettings =
        serde_json::from_str(&contents).map_err(|e| format!("{}", e))?;
    Ok(deserialized)
}

async fn load_data(source: &str) -> Arc<JsonSettings> {
    match load_data_result(source).await {
        Ok(settings) => Arc::new(settings),
        Err(e) => {
            println!("Could not load settings: {}", e);
            let redirects: DashMap<String, String> = DashMap::with_capacity(80);
            redirects.insert("".to_string(), "https://cendyne.dev/".to_string());
            let json = JsonSettings {
                redirects,
                last_symbol: Mutex::new("".to_string()),
            };

            Arc::new(json)
        }
    }
}

fn with_settings(
    settings: Arc<JsonSettings>,
) -> impl Filter<Extract = (Arc<JsonSettings>,), Error = Infallible> + Clone {
    warp::any().map(move || settings.clone())
}

fn with_alphabet(
    alphabet: Cow<String>,
) -> impl Filter<Extract = (Cow<String>,), Error = Infallible> + Clone {
    warp::any().map(move || alphabet.clone())
}

fn with_source(
    source: Cow<String>,
) -> impl Filter<Extract = (Cow<String>,), Error = Infallible> + Clone {
    warp::any().map(move || source.clone())
}

async fn save_data(settings: &JsonSettings, source: &str) -> Result<(), String> {
    let serialized = serde_json::to_string(&settings).map_err(|e| format!("{}", e))?;
    write(source, serialized)
        .await
        .map_err(|e| format!("{}", e))?;
    Ok(())
}

fn redirect_handler(name: String, settings: Arc<JsonSettings>) -> http::Result<impl Reply> {
    if let Some(path) = settings.redirects.get(&name) {
        Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", path.clone())
            .body(format!("Go to {}", *path))
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("".to_string())
    }
}

fn empty_redirect_handler(settings: Arc<JsonSettings>) -> http::Result<impl Reply> {
    redirect_handler("".to_string(), settings)
}

fn next_symbol(settings: &JsonSettings, alphabet: &str) -> Option<String> {
    settings
        .last_symbol
        .lock()
        .map(|mut symbol| {
            let mut last = (*symbol).clone();

            loop {
                let next_symbol = increment_string(alphabet, &last);

                if settings.redirects.contains_key(&next_symbol) {
                    last = next_symbol;
                    continue;
                }

                *symbol = next_symbol.clone();
                break next_symbol;
            }
        })
        .ok()
}

fn add_redirect(settings: &JsonSettings, key: &str, value: &str) {
    println!("Adding redirect from {} to {}", key, value);
    settings
        .redirects
        .insert(key.to_string(), value.to_string());
}

async fn push_handler(
    settings: Arc<JsonSettings>,
    authorization: String,
    alphabet: Cow<'_, String>,
    destination: String,
    source: Cow<'_, String>,
) -> Result<impl Reply, Infallible> {
    if let Ok(token) = env::var("TOKEN") {
        // This isn't great
        if format!("Bearer {}", token) == authorization {
            // Separate the mutex from the rest of the processing
            let symbol = next_symbol(&settings, &alphabet);
            if let Some(key) = symbol {
                add_redirect(&settings, &key, &destination);
                if save_data(&settings, &source).await.is_ok() {
                    Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(format!("/{}", key)))
                } else {
                    Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Could not save".to_string()))
                }
            } else {
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Server is poisoned".to_string()))
            }
        } else {
            Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Bad Authentication".to_string()))
        }
    } else {
        Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("TOKEN not set".to_string()))
    }
}

#[derive(Debug)]
struct CouldNotParsePlainText;

impl warp::reject::Reject for CouldNotParsePlainText {}

pub fn plaintext() -> impl Filter<Extract = (String,), Error = Rejection> + Copy {
    warp::filters::body::bytes().and_then(|buf: bytes:: Bytes| async move {
        String::from_utf8(buf.to_vec()).map_err(|_| {
            warp::reject::custom(CouldNotParsePlainText)
        })
    })
}

#[tokio::main]
async fn main() {
    match dotenv::dotenv() {
        Ok(_) => {}
        Err(e) => {
            format!("{}", e);
        }
    }

    let alphabet: String = env::var("ALPHABET")
        .ok()
        .unwrap_or_else(|| ALPHABET.to_string());
    let owned_alphabet = Cow::Owned(alphabet);
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let data_source = env::var("DATA")
        .ok()
        .unwrap_or_else(|| "data.json".to_string());
    let data = load_data(&data_source).await;
    let owned_data_source = Cow::Owned(data_source);

    let redirect = warp::get()
        .and(
            warp::path!(String)
                .and(with_settings(data.clone()))
                .map(redirect_handler),
        )
        .or(warp::get()
            .and(with_settings(data.clone()))
            .map(empty_redirect_handler));
    let authorization = warp::header::<String>("authorization");
    let push = warp::put().and(warp::path::end())
        .and(warp::body::content_length_limit(4096)
                .and(with_settings(data.clone()))
                .and(authorization)
                .and(with_alphabet(owned_alphabet))
                .and(plaintext())
                .and(with_source(owned_data_source))
                .and_then(push_handler),
        );

    let filters = redirect.or(push);

    println!("Listening on port {}", port);
    warp::serve(filters).run(([0, 0, 0, 0], port)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    const NUMBERS: &'static str = "0123456789";
    #[test]
    fn increment_none() {
        assert_eq!(increment_string(NUMBERS, ""), "0");
    }

    #[test]
    fn increment_some() {
        assert_eq!(increment_string(NUMBERS, "0"), "1");
        assert_eq!(increment_string(NUMBERS, "1"), "2");
        assert_eq!(increment_string(NUMBERS, "8"), "9");
    }

    #[test]
    fn increment_two_digits() {
        assert_eq!(increment_string(NUMBERS, "9"), "00");
        assert_eq!(increment_string(NUMBERS, "00"), "01");
        assert_eq!(increment_string(NUMBERS, "09"), "10");
        assert_eq!(increment_string(NUMBERS, "89"), "90");
        assert_eq!(increment_string(NUMBERS, "98"), "99");
    }

    #[test]
    fn increment_three_digits() {
        assert_eq!(increment_string(NUMBERS, "99"), "000");
        assert_eq!(increment_string(NUMBERS, "000"), "001");
        assert_eq!(increment_string(NUMBERS, "099"), "100");
        assert_eq!(increment_string(NUMBERS, "899"), "900");
        assert_eq!(increment_string(NUMBERS, "998"), "999");
    }

    #[test]
    fn next_symbol_test1() {
        let settings = JsonSettings {
            redirects: DashMap::new(),
            last_symbol: Mutex::new("".to_string()),
        };
        assert_eq!(next_symbol(&settings, NUMBERS), Some("0".to_string()));
        settings.redirects.insert("1".to_string(), "".to_string());
        // It does not overwrite
        assert_eq!(next_symbol(&settings, NUMBERS), Some("2".to_string()));
    }
    #[test]
    fn next_symbol_test2() {
        let settings = JsonSettings {
            redirects: DashMap::new(),
            last_symbol: Mutex::new("".to_string()),
        };
        settings.redirects.insert("0".to_string(), "".to_string());
        settings.redirects.insert("1".to_string(), "".to_string());
        settings.redirects.insert("2".to_string(), "".to_string());
        // It does not overwrite
        assert_eq!(next_symbol(&settings, NUMBERS), Some("3".to_string()));
        // Even if not written, it won't go back
        assert_eq!(next_symbol(&settings, NUMBERS), Some("4".to_string()));
    }
}
