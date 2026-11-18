use std::collections::HashMap;
use std::{fmt, result::Result, time};

use chrono::offset::Utc;
use chrono::{DateTime, Duration};

#[cfg(not(any(feature = "blocking", feature = "async")))]
compile_error!("enable at least one of the `blocking` or `async` features");

#[cfg(feature = "async")]
pub mod nonblocking;
mod util;

pub(crate) type Response = HashMap<String, serde_json::Value>;

#[derive(Debug, Default, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Credential {
    pub token: String,
    pub expiry: String,
    pub refresh_token: String,
}

impl Credential {
    fn empty() -> Credential {
        Credential {
            token: String::new(),
            expiry: String::new(),
            refresh_token: String::new(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let exp = match DateTime::parse_from_rfc3339(self.expiry.as_str()) {
            Ok(time) => time,
            Err(_) => return false,
        };
        let now = Utc::now();
        now > exp
    }
}

#[derive(Debug, Clone)]
pub enum DeviceFlowError {
    HttpError(String),
    GitHubError(String),
}

impl fmt::Display for DeviceFlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceFlowError::HttpError(string) => write!(f, "DeviceFlowError: {}", string),
            DeviceFlowError::GitHubError(string) => write!(f, "DeviceFlowError: {}", string),
        }
    }
}

impl std::error::Error for DeviceFlowError {}

impl From<reqwest::Error> for DeviceFlowError {
    fn from(e: reqwest::Error) -> Self {
        DeviceFlowError::HttpError(format!("{:?}", e))
    }
}

#[cfg(feature = "blocking")]
pub fn authorize(
    client_id: String,
    host: Option<String>,
    scope: Option<String>,
) -> Result<Credential, DeviceFlowError> {
    let mut flow = DeviceFlow::start(client_id.as_str(), host.as_deref(), scope.as_deref())?;

    flow.print_instructions();
    std::thread::sleep(FIVE_SECONDS);

    flow.poll(20)
}

#[cfg(feature = "blocking")]
pub fn refresh(
    client_id: &str,
    refresh_token: &str,
    host: Option<String>,
    scope: Option<String>,
) -> Result<Credential, DeviceFlowError> {
    let (url, body) = refresh_request(client_id, refresh_token, host.as_deref(), scope.as_deref());
    parse_refresh_response(util::post(&url, body)?)
}

#[derive(Debug, Clone)]
pub enum DeviceFlowState {
    Pending,
    Processing(time::Duration),
    Success(Credential),
    Failure(DeviceFlowError),
}

#[derive(Clone)]
pub struct DeviceFlow {
    pub host: String,
    pub client_id: String,
    pub scope: String,
    pub user_code: Option<String>,
    pub device_code: Option<String>,
    pub verification_uri: Option<String>,
    pub state: DeviceFlowState,
}

pub(crate) const FIVE_SECONDS: time::Duration = time::Duration::new(5, 0);

impl DeviceFlow {
    pub fn new(client_id: &str, maybe_host: Option<&str>, scope: Option<&str>) -> Self {
        Self {
            client_id: String::from(client_id),
            scope: match scope {
                Some(string) => String::from(string),
                None => String::new(),
            },
            host: match maybe_host {
                Some(string) => String::from(string),
                None => String::from("github.com"),
            },
            user_code: None,
            device_code: None,
            verification_uri: None,
            state: DeviceFlowState::Pending,
        }
    }

    #[cfg(feature = "blocking")]
    pub fn start(
        client_id: &str,
        maybe_host: Option<&str>,
        scope: Option<&str>,
    ) -> Result<DeviceFlow, DeviceFlowError> {
        let mut flow = DeviceFlow::new(client_id, maybe_host, scope);
        flow.setup();
        flow.started()
    }

    #[cfg(feature = "blocking")]
    pub fn setup(&mut self) {
        let (url, body) = self.setup_request();
        let res = util::post(&url, body);
        self.apply_setup_response(res);
    }

    #[cfg(feature = "blocking")]
    pub fn poll(&mut self, iterations: u32) -> Result<Credential, DeviceFlowError> {
        for count in 0..iterations {
            self.update();

            match self.next_interval(count, iterations) {
                Ok(Some(interval)) => std::thread::sleep(interval),
                Ok(None) => break,
                Err(err) => return Err(err),
            }
        }

        self.poll_result()
    }

    #[cfg(feature = "blocking")]
    pub fn update(&mut self) {
        let (url, body) = self.poll_request();
        let res = util::post(&url, body);
        self.apply_poll_response(res);
    }

    // Shared, IO-free logic used by both the blocking and async APIs.

    pub(crate) fn print_instructions(&self) {
        eprintln!(
            "Please visit {} in your browser",
            self.verification_uri.clone().unwrap()
        );
        eprintln!("And enter code: {}", self.user_code.clone().unwrap());
    }

