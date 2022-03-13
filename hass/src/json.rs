use serde::{Serialize, Deserialize};
use crate::error::Error;

type Id = u64;

/// WebSocket message format for Home Assistant, as described at
/// https://developers.home-assistant.io/docs/api/websocket/
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {

    // Authentication
    AuthRequired { ha_version: String },
    Auth { access_token: String },
    AuthOk { ha_version: String },
    AuthInvalid { message: String },

    // Command Phase
    Result {
        #[serde(flatten)]
        data: ResultBody,
    },

    // Subscribe Events
    SubscribeEvents { id: Id, event_type: Option<EventType> },

}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ResultBody {
    pub id: Id,
    pub success: bool,
    pub result: Option<ResultObject>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ResultObject {
    pub context: ContextObject,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ContextObject {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: String,
}

/// Event types as described on the Home Assistant webiste at
/// https://www.home-assistant.io/docs/configuration/events/
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    CallService,
    ComponentLoaded,
    CoreConfigUpdated,
    DataEntryFlowProgressed,
    HomeassistantStart,
    HomeassistantStarted,
    HomeassistantStop,
    HomeassistantFinalWrite,
    HomeassistantClose,
    LogbookEntry,
    ServiceRegistered,
    ServiceRemoved,
    StateChanged,
    ThemesUpdated,
    TimerOutOfSync,
    TimeChanged,
    UserAdded,
    UserRemoved,
    AutomationReloaded,
    AutomationTriggered,
    SceneReloaded,
    ScriptStarted,

    #[serde(other)]
    Unknown,
}

/// Serialize as JSON the given `WsMessage`.
pub fn serialize(msg: &WsMessage) -> Result<String, Error> {
    Ok(serde_json::to_string(&msg)?)
}

/// Deserialize from JSON a message.
pub fn deserialize(json: &str) -> Result<WsMessage, Error> {
    Ok(serde_json::from_str(&json)?)
}

/***** TESTS *****************************************************************/

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;
    use super::*;

    fn log_and_check(val: &WsMessage, json: &str) {
        tracing::debug!("{:?} <~~> {}", val, json);
        let deserialized : WsMessage = deserialize(&json).unwrap();
        assert_eq!(deserialized, *val);
        let serialized = serialize(&val).unwrap();
        let roundtrip = serialize(&deserialized).unwrap();
        assert_eq!(serialized, roundtrip);
    }

    macro_rules! serde_test {
        ( $name:ident, $msg:expr, $json:expr) => {
            #[test]
            #[traced_test]
            fn $name() {
                log_and_check(&$msg, $json);
            }
        };
    }

    serde_test!(msg_auth_required,
        WsMessage::AuthRequired { ha_version: String::from("2021.5.3") },
        "{ \"type\": \"auth_required\", \"ha_version\": \"2021.5.3\" }");

    serde_test!(msg_auth,
        WsMessage::Auth { access_token: String::from("ABCDEFGH") },
        "{ \"type\": \"auth\", \"access_token\": \"ABCDEFGH\" }");

    serde_test!(msg_auth_ok,
        WsMessage::AuthOk { ha_version: String::from("2021.5.3") },
        "{ \"type\": \"auth_ok\",  \"ha_version\": \"2021.5.3\" }");

    serde_test!(msg_auth_invalid,
        WsMessage::AuthInvalid { message: String::from("Invalid password") },
        "{\"type\": \"auth_invalid\", \"message\": \"Invalid password\"}");

    serde_test!(msg_auth_result_simple,
        WsMessage::Result {
            data: ResultBody {
                id: 18,
                success: true,
                result: None,
            }
        },
        "{
            \"id\": 18,
            \"type\": \"result\",
            \"success\": true,
            \"result\": null
        }");

    serde_test!(msg_auth_result_object,
        WsMessage::Result {
            data: ResultBody {
                id: 18,
                success: true,
                result: Some(ResultObject {
                    context: ContextObject {
                        id: String::from("326ef27d19415c60c492fe330945f954"),
                        parent_id: None,
                        user_id: String::from("31ddb597e03147118cf8d2f8fbea5553")
                    }
                }),
            }
        },
        "{
            \"id\": 18,
            \"type\": \"result\",
            \"success\": true,
            \"result\": {
                \"context\": {
                    \"id\": \"326ef27d19415c60c492fe330945f954\",
                    \"parent_id\": null,
                    \"user_id\": \"31ddb597e03147118cf8d2f8fbea5553\"
                }
            }
        }");

    #[test]
    #[traced_test]
    fn event_type() {
        fn et_test(e: EventType, expected: &str) {
            let json = serde_json::to_string(&e).unwrap();
            assert_eq!(&json, &format!("\"{}\"", expected));
            let ev : EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(e, ev);
        }
        et_test(EventType::CallService, "call_service");
        et_test(EventType::ComponentLoaded, "component_loaded");
        et_test(EventType::CoreConfigUpdated, "core_config_updated");
        et_test(EventType::DataEntryFlowProgressed, "data_entry_flow_progressed");
        et_test(EventType::HomeassistantStart, "homeassistant_start");
        et_test(EventType::HomeassistantStarted, "homeassistant_started");
        et_test(EventType::HomeassistantStop, "homeassistant_stop");
        et_test(EventType::HomeassistantFinalWrite, "homeassistant_final_write");
        et_test(EventType::HomeassistantClose, "homeassistant_close");
        et_test(EventType::LogbookEntry, "logbook_entry");
        et_test(EventType::ServiceRegistered, "service_registered");
        et_test(EventType::ServiceRemoved, "service_removed");
        et_test(EventType::StateChanged, "state_changed");
        et_test(EventType::ThemesUpdated, "themes_updated");
        et_test(EventType::TimerOutOfSync, "timer_out_of_sync");
        et_test(EventType::TimeChanged, "time_changed");
        et_test(EventType::UserAdded, "user_added");
        et_test(EventType::UserRemoved, "user_removed");
        et_test(EventType::AutomationReloaded, "automation_reloaded");
        et_test(EventType::AutomationTriggered, "automation_triggered");
        et_test(EventType::SceneReloaded, "scene_reloaded");
        et_test(EventType::ScriptStarted, "script_started");
    }

    #[test]
    #[traced_test]
    fn event_type_unknown() {
        let unknown = "\"an_unknown_event\"";
        let deserialized : EventType = serde_json::from_str(&unknown).unwrap();
        assert_eq!(&deserialized, &EventType::Unknown);
    }
}