use warp::{Filter, Rejection};

#[derive(Debug)]
pub struct CouldNotParsePlainText;

impl warp::reject::Reject for CouldNotParsePlainText {}

pub fn plaintext() -> impl Filter<Extract = (String,), Error = Rejection> + Copy {
    warp::filters::body::bytes().and_then(|buf: bytes::Bytes| async move {
        String::from_utf8(buf.to_vec()).map_err(|_| warp::reject::custom(CouldNotParsePlainText))
    })
}
