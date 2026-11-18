//! Async versions of the device flow API, built on `reqwest::Client` and
//! `tokio::time::sleep`. Enabled with the `async` feature.

use crate::{
    parse_refresh_response, refresh_request, util, Credential, DeviceFlow, DeviceFlowError,
    FIVE_SECONDS,
};

pub async fn authorize(
    client_id: String,
    host: Option<String>,
    scope: Option<String>,
) -> Result<Credential, DeviceFlowError> {
    let mut flow =
        DeviceFlow::start_async(client_id.as_str(), host.as_deref(), scope.as_deref()).await?;

    flow.print_instructions();
    tokio::time::sleep(FIVE_SECONDS).await;

    flow.poll_async(20).await
}

pub async fn refresh(
    client_id: &str,
    refresh_token: &str,
    host: Option<String>,
    scope: Option<String>,
) -> Result<Credential, DeviceFlowError> {
    let (url, body) = refresh_request(client_id, refresh_token, host.as_deref(), scope.as_deref());
    parse_refresh_response(util::post_async(&url, body).await?)
}

impl DeviceFlow {
    pub async fn start_async(
        client_id: &str,
        maybe_host: Option<&str>,
        scope: Option<&str>,
    ) -> Result<DeviceFlow, DeviceFlowError> {
        let mut flow = DeviceFlow::new(client_id, maybe_host, scope);
        flow.setup_async().await;
        flow.started()
    }

    pub async fn setup_async(&mut self) {
        let (url, body) = self.setup_request();
        let res = util::post_async(&url, body).await;
        self.apply_setup_response(res);
    }

    pub async fn poll_async(&mut self, iterations: u32) -> Result<Credential, DeviceFlowError> {
        for count in 0..iterations {
            self.update_async().await;

            match self.next_interval(count, iterations) {
                Ok(Some(interval)) => tokio::time::sleep(interval).await,
                Ok(None) => break,
                Err(err) => return Err(err),
            }
        }

        self.poll_result()
    }

    pub async fn update_async(&mut self) {
        let (url, body) = self.poll_request();
        let res = util::post_async(&url, body).await;
        self.apply_poll_response(res);
    }
}
