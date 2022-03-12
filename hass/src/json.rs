use serde::{Serialize, Deserialize};
use crate::error::Error;

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
    SubscribeEvents { id: u64, event_type: String },

}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ResultBody {
    pub id: u64,
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
    ScriptStarted
}

/***** TESTS *****************************************************************/

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;
    use super::*;

    fn log_and_check(val: &WsMessage, json: &str) {
        tracing::debug!("{:?} <~~> {}", val, json);
        let deserialized : WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, *val);
        let serialized = serde_json::to_string(&val).unwrap();
        let roundtrip = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(serialized, roundtrip);
    }

    #[test]
    #[traced_test]
    fn msg_auth_required() {
        let j = "{
            \"type\": \"auth_required\",
            \"ha_version\": \"2021.5.3\"
        }";
        let original = WsMessage::AuthRequired { ha_version: String::from("2021.5.3") };
        log_and_check(&original, j);
    }

    #[test]
    #[traced_test]
    fn msg_auth() {
        let j = "{
            \"type\": \"auth\",
            \"access_token\": \"ABCDEFGH\"
        }";
        let original = WsMessage::Auth { access_token: String::from("ABCDEFGH") };
        log_and_check(&original, j);
    }

    #[test]
    #[traced_test]
    fn msg_auth_ok() {
        let j = "{
            \"type\": \"auth_ok\",
            \"ha_version\": \"2021.5.3\"
        }";
        let original = WsMessage::AuthOk { ha_version: String::from("2021.5.3") };
        log_and_check(&original, j);
    }

    #[test]
    #[traced_test]
    fn msg_auth_invalid() {
        let j = "{
            \"type\": \"auth_invalid\",
            \"message\": \"Invalid password\"
        }";
        let original = WsMessage::AuthInvalid { message: String::from("Invalid password") };
        log_and_check(&original, j);
    }

    #[test]
    #[traced_test]
    fn msg_auth_result_simple() {
        let j = "{
            \"id\": 18,
            \"type\": \"result\",
            \"success\": true,
            \"result\": null
          }";
        let original = WsMessage::Result {
            data: ResultBody {
                id: 18,
                success: true,
                result: None,
            }
        };
        log_and_check(&original, j);
    }

    #[test]
    #[traced_test]
    fn msg_auth_result_object() {
        let j = "{
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
          }";
        let original = WsMessage::Result {
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
        };
        log_and_check(&original, j);
    }

    #[test]
    #[traced_test]
    fn event_type() {
        fn et_test(e: EventType, expected: &str) {
            let json = serde_json::to_string(&e).unwrap();
            assert_eq!(&json, &format!("\"{}\"", expected));
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
}