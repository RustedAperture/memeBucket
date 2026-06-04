use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

pub fn hash_csrf_token(session_secret: &str, token: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(session_secret.as_bytes())
        .expect("HMAC accepts secrets of any length");
    mac.update(token.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_csrf_token(session_secret: &str, token: &str, expected_hash: &str) -> bool {
    let Ok(expected_bytes) = hex::decode(expected_hash) else {
        return false;
    };

    let mut mac = HmacSha256::new_from_slice(session_secret.as_bytes())
        .expect("HMAC accepts secrets of any length");
    mac.update(token.as_bytes());
    mac.verify_slice(&expected_bytes).is_ok()
}
