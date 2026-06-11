use axum::{
    Json,
    extract::{Query, State},
};
use reqwest::Client;
use serde::Deserialize;

use crate::{app_state::AppState, error::AppError};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn search_gifs(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let api_key = state.klipy_api_key.as_ref().ok_or_else(|| {
        AppError::BadRequest("GIF search is not configured (KLIPY_API_KEY is missing)".to_string())
    })?;

    let client = Client::new();
    let q = query.q.unwrap_or_default().trim().to_string();

    let req = if q.is_empty() {
        client.get(format!(
            "https://api.klipy.com/api/v1/{}/gifs/trending",
            api_key
        ))
    } else {
        client
            .get(format!(
                "https://api.klipy.com/api/v1/{}/gifs/search",
                api_key
            ))
            .query(&[("q", q)])
    };

    let response = req.send().await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to contact Klipy API: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(AppError::InternalServerError(format!(
            "Klipy API returned an error: {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to parse Klipy API response: {}", e))
    })?;

    Ok(Json(json))
}
