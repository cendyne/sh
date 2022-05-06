use once_cell::sync::OnceCell;
use warp::{Filter, Rejection};

static CORRECT_BEARER_TOKEN_HASH: OnceCell<[u8; blake3::OUT_LEN]> = OnceCell::new();

fn load_correct_bearer_token_hash() -> Result<[u8; blake3::OUT_LEN], String> {
    let input = std::env::var("TOKEN").map_err(|e| format!("TOKEN: {}", e))?;
    let correct_bearer = format!("Bearer {}", input);
    let mut hasher = blake3::Hasher::new();
    hasher.update(correct_bearer.as_bytes());
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    Ok(*hash_bytes)
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

#[derive(Debug)]
pub struct BadAuthorization(pub String);

impl warp::reject::Reject for BadAuthorization {}

pub fn check_auth() -> impl Filter<Extract = (), Error = Rejection> + Copy {
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
