use anyhow::{bail, Context, Result};
use reqwest::{Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct Client {
    base_url: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            http: reqwest::Client::new(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, path)
    }

    pub fn ws_url(&self, path: &str) -> String {
        let url = self.url(path);
        if let Some(rest) = url.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = url.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            url
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request_json(Method::GET, path, Option::<&Value>::None)
            .await
    }

    pub async fn post<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request_json(Method::POST, path, Some(body)).await
    }

    pub async fn patch<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request_json(Method::PATCH, path, Some(body)).await
    }

    pub async fn put<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request_json(Method::PUT, path, Some(body)).await
    }

    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request_json(Method::DELETE, path, Option::<&Value>::None)
            .await
    }

    async fn request_json<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        let mut request = self.http.request(method, self.url(path));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await.context("sending API request")?;
        let status = response.status();
        let text = response.text().await.context("reading API response")?;
        if !status.is_success() {
            bail!("{}", api_error(status, &text));
        }
        if text.trim().is_empty() {
            return serde_json::from_value(Value::Null).context("decoding empty response");
        }
        serde_json::from_str(&text).with_context(|| format!("decoding API response: {text}"))
    }
}

fn api_error(status: StatusCode, text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return format!("API request failed with HTTP {status}");
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(message) = value
            .get("fault_message")
            .or_else(|| value.get("error"))
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
        {
            return format!("API request failed with HTTP {status}: {message}");
        }
    }

    format!("API request failed with HTTP {status}: {trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn joins_base_url_and_path() {
        let client = Client::new("http://localhost:18080/".into(), None);
        assert_eq!(client.url("/v1/vms"), "http://localhost:18080/v1/vms");
    }

    #[test]
    fn builds_websocket_url() {
        let client = Client::new("http://localhost:18080/".into(), None);
        assert_eq!(
            client.ws_url("/v1/vms/abc/shell/ws"),
            "ws://localhost:18080/v1/vms/abc/shell/ws"
        );
    }

    #[tokio::test]
    async fn sends_bearer_auth_for_gets() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/v1/auth/me")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_body(r#"{"username":"root"}"#)
            .create_async()
            .await;

        let client = Client::new(server.url(), Some("test-token".into()));
        let value: Value = client.get("/v1/auth/me").await.unwrap();

        mock.assert_async().await;
        assert_eq!(value["username"], "root");
    }

    #[tokio::test]
    async fn posts_json_bodies() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/vms")
            .match_body(mockito::Matcher::Json(json!({"name": "dev"})))
            .with_status(200)
            .with_body(r#"{"id":"00000000-0000-0000-0000-000000000001"}"#)
            .create_async()
            .await;

        let client = Client::new(server.url(), None);
        let value: Value = client
            .post("/v1/vms", &json!({"name": "dev"}))
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(value["id"], "00000000-0000-0000-0000-000000000001");
    }
}