    pub(crate) fn started(&self) -> Result<DeviceFlow, DeviceFlowError> {
        match &self.state {
            DeviceFlowState::Processing(_) => Ok(self.to_owned()),
            DeviceFlowState::Failure(err) => Err(err.to_owned()),
            _ => Err(util::credential_error(
                "Something truly unexpected happened".into(),
            )),
        }
    }

    pub(crate) fn setup_request(&self) -> (String, String) {
        (
            format!("https://{}/login/device/code", &self.host),
            format!("client_id={}&scope={}", &self.client_id, &self.scope),
        )
    }

    pub(crate) fn apply_setup_response(&mut self, res: Result<Response, reqwest::Error>) {
        let res = match res {
            Ok(res) => res,
            Err(err) => {
                self.state = DeviceFlowState::Failure(err.into());
                return;
            }
        };

        if res.contains_key("error") && res.contains_key("error_description") {
            self.state = DeviceFlowState::Failure(util::credential_error(
                res["error_description"].as_str().unwrap().into(),
            ))
        } else if res.contains_key("error") {
            self.state = DeviceFlowState::Failure(util::credential_error(format!(
                "Error response: {:?}",
                res["error"].as_str().unwrap()
            )))
        } else {
            self.user_code = Some(String::from(res["user_code"].as_str().unwrap()));
            self.device_code = Some(String::from(res["device_code"].as_str().unwrap()));
            self.verification_uri = Some(String::from(res["verification_uri"].as_str().unwrap()));
            self.state = DeviceFlowState::Processing(FIVE_SECONDS);
        }
    }

    pub(crate) fn poll_request(&self) -> (String, String) {
        (
            format!("https://{}/login/oauth/access_token", self.host),
            format!(
                "client_id={}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code",
                self.client_id,
                &self.device_code.clone().unwrap()
            ),
        )
    }

    pub(crate) fn apply_poll_response(&mut self, res: Result<Response, reqwest::Error>) {
        let res = match res {
            Ok(res) => res,
            Err(err) => {
                self.state = DeviceFlowState::Failure(err.into());
                return;
            }
        };

        if res.contains_key("error") {
            match res["error"].as_str().unwrap() {
                "authorization_pending" => {}
                "slow_down" => {
                    if let DeviceFlowState::Processing(current_interval) = self.state {
                        self.state = DeviceFlowState::Processing(current_interval + FIVE_SECONDS);
                    };
                }
                other_reason => {
                    self.state = DeviceFlowState::Failure(util::credential_error(format!(
                        "Error checking for token: {}",
                        other_reason
                    )));
                }
            }
        } else {
            self.state = DeviceFlowState::Success(credential_from(&res));
        }
    }

