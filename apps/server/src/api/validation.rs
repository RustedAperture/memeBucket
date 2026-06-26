use axum::{
    Json,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::AppError;

pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        value.validate().map_err(|e| {
            let errors = e
                .field_errors()
                .values()
                .map(|errs| {
                    errs.iter()
                        .map(|err| {
                            err.message
                                .as_ref()
                                .map(|m| m.as_ref())
                                .unwrap_or("invalid value")
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .collect::<Vec<_>>()
                .join("; ");
            AppError::BadRequest(errors)
        })?;

        Ok(ValidatedJson(value))
    }
}
