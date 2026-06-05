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
        pools::PoolRepository, images::ImageRepository,
        send_history::SendHistoryRepository, users::UserRepository,
    },
    services::{
        images::validate_http_url,
        random::{RandomService, RandomVisibility},
    },
};

const PING: u8 = 1;
const APPLICATION_COMMAND: u8 = 2;
const APPLICATION_COMMAND_AUTOCOMPLETE: u8 = 4;
const CHANNEL_MESSAGE_WITH_SOURCE: u8 = 4;
const APPLICATION_COMMAND_AUTOCOMPLETE_RESULT: u8 = 8;
const EPHEMERAL_FLAG: u64 = 64;

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

    let query = focused
        .string_value()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let pools = PoolRepository::new(state.pool.clone())
        .list_for_user(user.id)
        .await
        .unwrap_or_default();

    let choices = pools
        .into_iter()
        .filter(|pool| {
            query.is_empty() || pool.name.to_lowercase().contains(query.as_str())
        })
        .take(25)
        .map(|pool| {
            let name = pool.name;
            (name.clone(), name)
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

async fn handle_random_command(state: &AppState, user_id: Uuid, data: &InteractionData) -> Value {
    let Some(pool_name) = data
        .option("pool")
        .and_then(InteractionOption::string_value)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ephemeral_message("Pool is required.");
    };

    let private = data
        .option("private")
        .and_then(InteractionOption::bool_value)
        .unwrap_or(false);

    let service = RandomService::new(
        PoolRepository::new(state.pool.clone()),
        ImageRepository::new(state.pool.clone()),
        SendHistoryRepository::new(state.pool.clone()),
    );

    match service
        .select_random(
            user_id,
            pool_name,
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

    if !validate_http_url(url) {
        return ephemeral_message("URL must be a valid http or https URL.");
    }

    let pools = PoolRepository::new(state.pool.clone());
    let images = ImageRepository::new(state.pool.clone());
    let Some(pool) = (match pools.find_by_name_folded(user_id, pool_name).await {
        Ok(pool) => pool,
        Err(_) => return ephemeral_message("I hit a storage error while finding pool."),
    }) else {
        return ephemeral_message("I could not find that pool.");
    };

    match images.create(user_id, pool.id, url).await {
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
