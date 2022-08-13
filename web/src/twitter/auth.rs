use once_cell::sync::OnceCell;
use secrecy::{self, ExposeSecret as _, SecretString};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::serialize_static_map;

/// A token is something that can modify a request to make it authorized for
/// an API call
pub trait Token {
    fn apply(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder;
}

/// Secret credentials provided by twitter to the service owner (the owner of
/// bobbin itself)
#[derive(Debug, Clone)]
pub struct Credentials {
    pub consumer_key: SecretString,
    pub consumer_secret: SecretString,
}

const TOKEN_URL: &str = "https://api.twitter.com/oauth2/token";

#[derive(Debug, Error)]
pub enum BearerTokenError {
    #[error("HTTP error while creating bearer token")]
    HTTPError(#[from] reqwest::Error),

    #[error("token_type was not 'bearer'")]
    NonBearerError,
}

pub async fn generate_bearer_token(
    client: &reqwest::Client,
    credentials: &Credentials,
) -> Result<BearerToken, BearerTokenError> {
    #[derive(Deserialize)]
    enum TokenType {
        #[serde(rename = "bearer")]
        Bearer,

        #[serde(other)]
        Unknown,
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        token_type: TokenType,
        access_token: SecretString,
    }

    let response: TokenResponse = client
        .post(TOKEN_URL)
        .basic_auth(
            credentials.consumer_key.expose_secret(),
            Some(credentials.consumer_secret.expose_secret()),
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&serialize_static_map!(
            grant_type: "client_credentials",
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    match response.token_type {
        TokenType::Bearer => Ok(BearerToken {
            token: response.access_token,
        }),
        TokenType::Unknown => Err(BearerTokenError::NonBearerError),
    }
}

#[derive(Debug, Clone)]
pub struct BearerToken {
    token: SecretString,
}

impl Token for BearerToken {
    fn apply(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.bearer_auth(self.token.expose_secret())
    }
}

/// Helper trait to apply tokens to requests, implemented for `RequestBuilder`.
pub trait ApplyToken: Sized {
    fn apply_token(self, token: &impl Token) -> Self;
}

impl ApplyToken for reqwest::RequestBuilder {
    fn apply_token(self, token: &impl Token) -> Self {
        token.apply(self)
    }
}

// TODO: auth service: a background task that handles refreshing API tokens
// in the event that requests start failing. That way, if multiple request
// handlers all start failing at once, we can just get the one key and hand it
// back out without hammering twitter's api service.
