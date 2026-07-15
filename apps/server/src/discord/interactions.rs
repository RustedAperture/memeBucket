use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    discord::signatures::verify_interaction_signature,
    domain::user_key::DiscordUserKey,
    services::{
        images::resolve_image_url,
        random::{RandomService, RandomVisibility},
    },
};

const PING: u8 = 1;
const APPLICATION_COMMAND: u8 = 2;
const APPLICATION_COMMAND_AUTOCOMPLETE: u8 = 4;
const CHANNEL_MESSAGE_WITH_SOURCE: u8 = 4;
const APPLICATION_MODAL_SUBMIT: u8 = 5;
const APPLICATION_COMMAND_AUTOCOMPLETE_RESULT: u8 = 8;
const APPLICATION_MODAL: u8 = 9;
const EPHEMERAL_FLAG: u64 = 64;
const AUTOCOMPLETE_CHOICE_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub struct InteractionPayload {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub data: Option<InteractionData>,
    #[serde(default)]
    pub user: Option<InteractionUser>,
    #[serde(default)]
    pub member: Option<InteractionMember>,
    #[serde(default)]
    pub channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionData {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub options: Vec<InteractionOption>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub resolved: Option<InteractionResolved>,
    #[serde(default)]
    pub custom_id: Option<String>,
    #[serde(default)]
    pub components: Vec<InteractionComponent>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionResolved {
    #[serde(default)]
    pub messages: std::collections::HashMap<String, InteractionMessage>,
    #[serde(default)]
    pub users: std::collections::HashMap<String, InteractionUser>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionMessage {
    pub id: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub embeds: Vec<InteractionEmbed>,
    #[serde(default)]
    pub attachments: Vec<InteractionAttachment>,
    #[serde(default)]
    pub author: Option<InteractionUser>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InteractionComponent {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub custom_id: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub components: Vec<InteractionComponent>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub style: Option<u8>,
    #[serde(default)]
    pub min_length: Option<usize>,
    #[serde(default)]
    pub max_length: Option<usize>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub min_values: Option<usize>,
    #[serde(default)]
    pub max_values: Option<usize>,
    #[serde(default)]
    pub options: Vec<InteractionSelectOption>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InteractionSelectOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct InteractionEmbed {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub image: Option<InteractionEmbedMedia>,
    #[serde(default)]
    pub video: Option<InteractionEmbedMedia>,
    #[serde(default)]
    pub thumbnail: Option<InteractionEmbedMedia>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionEmbedMedia {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub proxy_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionAttachment {
    pub url: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionOption {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub options: Vec<InteractionOption>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionUser {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub accent_color: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionMember {
    #[serde(default)]
    pub user: Option<InteractionUser>,
    #[serde(default)]
    pub nick: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

pub fn plain_message(content: &str, private: bool) -> Value {
    let mut data = json!({
        "content": content,
    });

    if private {
        data["flags"] = json!(EPHEMERAL_FLAG);
    }

    json!({
        "type": CHANNEL_MESSAGE_WITH_SOURCE,
        "data": data,
    })
}

pub async fn fetch_user_accent_color(state: &AppState, user_id: &str) -> Option<u32> {
    if state.discord_bot_token().is_empty() {
        return None;
    }

    let url = format!("https://discord.com/api/v10/users/{}", user_id);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
        .ok()?;

    let res = client
        .get(&url)
        .header(
            "Authorization",
            format!("Bot {}", state.discord_bot_token()),
        )
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let user_data: serde_json::Value = res.json().await.ok()?;
    user_data
        .get("accent_color")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
}

pub fn embed_message(content: &str, image_url: &str, private: bool, color: Option<u32>) -> Value {
    let mut data = json!({
        "content": content,
        "embeds": [{
            "color": color.unwrap_or(5793266), // 0x5865F2 fallback
            "image": {
                "url": image_url
            }
        }]
    });

    if private {
        data["flags"] = json!(EPHEMERAL_FLAG);
    }

    json!({
        "type": CHANNEL_MESSAGE_WITH_SOURCE,
        "data": data,
    })
}

pub fn ephemeral_message(content: &str) -> Value {
    plain_message(content, true)
}

pub fn autocomplete_choices(values: Vec<(String, String)>) -> Value {
    let choices: Vec<Value> = values
        .into_iter()
        .filter(|(name, value)| is_valid_autocomplete_choice(name, value))
        .take(25)
        .map(|(name, value)| {
            json!({
                "name": name,
                "value": value,
            })
        })
        .collect();

    json!({
        "type": APPLICATION_COMMAND_AUTOCOMPLETE_RESULT,
        "data": {
            "choices": choices,
        }
    })
}

fn is_valid_autocomplete_choice(name: &str, value: &str) -> bool {
    !name.is_empty()
        && !value.is_empty()
        && name.chars().count() <= AUTOCOMPLETE_CHOICE_LIMIT
        && value.chars().count() <= AUTOCOMPLETE_CHOICE_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_bucket_names_accepts_comma_separated_values_with_optional_spaces() {
        assert_eq!(split_bucket_names("cat,dog"), vec!["cat", "dog"]);
        assert_eq!(split_bucket_names("cat, dog"), vec!["cat", "dog"]);
        assert_eq!(split_bucket_names(" cat,  dog ,, "), vec!["cat", "dog"]);
    }

    #[test]
    fn bucket_autocomplete_context_completes_last_comma_separated_segment() {
        let context = bucket_autocomplete_context("cat, do");

        assert_eq!(context.query, "do");
        assert_eq!(context.value_for("Dogs"), "cat, Dogs");
        assert_eq!(
            context.choice_for("Dogs"),
            ("cat, Dogs".to_string(), "cat, Dogs".to_string())
        );
    }

    #[test]
    fn bucket_autocomplete_context_completes_first_segment_without_prefix() {
        let context = bucket_autocomplete_context("do");

        assert_eq!(context.query, "do");
        assert_eq!(context.value_for("Dogs"), "Dogs");
        assert_eq!(
            context.choice_for("Dogs"),
            ("Dogs".to_string(), "Dogs".to_string())
        );
    }

    #[test]
    fn autocomplete_choices_omits_values_discord_would_reject() {
        let too_long_value = "x".repeat(AUTOCOMPLETE_CHOICE_LIMIT + 1);
        let response = autocomplete_choices(vec![
            ("Invalid".to_string(), too_long_value),
            ("Valid".to_string(), "valid".to_string()),
        ]);
        let choices = response["data"]["choices"].as_array().unwrap();

        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0]["name"], "Valid");
        assert_eq!(choices[0]["value"], "valid");
    }
}

pub async fn handle_interaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if !verify_interaction_signature(&headers, &body, state.discord_public_key()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let payload: InteractionPayload = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match payload.kind {
        PING => Json(InteractionResponse {
            kind: PING,
            data: None,
        })
        .into_response(),
        APPLICATION_COMMAND => Json(dispatch_command(&state, &payload).await).into_response(),
        APPLICATION_COMMAND_AUTOCOMPLETE => {
            Json(dispatch_autocomplete(&state, &payload).await).into_response()
        }
        APPLICATION_MODAL_SUBMIT => {
            Json(dispatch_modal_submit(&state, &payload).await).into_response()
        }
        _ => Json(ephemeral_message("Unsupported interaction.")).into_response(),
    }
}

impl InteractionData {
    fn option(&self, name: &str) -> Option<&InteractionOption> {
        self.options.iter().find(|option| option.name == name)
    }

    fn subcommand(&self) -> Option<&InteractionOption> {
        self.options.iter().find(|option| option.kind == 1)
    }
}

impl InteractionOption {
    fn option(&self, name: &str) -> Option<&InteractionOption> {
        self.options.iter().find(|option| option.name == name)
    }

    fn string_value(&self) -> Option<&str> {
        self.value.as_ref()?.as_str()
    }

    fn bool_value(&self) -> Option<bool> {
        self.value.as_ref()?.as_bool()
    }

    fn find_focused_option(&self, name: &str) -> Option<&InteractionOption> {
        if self.focused && self.name == name {
            return Some(self);
        }

        self.options
            .iter()
            .find_map(|option| option.find_focused_option(name))
    }
}

#[derive(Clone, Debug)]
struct AppUser {
    id: Uuid,
}

enum DiscordAuthError {
    SecretMissing,
    MissingUser,
    Storage,
}

impl DiscordAuthError {
    fn user_message(&self) -> &'static str {
        match self {
            Self::SecretMissing => "Discord account setup is unavailable right now.",
            Self::MissingUser => "I could not identify your Discord account.",
            Self::Storage => "I could not access your account right now.",
        }
    }
}

async fn dispatch_command(state: &AppState, payload: &InteractionPayload) -> Value {
    let Some(data) = payload.data.as_ref() else {
        return ephemeral_message("Malformed command payload.");
    };

    let user = match resolve_user(state, payload).await {
        Ok(user) => user,
        Err(error) => return ephemeral_message(error.user_message()),
    };

    match data.name.as_str() {
        "mb" => handle_random_command(state, user.id, data).await,
        "bucket" => handle_bucket_command(state, user.id, data).await,
        "manage" => handle_manage_command().await,
        "Add to Bucket" => {
            handle_add_to_bucket_message_command(
                state,
                user.id,
                data,
                payload.channel_id.as_deref(),
            )
            .await
        }
        "Reply with GIF" => handle_reply_with_gif_command(state, user.id, data).await,
        _ => ephemeral_message("Unsupported command."),
    }
}

async fn dispatch_autocomplete(state: &AppState, payload: &InteractionPayload) -> Value {
    let Some(data) = payload.data.as_ref() else {
        return autocomplete_choices(Vec::new());
    };

    let user = match resolve_user(state, payload).await {
        Ok(user) => user,
        Err(_) => return autocomplete_choices(Vec::new()),
    };

    let Some(focused) = find_focused_option(data, "bucket") else {
        return autocomplete_choices(Vec::new());
    };
    if !supports_bucket_autocomplete(data) {
        return autocomplete_choices(Vec::new());
    }

    let focused_value = focused.string_value().unwrap_or_default();
    let autocomplete = if data.name == "mb" {
        bucket_autocomplete_context(focused_value)
    } else {
        BucketAutocompleteContext::single(focused_value)
    };
    let mut buckets = state
        .bucket_repo
        .list_for_user(user.id)
        .await
        .unwrap_or_default();

    if data.name == "mb"
        && let Ok(subscribed) = state.bucket_repo.list_subscribed_for_user(user.id).await
    {
        buckets.extend(subscribed);
    }

    let choices = buckets
        .into_iter()
        .filter(|bucket| {
            autocomplete.query.is_empty()
                || bucket
                    .name
                    .to_lowercase()
                    .contains(autocomplete.query.as_str())
        })
        .filter(|bucket| !autocomplete.already_completed(&bucket.name))
        .take(25)
        .map(|bucket| {
            let name = bucket.name;
            autocomplete.choice_for(&name)
        })
        .collect();

    autocomplete_choices(choices)
}

fn supports_bucket_autocomplete(data: &InteractionData) -> bool {
    match data.name.as_str() {
        "mb" => true,
        "bucket" => data
            .subcommand()
            .map(|subcommand| subcommand.name == "add")
            .unwrap_or(false),
        _ => false,
    }
}

fn find_focused_option<'a>(data: &'a InteractionData, name: &str) -> Option<&'a InteractionOption> {
    data.options
        .iter()
        .find_map(|option| option.find_focused_option(name))
}

#[derive(Debug, Eq, PartialEq)]
struct BucketAutocompleteContext {
    prefix: String,
    query: String,
    completed_names: Vec<String>,
}

impl BucketAutocompleteContext {
    fn single(value: &str) -> Self {
        Self {
            prefix: String::new(),
            query: value.trim().to_lowercase(),
            completed_names: Vec::new(),
        }
    }

    fn value_for(&self, bucket_name: &str) -> String {
        if self.prefix.is_empty() {
            bucket_name.to_string()
        } else {
            format!("{}, {bucket_name}", self.prefix)
        }
    }

    fn choice_for(&self, bucket_name: &str) -> (String, String) {
        let value = self.value_for(bucket_name);
        (value.clone(), value)
    }

    fn already_completed(&self, bucket_name: &str) -> bool {
        self.completed_names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(bucket_name))
    }
}

fn bucket_autocomplete_context(value: &str) -> BucketAutocompleteContext {
    let Some((prefix, query)) = value.rsplit_once(',') else {
        return BucketAutocompleteContext::single(value);
    };

    let completed_names = split_bucket_names(prefix);
    BucketAutocompleteContext {
        prefix: completed_names.join(", "),
        query: query.trim().to_lowercase(),
        completed_names,
    }
}

fn split_bucket_names(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

async fn handle_random_command(state: &AppState, user_id: Uuid, data: &InteractionData) -> Value {
    let Some(bucket_names_value) = data
        .option("bucket")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Bucket is required.");
    };
    let bucket_names = split_bucket_names(bucket_names_value);
    if bucket_names.is_empty() {
        return ephemeral_message("Bucket is required.");
    }

    let target = data
        .option("target")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let private = data
        .option("private")
        .and_then(InteractionOption::bool_value)
        .unwrap_or(false);

    let service = RandomService::new(
        state.bucket_repo.clone(),
        state.image_repo.clone(),
        state.send_history_repo.clone(),
    );
    let bucket_name_refs = bucket_names.iter().map(String::as_str).collect::<Vec<_>>();

    match service
        .select_random_from_buckets(
            user_id,
            &bucket_name_refs,
            if private {
                RandomVisibility::Private
            } else {
                RandomVisibility::Public
            },
        )
        .await
    {
        Ok(selection) => {
            let mut target_color = None;
            let content = if let Some(target_id) = target {
                if let Some(resolved) = &data.resolved
                    && let Some(user) = resolved.users.get(target_id)
                {
                    target_color = user.accent_color;
                }
                if target_color.is_none() {
                    target_color = fetch_user_accent_color(state, target_id).await;
                }
                format!("<@{target_id}>")
            } else {
                String::new()
            };
            embed_message(&content, &selection.url, private, target_color)
        }
        Err(error) => ephemeral_message(error.user_message()),
    }
}

async fn handle_bucket_command(state: &AppState, user_id: Uuid, data: &InteractionData) -> Value {
    let Some(subcommand) = data.subcommand() else {
        return ephemeral_message("Unsupported command.");
    };

    match subcommand.name.as_str() {
        "create" => handle_bucket_create(state, user_id, subcommand).await,
        "add" => handle_bucket_add(state, user_id, subcommand).await,
        "list" => handle_bucket_list(state, user_id).await,
        _ => ephemeral_message("Unsupported command."),
    }
}

async fn handle_bucket_create(
    state: &AppState,
    user_id: Uuid,
    subcommand: &InteractionOption,
) -> Value {
    let Some(name) = subcommand
        .option("name")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Bucket name cannot be blank.");
    };

    match state.bucket_repo.create(user_id, name).await {
        Ok(bucket) => ephemeral_message(&format!("Created bucket \"{}\".", bucket.name)),
        Err(sqlx::Error::RowNotFound) => {
            ephemeral_message("You already have a bucket with that name.")
        }
        Err(_) => ephemeral_message("I hit a storage error while creating bucket."),
    }
}

async fn handle_bucket_add(
    state: &AppState,
    user_id: Uuid,
    subcommand: &InteractionOption,
) -> Value {
    let Some(bucket_name) = subcommand
        .option("bucket")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Bucket is required.");
    };

    let Some(url) = subcommand
        .option("url")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Image URL is required.");
    };

    let resolved = match resolve_image_url(url).await {
        Ok(resolved) => resolved,
        Err(err) => return ephemeral_message(err.user_message()),
    };

    let buckets = state.bucket_repo.clone();
    let images = state.image_repo.clone();
    let Some(bucket) = (match buckets.find_by_name_folded(user_id, bucket_name).await {
        Ok(bucket) => bucket,
        Err(_) => return ephemeral_message("I hit a storage error while finding bucket."),
    }) else {
        return ephemeral_message("I could not find that bucket.");
    };

    match images.create(user_id, bucket.id, &resolved.url).await {
        Ok(image) => {
            if let Some(notes) = &resolved.notes {
                let _ = images
                    .update_notes(user_id, bucket.id, image.id, Some(notes))
                    .await;
            }
            if !resolved.tags.is_empty() {
                let _ = images
                    .update_metadata_partial(
                        user_id,
                        bucket.id,
                        image.id,
                        &crate::repositories::images::UpdateImageMetadataPatch {
                            title: None,
                            notes: None,
                            favorite: None,
                            random_weight: None,
                            tags: Some(resolved.tags.clone()),
                            url: None,
                        },
                    )
                    .await;
            }
            ephemeral_message(&format!("Added image to \"{}\".", bucket.name))
        }
        Err(sqlx::Error::RowNotFound) => ephemeral_message("I could not find that bucket."),
        Err(_) => ephemeral_message("I hit a storage error while saving image."),
    }
}

async fn handle_bucket_list(state: &AppState, user_id: Uuid) -> Value {
    match state.bucket_repo.list_for_user(user_id).await {
        Ok(buckets) if buckets.is_empty() => ephemeral_message("You have no buckets yet."),
        Ok(buckets) => {
            let content = format!(
                "Your buckets:\n{}",
                buckets
                    .into_iter()
                    .map(|bucket| format!("- {}", bucket.name))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            ephemeral_message(&content)
        }
        Err(_) => ephemeral_message("I hit a storage error while listing buckets."),
    }
}

async fn handle_manage_command() -> Value {
    let base_url = std::env::var("PUBLIC_BASE_URL").unwrap_or_default();
    let url = if base_url.is_empty() {
        "https://example.com".to_string()
    } else {
        base_url
    };
    ephemeral_message(&format!(
        "Manage your buckets at: {}/buckets",
        url.trim_end_matches('/')
    ))
}

async fn resolve_user(
    state: &AppState,
    payload: &InteractionPayload,
) -> Result<AppUser, DiscordAuthError> {
    let secret = std::env::var("APP_USER_KEY_SECRET")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(DiscordAuthError::SecretMissing)?;

    let discord_user = payload
        .user
        .as_ref()
        .or_else(|| {
            payload
                .member
                .as_ref()
                .and_then(|member| member.user.as_ref())
        })
        .ok_or(DiscordAuthError::MissingUser)?;

    let display_name = payload
        .member
        .as_ref()
        .and_then(|member| member.nick.as_deref())
        .or(discord_user.global_name.as_deref())
        .or(discord_user.username.as_deref());

    let discord_user_key = DiscordUserKey::derive(secret.as_bytes(), &discord_user.id);
    let stored_user = state
        .user_repo
        .upsert_by_provider("discord", discord_user_key.as_hex(), display_name, None)
        .await
        .map_err(|_| DiscordAuthError::Storage)?;

    Ok(AppUser { id: stored_user.id })
}

async fn handle_add_to_bucket_message_command(
    state: &AppState,
    user_id: Uuid,
    data: &InteractionData,
    channel_id: Option<&str>,
) -> Value {
    let target_id = match &data.target_id {
        Some(id) => id,
        None => return ephemeral_message("Could not find the selected message."),
    };

    let resolved = match &data.resolved {
        Some(res) => res,
        None => return ephemeral_message("Could not find message details."),
    };

    let message = match resolved.messages.get(target_id) {
        Some(msg) => msg,
        None => return ephemeral_message("Could not find the selected message."),
    };

    let mut image_url = None;

    if let Some(attachment) = message.attachments.iter().find(|a| {
        let ct = a.content_type.as_deref().unwrap_or("");
        ct.starts_with("image/") || ct.starts_with("video/")
    }) {
        image_url = attachment
            .proxy_url
            .clone()
            .or_else(|| Some(attachment.url.clone()));
    } else if let Some(embed) = message.embeds.iter().find(|e| {
        e.image.is_some() || e.video.is_some() || e.thumbnail.is_some() || e.url.is_some()
    }) {
        if let Some(media) = &embed.image {
            image_url = media.proxy_url.clone().or_else(|| media.url.clone());
        } else if let Some(media) = &embed.video {
            image_url = media.proxy_url.clone().or_else(|| media.url.clone());
        } else if let Some(media) = &embed.thumbnail {
            image_url = media.proxy_url.clone().or_else(|| media.url.clone());
        } else {
            image_url = embed.url.clone();
        }
    } else {
        let content = message.content.trim();
        for part in content.split_whitespace() {
            if part.starts_with("http") {
                image_url = Some(part.to_string());
                break;
            }
        }
    }

    if let Some(url) = &image_url
        && url.contains("discordapp.")
        && !url.contains("ex=")
    {
        for part in message.content.split_whitespace() {
            if part.starts_with("http") && part.contains("discordapp.") && part.contains("ex=") {
                image_url = Some(part.to_string());
                break;
            }
        }
    }

    let Some(url) = image_url else {
        return ephemeral_message("I could not find an image or GIF in that message.");
    };

    tracing::debug!(extracted_url = %url, "URL extracted from Discord message");

    let url = if url.contains("cdn.discordapp.com") && !url.contains("ex=") {
        tracing::debug!("Discord CDN URL missing auth params, attempting API re-fetch");
        let bot_token = state.discord_bot_token();
        if !bot_token.is_empty() {
            if let Some(fresh) =
                refetch_attachment_url(bot_token, channel_id, &message.id, &url).await
            {
                tracing::debug!(refreshed_url = %fresh, "Got fresh attachment URL from Discord API");
                fresh
            } else {
                let proxy_url = url.replace("cdn.discordapp.com", "media.discordapp.net");
                tracing::debug!(proxy_url = %proxy_url, "Trying media.discordapp.net proxy as fallback");
                proxy_url
            }
        } else {
            let proxy_url = url.replace("cdn.discordapp.com", "media.discordapp.net");
            tracing::debug!(proxy_url = %proxy_url, "Trying media.discordapp.net proxy as fallback");
            proxy_url
        }
    } else {
        url
    };

    let resolved = match resolve_image_url(&url).await {
        Ok(resolved) => resolved,
        Err(err) => return ephemeral_message(err.user_message()),
    };
    let mut resolved_url = resolved.url;
    let auto_notes = resolved.notes;

    let is_video = crate::services::video_converter::is_video_url(&resolved_url);

    if is_video && let Some(storage) = state.storage() {
        match crate::services::video_converter::convert_and_upload_video(&resolved_url, storage)
            .await
        {
            Ok(new_url) => resolved_url = new_url,
            Err(_) => {
                return ephemeral_message("I hit an error converting that video.");
            }
        }
    }

    let buckets = state.bucket_repo.clone();
    let bucket_name = "Inbox";

    let bucket = match buckets.create(user_id, bucket_name).await {
        Ok(bucket) => bucket,
        Err(sqlx::Error::RowNotFound) => {
            match buckets.find_by_name_folded(user_id, bucket_name).await {
                Ok(Some(bucket)) => bucket,
                _ => return ephemeral_message("I hit a storage error while finding your bucket."),
            }
        }
        Err(_) => return ephemeral_message("I hit a storage error while creating the bucket."),
    };

    let images = state.image_repo.clone();
    match images.create(user_id, bucket.id, &resolved_url).await {
        Ok(image) => {
            if let Some(notes) = &auto_notes {
                let _ = images
                    .update_notes(user_id, bucket.id, image.id, Some(notes))
                    .await;
            }
            if !resolved.tags.is_empty() {
                let _ = images
                    .update_metadata_partial(
                        user_id,
                        bucket.id,
                        image.id,
                        &crate::repositories::images::UpdateImageMetadataPatch {
                            title: None,
                            notes: None,
                            favorite: None,
                            random_weight: None,
                            tags: Some(resolved.tags.clone()),
                            url: None,
                        },
                    )
                    .await;
            }
            ephemeral_message(&format!("Added image to \"{}\".", bucket.name))
        }
        Err(_) => ephemeral_message("I hit a storage error while saving the image."),
    }
}

async fn refetch_attachment_url(
    bot_token: &str,
    channel_id: Option<&str>,
    message_id: &str,
    original_url: &str,
) -> Option<String> {
    let channel_id = channel_id?;

    let api_url =
        format!("https://discord.com/api/v10/channels/{channel_id}/messages/{message_id}");

    let client = reqwest::Client::new();
    let resp = client
        .get(&api_url)
        .header("Authorization", format!("Bot {bot_token}"))
        .header("User-Agent", "memeBucketBot/1.0")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::debug!(
            status = %status,
            body = %body,
            "Discord API message re-fetch failed"
        );
        return None;
    }

    let body: Value = resp.json().await.ok()?;

    let original_filename = original_url.rsplit('/').next().unwrap_or("");

    if let Some(attachments) = body["attachments"].as_array() {
        for attachment in attachments {
            if let Some(url) = attachment["url"].as_str() {
                let attachment_filename = url
                    .split('?')
                    .next()
                    .and_then(|base| base.rsplit('/').next())
                    .unwrap_or("");
                if attachment_filename == original_filename {
                    return Some(url.to_string());
                }
            }
        }
        if let Some(first) = attachments.first()
            && let Some(url) = first["url"].as_str()
        {
            return Some(url.to_string());
        }
    }

    None
}

async fn dispatch_modal_submit(state: &AppState, payload: &InteractionPayload) -> Value {
    let Some(data) = payload.data.as_ref() else {
        return ephemeral_message("Malformed modal submit payload.");
    };

    let user = match resolve_user(state, payload).await {
        Ok(user) => user,
        Err(error) => return ephemeral_message(error.user_message()),
    };

    let custom_id = data.custom_id.as_deref().unwrap_or("");
    if custom_id.starts_with("reply_with_gif:") {
        handle_reply_with_gif_submit(state, user.id, data, custom_id).await
    } else {
        ephemeral_message("Unknown modal submission.")
    }
}

async fn handle_reply_with_gif_command(
    _state: &AppState,
    _user_id: Uuid,
    data: &InteractionData,
) -> Value {
    let target_id = match &data.target_id {
        Some(id) => id,
        None => return ephemeral_message("Could not find the selected message."),
    };

    let resolved = match &data.resolved {
        Some(res) => res,
        None => return ephemeral_message("Could not find message details."),
    };

    let message = match resolved.messages.get(target_id) {
        Some(msg) => msg,
        None => return ephemeral_message("Could not find the selected message."),
    };

    let author = match &message.author {
        Some(author) => author,
        None => return ephemeral_message("Could not find message author."),
    };

    let author_id = &author.id;
    let target_color = author.accent_color;

    let color_str = match target_color {
        Some(c) => c.to_string(),
        None => String::new(),
    };

    let custom_id = format!("reply_with_gif:{}:{}", author_id, color_str);
    let mut components = vec![];

    let text_input = json!({
        "type": 4,
        "custom_id": "search_term",
        "label": "Bucket Name",
        "style": 1,
        "min_length": 1,
        "max_length": 100,
        "placeholder": "e.g. animals, funny",
        "required": true
    });

    components.push(json!({
        "type": 1,
        "components": [text_input]
    }));

    json!({
        "type": APPLICATION_MODAL,
        "data": {
            "custom_id": custom_id,
            "title": "Reply with GIF",
            "components": components
        }
    })
}

async fn handle_reply_with_gif_submit(
    state: &AppState,
    user_id: Uuid,
    data: &InteractionData,
    custom_id: &str,
) -> Value {
    let remainder = custom_id.trim_start_matches("reply_with_gif:");
    let mut parts = remainder.splitn(2, ':');
    let author_id = parts.next().unwrap_or("");
    let mut target_color = parts.next().and_then(|s| s.parse::<u32>().ok());

    if target_color.is_none() {
        target_color = fetch_user_accent_color(state, author_id).await;
    }

    let mut selected_buckets = Vec::new();
    let mut search_term = String::new();

    for row in &data.components {
        for component in &row.components {
            if component.custom_id.as_deref() == Some("buckets")
                || component.custom_id.as_deref() == Some("pools")
            {
                selected_buckets.extend(component.values.iter().cloned());
            } else if component.custom_id.as_deref() == Some("search_term")
                && let Some(val) = &component.value
            {
                search_term = val.clone();
            }
        }
    }

    let mut bucket_names = split_bucket_names(&search_term);
    bucket_names.extend(selected_buckets);

    if bucket_names.is_empty() {
        return ephemeral_message("Please provide a bucket name.");
    }

    let service = RandomService::new(
        state.bucket_repo.clone(),
        state.image_repo.clone(),
        state.send_history_repo.clone(),
    );
    let bucket_name_refs = bucket_names.iter().map(String::as_str).collect::<Vec<_>>();

    match service
        .select_random_from_buckets(user_id, &bucket_name_refs, RandomVisibility::Public)
        .await
    {
        Ok(selection) => {
            let content = format!("<@{author_id}>");
            embed_message(&content, &selection.url, false, target_color)
        }
        Err(error) => {
            let user_buckets = state
                .bucket_repo
                .list_bucket_names_for_user(user_id)
                .await
                .unwrap_or_default();

            let mut msg = error.user_message().to_string();
            if !user_buckets.is_empty() {
                let bucket_list = user_buckets.join(", ");
                msg = format!("{}\n\n**Your available buckets:** {}", msg, bucket_list);
            }
            ephemeral_message(&msg)
        }
    }
}
