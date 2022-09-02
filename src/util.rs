
use crate::{DeviceFlow, DeviceFlowError, DeviceFlowState};
use std::collections::HashMap;

pub fn credential_error(msg: String) -> DeviceFlowError {
    DeviceFlowError::GitHubError(msg.into())
}

pub fn send_request(device_flow: &mut DeviceFlow, url: String, body: String) -> Option<HashMap<String, serde_json::Value>> {
        let client = reqwest::blocking::Client::new();
        let response_struct = client.post(&url)
            .header("Accept", "application/json")
            .body(body)
            .send();

        match response_struct {
            Ok(resp) => {
                match resp.json::<HashMap<String, serde_json::Value>>() {
                    Ok(hm) => Some(hm),
                    Err(err) => {
                        device_flow.state = DeviceFlowState::Failure(err.into());
                        None
                    }
                }
            },
            Err(err) => {
                device_flow.state = DeviceFlowState::Failure(err.into());
                None
            }
        }
}
