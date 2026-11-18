use crate::{DeviceFlowError, Response};

pub fn credential_error(msg: String) -> DeviceFlowError {
    DeviceFlowError::GitHubError(msg.into())
}

#[cfg(feature = "blocking")]
pub fn post(url: &str, body: String) -> Result<Response, reqwest::Error> {
    reqwest::blocking::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .body(body)
        .send()?
        .json::<Response>()
}

#[cfg(feature = "async")]
pub async fn post_async(url: &str, body: String) -> Result<Response, reqwest::Error> {
    reqwest::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await?
        .json::<Response>()
        .await
}
