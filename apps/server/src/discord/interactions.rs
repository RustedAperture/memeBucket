use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{app_state::AppState, discord::signatures::verify_interaction_signature};

#[derive(Debug, Deserialize)]
pub struct InteractionPayload {
    #[serde(rename = "type")]
    pub kind: u8,
}

#[derive(Debug, Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
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
        1 => Json(InteractionResponse {
            kind: 1,
            data: None,
        })
        .into_response(),
        _ => Json(json!({
            "type": 4,
            "data": {
                "content": "Unsupported interaction.",
                "flags": 64
            }
        }))
        .into_response(),
    }
}
