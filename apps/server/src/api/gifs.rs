use axum::{
    Json,
    extract::{Query, State},
};
use reqwest::Client;
use serde::Deserialize;
use std::time::Instant;

use crate::{
    app_state::{AppState, GifSearchCacheEntry},
    error::AppError,
};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
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
    let page = query.page;
    let per_page = query.per_page.map(|value| value.clamp(1, 50));
    let cache_key = gif_search_cache_key(&q, page, per_page);

    if let Some(cached) = get_cached_gif_search(&state, &cache_key) {
        return Ok(Json(cached));
    }

    let mut params = Vec::new();
    if let Some(page) = page {
        params.push(("page", page.to_string()));
    }
    if let Some(per_page) = per_page {
        params.push(("per_page", per_page.to_string()));
    }

    let req = if q.is_empty() {
        let request = client.get(format!(
            "{}/api/v1/{}/gifs/trending",
            state.klipy_api_base_url, api_key
        ));
        if params.is_empty() {
            request
        } else {
            request.query(&params)
        }
    } else {
        params.push(("q", q));
        client
            .get(format!(
                "{}/api/v1/{}/gifs/search",
                state.klipy_api_base_url, api_key
            ))
            .query(&params)
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

    cache_gif_search(&state, cache_key, json.clone());

    Ok(Json(json))
}

fn gif_search_cache_key(q: &str, page: Option<u32>, per_page: Option<u32>) -> String {
    let normalized = q.trim().to_ascii_lowercase();
    let query_key = if normalized.is_empty() {
        "trending".to_string()
    } else {
        format!("search:{normalized}")
    };

    format!(
        "{query_key}:page={}:per_page={}",
        page.unwrap_or(1),
        per_page.unwrap_or(24)
    )
}

fn get_cached_gif_search(state: &AppState, cache_key: &str) -> Option<serde_json::Value> {
    let now = Instant::now();
    let mut cache = state.gif_search_cache.lock().ok()?;
    let entry = cache.get(cache_key)?;

    if entry.expires_at > now {
        return Some(entry.value.clone());
    }

    cache.remove(cache_key);
    None
}

fn cache_gif_search(state: &AppState, cache_key: String, value: serde_json::Value) {
    let Ok(mut cache) = state.gif_search_cache.lock() else {
        return;
    };

    cache.insert(
        cache_key,
        GifSearchCacheEntry {
            value,
            expires_at: Instant::now() + state.gif_search_cache_ttl,
        },
    );
}
