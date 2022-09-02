
use crate::DeviceFlowError;

pub fn credential_error(msg: String) -> DeviceFlowError {
    DeviceFlowError::GitHubError(msg.into())
}

