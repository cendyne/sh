use dashmap::DashMap;
use http::{Response, StatusCode};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::fs::{read_to_string, write};
use warp::{Filter, Rejection, Reply};

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

static CORRECT_BEARER_TOKEN_HASH: OnceCell<[u8; blake3::OUT_LEN]> = OnceCell::new();
static CHOSEN_ALPHABET: OnceCell<String> = OnceCell::new();
static DATA_SOURCE: OnceCell<String> = OnceCell::new();
static SETTINGS: OnceCell<Arc<JsonSettings>> = OnceCell::new();

fn load_correct_bearer_token_hash() -> Result<[u8; blake3::OUT_LEN], String> {
    let input = std::env::var("TOKEN").map_err(|e| format!("{}", e))?;
    let correct_bearer = format!("Bearer {}", input);
    let mut hasher = blake3::Hasher::new();
    hasher.update(correct_bearer.as_bytes());
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    Ok(*hash_bytes)
}

fn chosen_alphabet() -> &'static str {
    CHOSEN_ALPHABET.get_or_init(|| {
        env::var("ALPHABET")
            .ok()
            .unwrap_or_else(|| ALPHABET.to_string())
    })
}

fn data_source() -> &'static str {
    DATA_SOURCE.get_or_init(|| {
        env::var("DATA")
            .ok()
            .unwrap_or_else(|| "data.json".to_string())
    })
}

async fn load_json_settings() -> Arc<JsonSettings> {
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
            let json = JsonSettings {
                redirects,
                last_symbol: Mutex::new("".to_string()),
            };

            Arc::new(json)
        }
    }
}

async fn save_data(settings: &JsonSettings, source: &str) -> Result<(), String> {
    let serialized = serde_json::to_string(&settings).map_err(|e| format!("{}", e))?;
    write(source, serialized)
        .await
        .map_err(|e| format!("{}", e))?;
    Ok(())
}

async fn redirect_handler(name: String) -> http::Result<impl Reply> {
    let settings = load_json_settings().await;
    let found_path = settings.redirects.get(&name);
    // Cannot embed redirects get into the if let below due to borrow lifetimes
    if let Some(path) = found_path {
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

async fn empty_redirect_handler() -> http::Result<impl Reply> {
    redirect_handler("".to_string()).await
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

fn check_authorization(authorization: String) -> Result<(), String> {
    let correct_hash = CORRECT_BEARER_TOKEN_HASH.get_or_try_init(load_correct_bearer_token_hash)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(authorization.as_bytes());
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    if constant_time_eq::constant_time_eq_32(correct_hash, hash_bytes) {
        Ok(())
    } else {
        Err("Authorization failed".to_string())
    }
}

async fn push_handler(destination: String) -> Result<impl Reply, Infallible> {
    let settings = load_json_settings().await;
    let alphabet = chosen_alphabet();
    let symbol = next_symbol(&settings, alphabet);
    if let Some(key) = symbol {
        add_redirect(&settings, &key, &destination);
        if save_data(&settings, data_source()).await.is_ok() {
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
}

async fn push_custom_handler(path: String, destination: String) -> Result<impl Reply, Infallible> {
    let settings = load_json_settings().await;
    add_redirect(&settings, &path, &destination);
    if save_data(&settings, data_source()).await.is_ok() {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(format!("/{}", path)))
    } else {
        Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Could not save".to_string()))
    }
}

#[derive(Debug)]
struct CouldNotParsePlainText;

impl warp::reject::Reject for CouldNotParsePlainText {}

#[derive(Debug)]
struct BadAuthorization(String);

impl warp::reject::Reject for BadAuthorization {}

pub fn plaintext() -> impl Filter<Extract = (String,), Error = Rejection> + Copy {
    warp::filters::body::bytes().and_then(|buf: bytes::Bytes| async move {
        String::from_utf8(buf.to_vec()).map_err(|_| warp::reject::custom(CouldNotParsePlainText))
    })
}

fn check_auth() -> impl Filter<Extract = (), Error = Rejection> + Copy {
    warp::any()
        .and(warp::header::<String>("authorization"))
        .and_then(|auth| async move {
            if let Err(message) = check_authorization(auth) {
                Err(warp::reject::custom(BadAuthorization(message)))
            } else {
                Ok(())
            }
        })
        .untuple_one()
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() {
        Ok(warp::reply::with_status(
            "NOT FOUND".to_string(),
            StatusCode::NOT_FOUND,
        ))
    } else if let Some(BadAuthorization(message)) = err.find() {
        Ok(warp::reply::with_status(
            message.clone(),
            StatusCode::UNAUTHORIZED,
        ))
    } else if let Some(CouldNotParsePlainText) = err.find() {
        Ok(warp::reply::with_status(
            "Input does not appear to be utf8".to_string(),
            StatusCode::BAD_REQUEST,
        ))
    } else {
        Err(err)
    }
}

#[tokio::main]
async fn main() {
    match dotenv::dotenv() {
        Ok(_) => {}
        Err(e) => {
            format!("{}", e);
        }
    }

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    load_json_settings().await;

    let redirect = warp::get()
        .and(warp::path!(String))
        .then(redirect_handler)
        .or(warp::get().then(empty_redirect_handler));
    let push = warp::put()
        .and(warp::path::end())
        .and(warp::body::content_length_limit(4096))
        .and(check_auth())
        .and(plaintext())
        .and_then(push_handler);
    let push_custom = warp::put()
        .and(warp::path!("custom" / String))
        .and(warp::body::content_length_limit(4096))
        .and(check_auth())
        .and(plaintext())
        .and_then(push_custom_handler);

    let filters = redirect.or(push).or(push_custom).recover(handle_rejection);

    println!("Listening on port {}", port);
    warp::serve(filters).run(([0, 0, 0, 0], port)).await;
}

// TODO handle rejections

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
