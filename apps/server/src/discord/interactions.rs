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
    repositories::{
        images::ImageRepository, pools::PoolRepository, send_history::SendHistoryRepository,
        users::UserRepository,
    },
    services::{
        images::resolve_image_url,
        random::{RandomService, RandomVisibility},
    },
};

const PING: u8 = 1;
const APPLICATION_COMMAND: u8 = 2;
const APPLICATION_COMMAND_AUTOCOMPLETE: u8 = 4;
const CHANNEL_MESSAGE_WITH_SOURCE: u8 = 4;
const APPLICATION_COMMAND_AUTOCOMPLETE_RESULT: u8 = 8;
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
}

#[derive(Debug, Deserialize)]
pub struct InteractionData {
    pub name: String,
    #[serde(default)]
    pub options: Vec<InteractionOption>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub resolved: Option<InteractionResolved>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionResolved {
    #[serde(default)]
    pub messages: std::collections::HashMap<String, InteractionMessage>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionMessage {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub embeds: Vec<InteractionEmbed>,
    #[serde(default)]
    pub attachments: Vec<InteractionAttachment>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionEmbed {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub image: Option<InteractionEmbedMedia>,
    #[serde(default)]
    pub video: Option<InteractionEmbedMedia>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionEmbedMedia {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionAttachment {
    pub url: String,
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
    fn split_pool_names_accepts_comma_separated_values_with_optional_spaces() {
        assert_eq!(split_pool_names("cat,dog"), vec!["cat", "dog"]);
        assert_eq!(split_pool_names("cat, dog"), vec!["cat", "dog"]);
        assert_eq!(split_pool_names(" cat,  dog ,, "), vec!["cat", "dog"]);
    }

    #[test]
    fn pool_autocomplete_context_completes_last_comma_separated_segment() {
        let context = pool_autocomplete_context("cat, do");

        assert_eq!(context.query, "do");
        assert_eq!(context.value_for("Dogs"), "cat, Dogs");
        assert_eq!(
            context.choice_for("Dogs"),
            ("cat, Dogs".to_string(), "cat, Dogs".to_string())
        );
    }

    #[test]
    fn pool_autocomplete_context_completes_first_segment_without_prefix() {
        let context = pool_autocomplete_context("do");

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
        "ez" => handle_random_command(state, user.id, data).await,
        "pool" => handle_pool_command(state, user.id, data).await,
        "manage" => handle_manage_command().await,
        "Add to Pool" => handle_add_to_pool_message_command(state, user.id, data).await,
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

    let Some(focused) = find_focused_option(data, "pool") else {
        return autocomplete_choices(Vec::new());
    };
    if !supports_pool_autocomplete(data) {
        return autocomplete_choices(Vec::new());
    }

    let focused_value = focused.string_value().unwrap_or_default();
    let autocomplete = if data.name == "ez" {
        pool_autocomplete_context(focused_value)
    } else {
        PoolAutocompleteContext::single(focused_value)
    };
    let mut pools = PoolRepository::new(state.pool.clone())
        .list_for_user(user.id)
        .await
        .unwrap_or_default();

    if data.name == "ez"
        && let Ok(subscribed) = PoolRepository::new(state.pool.clone())
            .list_subscribed_for_user(user.id)
            .await
    {
        pools.extend(subscribed);
    }

    let choices = pools
        .into_iter()
        .filter(|pool| {
            autocomplete.query.is_empty()
                || pool
                    .name
                    .to_lowercase()
                    .contains(autocomplete.query.as_str())
        })
        .filter(|pool| !autocomplete.already_completed(&pool.name))
        .take(25)
        .map(|pool| {
            let name = pool.name;
            autocomplete.choice_for(&name)
        })
        .collect();

    autocomplete_choices(choices)
}

fn supports_pool_autocomplete(data: &InteractionData) -> bool {
    match data.name.as_str() {
        "ez" => true,
        "pool" => data
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
struct PoolAutocompleteContext {
    prefix: String,
    query: String,
    completed_names: Vec<String>,
}

impl PoolAutocompleteContext {
    fn single(value: &str) -> Self {
        Self {
            prefix: String::new(),
            query: value.trim().to_lowercase(),
            completed_names: Vec::new(),
        }
    }

    fn value_for(&self, pool_name: &str) -> String {
        if self.prefix.is_empty() {
            pool_name.to_string()
        } else {
            format!("{}, {pool_name}", self.prefix)
        }
    }

    fn choice_for(&self, pool_name: &str) -> (String, String) {
        let value = self.value_for(pool_name);
        (value.clone(), value)
    }

    fn already_completed(&self, pool_name: &str) -> bool {
        self.completed_names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(pool_name))
    }
}

fn pool_autocomplete_context(value: &str) -> PoolAutocompleteContext {
    let Some((prefix, query)) = value.rsplit_once(',') else {
        return PoolAutocompleteContext::single(value);
    };

    let completed_names = split_pool_names(prefix);
    PoolAutocompleteContext {
        prefix: completed_names.join(", "),
        query: query.trim().to_lowercase(),
        completed_names,
    }
}

fn split_pool_names(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

async fn handle_random_command(state: &AppState, user_id: Uuid, data: &InteractionData) -> Value {
    let Some(pool_names_value) = data
        .option("pool")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Pool is required.");
    };
    let pool_names = split_pool_names(pool_names_value);
    if pool_names.is_empty() {
        return ephemeral_message("Pool is required.");
    }

    let private = data
        .option("private")
        .and_then(InteractionOption::bool_value)
        .unwrap_or(false);

    let service = RandomService::new(
        PoolRepository::new(state.pool.clone()),
        ImageRepository::new(state.pool.clone()),
        SendHistoryRepository::new(state.pool.clone()),
    );
    let pool_name_refs = pool_names.iter().map(String::as_str).collect::<Vec<_>>();

    match service
        .select_random_from_pools(
            user_id,
            &pool_name_refs,
            if private {
                RandomVisibility::Private
            } else {
                RandomVisibility::Public
            },
        )
        .await
    {
        Ok(selection) => plain_message(&selection.url, private),
        Err(error) => ephemeral_message(error.user_message()),
    }
}

async fn handle_pool_command(state: &AppState, user_id: Uuid, data: &InteractionData) -> Value {
    let Some(subcommand) = data.subcommand() else {
        return ephemeral_message("Unsupported command.");
    };

    match subcommand.name.as_str() {
        "create" => handle_pool_create(state, user_id, subcommand).await,
        "add" => handle_pool_add(state, user_id, subcommand).await,
        "list" => handle_pool_list(state, user_id).await,
        _ => ephemeral_message("Unsupported command."),
    }
}

async fn handle_pool_create(
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
        return ephemeral_message("Pool name cannot be blank.");
    };

    match PoolRepository::new(state.pool.clone())
        .create(user_id, name)
        .await
    {
        Ok(pool) => ephemeral_message(&format!("Created pool \"{}\".", pool.name)),
        Err(sqlx::Error::RowNotFound) => {
            ephemeral_message("You already have a pool with that name.")
        }
        Err(_) => ephemeral_message("I hit a storage error while creating pool."),
    }
}

async fn handle_pool_add(state: &AppState, user_id: Uuid, subcommand: &InteractionOption) -> Value {
    let Some(pool_name) = subcommand
        .option("pool")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Pool is required.");
    };

    let Some(url) = subcommand
        .option("url")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Image URL is required.");
    };

    let resolved_url = match resolve_image_url(url).await {
        Ok(url) => url,
        Err(err) => return ephemeral_message(err.user_message()),
    };

    let pools = PoolRepository::new(state.pool.clone());
    let images = ImageRepository::new(state.pool.clone());
    let Some(pool) = (match pools.find_by_name_folded(user_id, pool_name).await {
        Ok(pool) => pool,
        Err(_) => return ephemeral_message("I hit a storage error while finding pool."),
    }) else {
        return ephemeral_message("I could not find that pool.");
    };

    match images.create(user_id, pool.id, &resolved_url).await {
        Ok(_) => ephemeral_message(&format!("Added image to \"{}\".", pool.name)),
        Err(sqlx::Error::RowNotFound) => ephemeral_message("I could not find that pool."),
        Err(_) => ephemeral_message("I hit a storage error while saving image."),
    }
}

async fn handle_pool_list(state: &AppState, user_id: Uuid) -> Value {
    match PoolRepository::new(state.pool.clone())
        .list_for_user(user_id)
        .await
    {
        Ok(pools) if pools.is_empty() => ephemeral_message("You have no pools yet."),
        Ok(pools) => {
            let content = format!(
                "Your pools:\n{}",
                pools
                    .into_iter()
                    .map(|pool| format!("- {}", pool.name))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            ephemeral_message(&content)
        }
        Err(_) => ephemeral_message("I hit a storage error while listing pools."),
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
        "Manage your pools at: {}/pools",
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
    let stored_user = UserRepository::new(state.pool.clone())
        .upsert_by_discord_key(discord_user_key.as_hex(), display_name, None)
        .await
        .map_err(|_| DiscordAuthError::Storage)?;

    Ok(AppUser { id: stored_user.id })
}

async fn handle_add_to_pool_message_command(
    state: &AppState,
    user_id: Uuid,
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

    let mut image_url = None;

    if let Some(attachment) = message.attachments.iter().find(|a| {
        let ct = a.content_type.as_deref().unwrap_or("");
        ct.starts_with("image/") || ct.starts_with("video/")
    }) {
        image_url = Some(attachment.url.clone());
    } else if let Some(embed) = message
        .embeds
        .iter()
        .find(|e| e.image.is_some() || e.video.is_some() || e.url.is_some())
    {
        if let Some(media) = &embed.image {
            image_url = media.url.clone();
        } else if let Some(media) = &embed.video {
            image_url = media.url.clone();
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

    let Some(url) = image_url else {
        return ephemeral_message("I could not find an image or GIF in that message.");
    };

    let mut resolved_url = match resolve_image_url(&url).await {
        Ok(url) => url,
        Err(err) => return ephemeral_message(err.user_message()),
    };

    let is_video = {
        let base = resolved_url
            .split('?')
            .next()
            .unwrap_or(&resolved_url)
            .to_lowercase();
        base.ends_with(".mp4") || base.ends_with(".webm")
    };

    if is_video && let Some(key) = &state.imgbb_api_key {
        match crate::services::video_converter::convert_and_upload_mp4(&resolved_url, key).await {
            Ok(new_url) => resolved_url = new_url,
            Err(_) => {
                return ephemeral_message("I hit an error converting that video to a GIF.");
            }
        }
    }

    let pools = PoolRepository::new(state.pool.clone());
    let pool_name = "Added from Discord";

    let pool = match pools.create(user_id, pool_name).await {
        Ok(pool) => pool,
        Err(sqlx::Error::RowNotFound) => {
            match pools.find_by_name_folded(user_id, pool_name).await {
                Ok(Some(pool)) => pool,
                _ => return ephemeral_message("I hit a storage error while finding your pool."),
            }
        }
        Err(_) => return ephemeral_message("I hit a storage error while creating the pool."),
    };

    let images = ImageRepository::new(state.pool.clone());
    match images.create(user_id, pool.id, &resolved_url).await {
        Ok(_) => ephemeral_message(&format!("Added image to \"{}\".", pool.name)),
        Err(_) => ephemeral_message("I hit a storage error while saving the image."),
    }
}
