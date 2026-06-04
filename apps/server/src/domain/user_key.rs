use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscordUserKey(String);

impl DiscordUserKey {
    pub fn derive(secret: &[u8], discord_user_id: &str) -> Self {
        let mut mac =
            HmacSha256::new_from_slice(secret).expect("HMAC accepts secrets of any length");
        mac.update(discord_user_id.as_bytes());
        let result = mac.finalize().into_bytes();
        Self(hex::encode(result))
    }

    pub fn as_hex(&self) -> &str {
        &self.0
    }
}
