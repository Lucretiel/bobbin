use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use reqwest;
use secrecy::{self, ExposeSecret, SecretString};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

/// A token is something that can modify a request to make it authorized for
/// an API call
pub trait Token {
    fn apply(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder;
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub consumer_key: SecretString,
    pub consumer_secret: SecretString,
}

const TOKEN_URL: &'static str = "https://api.twitter.com/oauth2/token";

#[derive(Debug)]
pub enum BearerTokenError {
    HTTPError(reqwest::Error),
    NonBearerError,
}

impl Display for BearerTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BearerTokenError::HTTPError(err) => write!(f, "HTTP error: {}", err),
            BearerTokenError::NonBearerError => write!(f, "token_type was not 'bearer'"),
        }
    }
}

impl Error for BearerTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BearerTokenError::HTTPError(err) => Some(err),
            BearerTokenError::NonBearerError => None,
        }
    }
}

impl From<reqwest::Error> for BearerTokenError {
    fn from(err: reqwest::Error) -> Self {
        BearerTokenError::HTTPError(err)
    }
}

pub async fn generate_bearer_token(
    client: &reqwest::Client,
    credentials: &Credentials,
) -> Result<BearerToken, BearerTokenError> {
    struct GrantTypeFormData;

    impl Serialize for GrantTypeFormData {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry("grant_type", "client_credentials")?;
            map.end()
        }
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        token_type: String,
        access_token: SecretString,
    }

    let result: TokenResponse = client
        .post(TOKEN_URL)
        .basic_auth(
            credentials.consumer_key.expose_secret(),
            Some(credentials.consumer_secret.expose_secret()),
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&GrantTypeFormData)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if result.token_type != "bearer" {
        Err(BearerTokenError::NonBearerError)
    } else {
        Ok(BearerToken {
            token: result.access_token,
        })
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

// TODO: auth service: a background task that handles refreshing API tokens
// in the event that requests start failing. That way, if multiple request
// handlers all start failing at once, we can just get the one key and hand it
// back out without hammering twitter's api service.
