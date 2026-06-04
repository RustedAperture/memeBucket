use random_media_bot_server::domain::user_key::DiscordUserKey;

#[test]
fn same_secret_and_discord_id_produce_same_key() {
    let first = DiscordUserKey::derive(b"test-secret", "123456789012345678");
    let second = DiscordUserKey::derive(b"test-secret", "123456789012345678");

    assert_eq!(first.as_hex(), second.as_hex());
    assert_ne!(first.as_hex(), "123456789012345678");
}

#[test]
fn different_secrets_produce_different_keys() {
    let first = DiscordUserKey::derive(b"secret-a", "123456789012345678");
    let second = DiscordUserKey::derive(b"secret-b", "123456789012345678");

    assert_ne!(first.as_hex(), second.as_hex());
}

#[test]
fn same_secret_and_different_discord_ids_produce_different_keys() {
    let first = DiscordUserKey::derive(b"test-secret", "123456789012345678");
    let second = DiscordUserKey::derive(b"test-secret", "987654321098765432");

    assert_ne!(first.as_hex(), second.as_hex());
}

#[test]
fn key_matches_known_hmac_sha256_vector() {
    let key = DiscordUserKey::derive(b"test-secret", "123456789012345678");

    assert_eq!(
        key.as_hex(),
        "a1658c89203bb13e4e154f70156deb18a74120ab36551a1f2b695b75b3c17e0d"
    );
}

#[test]
fn key_is_lowercase_hex_sha256_length() {
    let key = DiscordUserKey::derive(b"test-secret", "123456789012345678");

    assert_eq!(key.as_hex().len(), 64);
    assert!(
        key.as_hex()
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}
