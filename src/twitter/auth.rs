use reqwest

/// A token is something that can modify a request to make it authorized for
/// an API call
pub trait Token {
	fn authorize_request(&self, request: reqwest::R)
}