    /// After an update: `Ok(Some(interval))` to sleep and keep polling,
    /// `Ok(None)` to stop.
    pub(crate) fn next_interval(
        &self,
        count: u32,
        iterations: u32,
    ) -> Result<Option<time::Duration>, DeviceFlowError> {
        if let DeviceFlowState::Processing(interval) = self.state {
            if count == iterations {
                return Err(util::credential_error("Max poll iterations reached".into()));
            }
            Ok(Some(interval))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn poll_result(&self) -> Result<Credential, DeviceFlowError> {
        match &self.state {
            DeviceFlowState::Success(cred) => Ok(cred.to_owned()),
            DeviceFlowState::Failure(err) => Err(err.to_owned()),
            _ => Err(util::credential_error(
                "Unable to fetch credential, sorry :/".into(),
            )),
        }
    }
}

fn calculate_expiry(expires_in: i64) -> String {
    let expires_in = Duration::seconds(expires_in);
    let mut expiry: DateTime<Utc> = Utc::now();
    expiry = expiry + expires_in;
    expiry.to_rfc3339()
}

fn credential_from(res: &Response) -> Credential {
    let mut credential = Credential::empty();
    credential.token = res["access_token"].as_str().unwrap().to_string();

    if let Some(expires_in) = res.get("expires_in") {
        credential.expiry = calculate_expiry(expires_in.as_i64().unwrap());
        credential.refresh_token = res["refresh_token"].as_str().unwrap().to_string();
    }

    credential
}

pub(crate) fn refresh_request(
    client_id: &str,
    refresh_token: &str,
    maybe_host: Option<&str>,
    maybe_scope: Option<&str>,
) -> (String, String) {
    let host = maybe_host.unwrap_or("github.com");
    let scope = maybe_scope.unwrap_or("");

    (
        format!("https://{}/login/oauth/access_token", &host),
        format!(
            "client_id={}&refresh_token={}&client_secret=&grant_type=refresh_token&scope={}",
            &client_id, &refresh_token, &scope
        ),
    )
}

pub(crate) fn parse_refresh_response(res: Response) -> Result<Credential, DeviceFlowError> {
    if res.contains_key("error") {
        Err(util::credential_error(
            res["error"].as_str().unwrap().into(),
        ))
    } else {
        Ok(credential_from(&res))
    }
}


#[cfg(test)]
mod tests {
    use crate::{Credential};
    use chrono::offset::Utc;
    use chrono::{DateTime, Duration};

    #[test]
    fn credential_expiry_is_expired_returns_false_when_expiry_is_in_the_future() {
        let expires_in = Duration::seconds(28800);
        let mut expiry: DateTime<Utc> = Utc::now();
        expiry = expiry + expires_in;
        let calculated_expiry = expiry.to_rfc3339();

        let credential = Credential {
            token: String::from("irrelevant"),
            expiry: calculated_expiry,
            refresh_token: String::from("irrelevant"),
        };

        assert_eq!(false, credential.is_expired());
    }

    #[test]
    fn credential_expiry_is_expired_returns_true_when_expiry_is_in_the_past() {
        let expires_in = Duration::seconds(42);
        let mut expiry: DateTime<Utc> = Utc::now();
        expiry = expiry - expires_in;
        let calculated_expiry = expiry.to_rfc3339();

        let credential = Credential {
            token: String::from("irrelevant"),
            expiry: calculated_expiry,
            refresh_token: String::from("irrelevant"),
        };

        assert_eq!(true, credential.is_expired());
    }

    fn response(pairs: &[(&str, serde_json::Value)]) -> crate::Response {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn processing_flow() -> crate::DeviceFlow {
        let mut flow = crate::DeviceFlow::new("client", None, None);
        flow.device_code = Some("device".into());
        flow.state = crate::DeviceFlowState::Processing(crate::FIVE_SECONDS);
        flow
    }

    #[test]
    fn apply_setup_response_moves_to_processing() {
        let mut flow = crate::DeviceFlow::new("client", None, None);
        flow.apply_setup_response(Ok(response(&[
            ("user_code", "ABCD-1234".into()),
            ("device_code", "device".into()),
            ("verification_uri", "https://github.com/login/device".into()),
        ])));

        assert!(matches!(flow.state, crate::DeviceFlowState::Processing(_)));
        assert_eq!(flow.user_code.as_deref(), Some("ABCD-1234"));
        assert!(flow.started().is_ok());
    }

    #[test]
    fn apply_setup_response_records_error_description() {
        let mut flow = crate::DeviceFlow::new("client", None, None);
        flow.apply_setup_response(Ok(response(&[
            ("error", "bad".into()),
            ("error_description", "nope".into()),
        ])));

        assert!(matches!(
            flow.started(),
            Err(crate::DeviceFlowError::GitHubError(msg)) if msg == "nope"
        ));
    }

    #[test]
    fn apply_poll_response_keeps_waiting_while_pending() {
        let mut flow = processing_flow();
        flow.apply_poll_response(Ok(response(&[("error", "authorization_pending".into())])));

        assert_eq!(flow.next_interval(0, 20).unwrap(), Some(crate::FIVE_SECONDS));
    }

    #[test]
    fn apply_poll_response_backs_off_on_slow_down() {
        let mut flow = processing_flow();
        flow.apply_poll_response(Ok(response(&[("error", "slow_down".into())])));

        assert_eq!(
            flow.next_interval(0, 20).unwrap(),
            Some(crate::FIVE_SECONDS + crate::FIVE_SECONDS)
        );
    }

    #[test]
    fn apply_poll_response_fails_on_other_errors() {
        let mut flow = processing_flow();
        flow.apply_poll_response(Ok(response(&[("error", "access_denied".into())])));

        assert_eq!(flow.next_interval(0, 20).unwrap(), None);
        assert!(flow.poll_result().is_err());
    }

    #[test]
    fn apply_poll_response_returns_credential_on_success() {
        let mut flow = processing_flow();
        flow.apply_poll_response(Ok(response(&[
            ("access_token", "tok".into()),
            ("expires_in", 28800.into()),
            ("refresh_token", "ref".into()),
        ])));

        let cred = flow.poll_result().unwrap();
        assert_eq!(cred.token, "tok");
        assert_eq!(cred.refresh_token, "ref");
        assert!(!cred.is_expired());
    }

    #[test]
    fn parse_refresh_response_handles_success_and_error() {
        let ok = crate::parse_refresh_response(response(&[("access_token", "tok".into())])).unwrap();
        assert_eq!(ok.token, "tok");
        assert_eq!(ok.expiry, "");

        assert!(crate::parse_refresh_response(response(&[("error", "bad_refresh_token".into())]))
            .is_err());
    }
}
