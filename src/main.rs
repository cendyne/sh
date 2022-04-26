use http::{Response, StatusCode};
use once_cell::sync::OnceCell;

use std::convert::Infallible;
use std::env;

use warp::{Filter, Rejection, Reply};

mod auth;
mod id;
mod settings;

use crate::auth::*;
use crate::id::*;
use crate::settings::*;

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
static CHOSEN_ALPHABET: OnceCell<String> = OnceCell::new();

fn chosen_alphabet() -> &'static str {
    CHOSEN_ALPHABET.get_or_init(|| {
        env::var("ALPHABET")
            .ok()
            .unwrap_or_else(|| ALPHABET.to_string())
    })
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

pub fn plaintext() -> impl Filter<Extract = (String,), Error = Rejection> + Copy {
    warp::filters::body::bytes().and_then(|buf: bytes::Bytes| async move {
        String::from_utf8(buf.to_vec()).map_err(|_| warp::reject::custom(CouldNotParsePlainText))
    })
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
