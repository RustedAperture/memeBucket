use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

use crate::{
    app_state::AppState,
    auth::sessions::{AuthenticatedUser, lookup_session, read_session_cookie},
};

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Allow tests to inject an AuthenticatedUser directly via extensions
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            return Ok(user.clone());
        }

        let session_id =
            read_session_cookie(&parts.headers).ok_or(StatusCode::UNAUTHORIZED)?;

        lookup_session(&state.pool, &session_id)
            .await
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}
