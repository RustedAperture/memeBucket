use axum::{body::Bytes, http::HeaderMap};

const MAX_SIGNATURE_AGE_SECONDS: u64 = 300;

pub fn verify_interaction_signature(
    headers: &HeaderMap,
    body: &Bytes,
    public_key_hex: &str,
) -> bool {
    let Some(signature) = headers
        .get("x-signature-ed25519")
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(timestamp) = headers
        .get("x-signature-timestamp")
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    if !timestamp_is_fresh(timestamp) {
        return false;
    }
    let Ok(public_key_bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(signature_bytes) = hex::decode(signature) else {
        return false;
    };
    let Ok(verifying_key_bytes) = <[u8; 32]>::try_from(public_key_bytes.as_slice()) else {
        return false;
    };
    let Ok(verifying_key) = ed25519_dalek::VerifyingKey::from_bytes(&verifying_key_bytes) else {
        return false;
    };
    let Ok(discord_signature) = ed25519_dalek::Signature::from_slice(&signature_bytes) else {
        return false;
    };

    let mut message = timestamp.as_bytes().to_vec();
    message.extend_from_slice(body);

    ed25519_dalek::Verifier::verify(&verifying_key, &message, &discord_signature).is_ok()
}

fn timestamp_is_fresh(timestamp: &str) -> bool {
    let Ok(timestamp) = timestamp.parse::<u64>() else {
        return false;
    };
    let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return false;
    };
    let now = now.as_secs();

    now.abs_diff(timestamp) <= MAX_SIGNATURE_AGE_SECONDS
}

#[cfg(test)]
mod tests {
    use super::verify_interaction_signature;
    use axum::body::Bytes;
    use axum::http::{HeaderMap, HeaderValue, header::HeaderName};
    use ed25519_dalek::Signer;

    fn unix_timestamp_now() -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    fn signed_request(body: &[u8]) -> (HeaderMap, String) {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        let timestamp = unix_timestamp_now();
        let signature = signing_key
            .sign(&[timestamp.as_bytes(), body].concat())
            .to_bytes();

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-signature-ed25519"),
            HeaderValue::from_str(&hex::encode(signature)).unwrap(),
        );
        headers.insert(
            HeaderName::from_static("x-signature-timestamp"),
            HeaderValue::from_str(&timestamp).unwrap(),
        );

        (headers, hex::encode(signing_key.verifying_key().to_bytes()))
    }

    #[test]
    fn valid_signature_succeeds() {
        let body = Bytes::from_static(br#"{"type":1}"#);
        let (headers, public_key_hex) = signed_request(body.as_ref());

        assert!(verify_interaction_signature(
            &headers,
            &body,
            &public_key_hex
        ));
    }

    #[test]
    fn tampered_body_fails() {
        let body = Bytes::from_static(br#"{"type":1}"#);
        let tampered_body = Bytes::from_static(br#"{"type":2}"#);
        let (headers, public_key_hex) = signed_request(body.as_ref());

        assert!(!verify_interaction_signature(
            &headers,
            &tampered_body,
            &public_key_hex
        ));
    }

    #[test]
    fn tampered_signature_fails() {
        let body = Bytes::from_static(br#"{"type":1}"#);
        let (mut headers, public_key_hex) = signed_request(body.as_ref());
        let signature_header = headers
            .get("x-signature-ed25519")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let mut signature_bytes = hex::decode(signature_header).unwrap();
        signature_bytes[0] ^= 0xFF;
        headers.insert(
            HeaderName::from_static("x-signature-ed25519"),
            HeaderValue::from_str(&hex::encode(signature_bytes)).unwrap(),
        );

        assert!(!verify_interaction_signature(
            &headers,
            &body,
            &public_key_hex
        ));
    }

    #[test]
    fn missing_headers_fail() {
        let headers = HeaderMap::new();
        let body = Bytes::from_static(br#"{"type":1}"#);

        assert!(!verify_interaction_signature(
            &headers,
            &body,
            &"11".repeat(32)
        ));
    }

    #[test]
    fn invalid_public_key_hex_fails() {
        let body = Bytes::from_static(br#"{"type":1}"#);
        let (headers, _) = signed_request(body.as_ref());

        assert!(!verify_interaction_signature(&headers, &body, "not-hex"));
    }

    #[test]
    fn bad_public_key_length_fails() {
        let body = Bytes::from_static(br#"{"type":1}"#);
        let (headers, _) = signed_request(body.as_ref());

        assert!(!verify_interaction_signature(&headers, &body, "abcd"));
    }
}
