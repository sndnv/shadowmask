use domain::error::MetadataError;
use serde::de::DeserializeOwned;

pub(crate) async fn send_ok(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("http status {}", response.status()));
    }
    Ok(response)
}

pub(crate) async fn get_json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T, MetadataError> {
    let response = send_ok(request).await.map_err(MetadataError::Backend)?;
    response
        .json::<T>()
        .await
        .map_err(|e| MetadataError::Parse(e.to_string()))
}
