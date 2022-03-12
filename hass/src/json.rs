use serde::{Serialize, Deserialize};
use crate::error::Error;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    //#[serde(rename="type")]
    //msg_type: String,

    AuthRequired { ha_version: String },
    Auth { access_token: String },
    AuthOk { ha_version: String },
}

impl ToString for WsMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

impl From<&str> for WsMessage {
    fn from(s: &str) -> Self {
        serde_json::from_str(&s).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;
    use super::{*, WsMessage::*};

    fn log_and_check(original : &WsMessage, serialized: &str) {
        tracing::debug!("{:?} ~~> {}", original, serialized);
        let deserialized : WsMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, *original);
    }

    #[test]
    #[traced_test]
    fn msg_auth_required() {
        let item = AuthRequired { ha_version: String::from("2022.3.3") };
        let serialized = serde_json::to_string(&item).unwrap();
        log_and_check(&item, &serialized);
    }

    #[test]
    #[traced_test]
    fn wsmessage_trait_tostring() {
        let item = Auth { access_token: String::from("abc123") };
        let serialized = item.to_string();
        log_and_check(&item, &serialized);
    }

    #[test]
    #[traced_test]
    fn wsmessage_trait_from() {
        let item = AuthOk { ha_version: String::from("2022.4.1") };
        let from = WsMessage::from("{\"type\": \"auth_ok\", \"ha_version\": \"2022.4.1\"}");
        tracing::debug!("item={:?} from=%{:?}", item, from);
        assert_eq!(item, from);
    }
}