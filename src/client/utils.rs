use secrecy::ExposeSecret;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use crate::client::config::Config;
use crate::client::consts::{AUTHORIZATION_HEADER, OPENAI_BETA_HEADER};

pub fn build_request(config: &Config) -> tokio_tungstenite::tungstenite::Result<Request> {
    let mut request = format!("{}/realtime?model={}", config.base_url().clone(), config.model().clone()).into_client_request()?;
    request.headers_mut()
        .insert(
            AUTHORIZATION_HEADER,
            format!("Bearer {}", config.api_key().expose_secret()).as_str().parse()?
        );
    request.headers_mut().insert(OPENAI_BETA_HEADER, "realtime=v1".parse()?);
    Ok(request)
}