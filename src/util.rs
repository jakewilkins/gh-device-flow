
use crate::DeviceFlowError;

pub fn credential_error(msg: &str) -> Box<DeviceFlowError> {
    Box::new(DeviceFlowError(msg.into()))
}

