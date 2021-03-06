use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crate::error::Error;

pub type Id = u64;

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
        id: Id,
        success: bool,

        #[serde(flatten)]
        data: ResultBody,
    },

    // Subscribe Events
    SubscribeEvents { id: Id, event_type: Option<EventType> },
    Event {
        id: Id,
        event: EventObj,
    },
    UnsubscribeEvents {id: Id, subscription: Id },
    FireEvent {
        id: Id,
        event_type: EventType,
        event_data: Option<serde_json::Value>,
    },

    // Calling a service
    // TODO: https://developers.home-assistant.io/docs/api/websocket/#calling-a-service

    // Fetching states
    GetStates { id: Id },
    // TODO: this provides a list in "result", not an object

    // Pings and Pongs
    Ping { id: Id },
    Pong { id: Id },

}

impl WsMessage {

    /// Creates a new `WsMessage::Result` with the given `id`,
    /// the `success` field set to `true`, and data set to an
    /// empty `ResultBody::Result`.
    pub fn new_result_success(id: Id) -> WsMessage {
        WsMessage::Result {
            id,
            success: true,
            data: ResultBody::Result {
                result: None
            }
        }
    }

    /// Retrieves the `Id` associated to the message, if any.
    pub fn id(&self) -> Option<Id> {
        use WsMessage::*;
        match self {
            //  Variants with an Id
            Result { id, .. } => Some(*id),
            SubscribeEvents { id, .. } => Some(*id),
            UnsubscribeEvents { id, .. } => Some(*id),
            Event { id, .. } => Some(*id),
            FireEvent { id, .. } => Some(*id),
            GetStates { id } => Some(*id),
            Ping { id } => Some(*id),
            Pong { id } => Some(*id),

            //  Variants without
            //* (avoid `_ => None` to get compile errors when missing some variants)
            Auth { .. } => None,
            AuthInvalid { .. } => None,
            AuthOk { .. } => None,
            AuthRequired { .. } => None,
        }
    }

    /// Sets a new `Id` associated to the message, if possible, otherwise return
    /// the message as-is.
    pub fn set_id(self, new_id: Id) -> WsMessage {
        use WsMessage::*;
        match self {
            //  Variants with an Id
            Result { success, data, .. } => {
                Result { id: new_id, success, data }
            },
            SubscribeEvents { event_type, .. } => {
                SubscribeEvents { id: new_id, event_type }
            },
            UnsubscribeEvents { subscription, .. } => {
                UnsubscribeEvents { id: new_id, subscription }
            },
            Event { event, .. } => {
                Event { id: new_id, event}
            },
            FireEvent { event_data, event_type, .. } => {
                FireEvent { id: new_id, event_data, event_type}
            },
            GetStates { .. } => {
                GetStates { id: new_id }
            },
            Ping { .. } => {
                Ping { id: new_id }
            },
            Pong { .. } => {
                Pong { id: new_id }
            },

            //  Variants without
            //* (avoid `_ => self` to get compile errors when missing some variants)
            Auth { .. } => self,
            AuthInvalid { .. } => self,
            AuthOk { .. } => self,
            AuthRequired { .. } => self,
        }
    }

}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub enum ResultBody {
    Result { result: Option<ResultObject> },
    Error { error: ErrorObject },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(untagged)]
pub enum ResultObject {
    Object { context: ContextObject },
    Array(Vec<serde_json::Value>),
}

#[derive(Serialize, Deserialize, Default, PartialEq, Eq, Debug)]
pub struct ContextObject {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ErrorObject {
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(untagged)]
pub enum EventObj {
    // https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-events
    Event {
        data: serde_json::Value,
        event_type: EventType,
        time_fired: DateTime<Utc>,
        origin: String,
        context: ContextObject,
    },
    // https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-trigger
    Trigger {
        variables: serde_json::Value,
        context: ContextObject,
    },
}

/// Event types as described on the Home Assistant webiste at
/// https://www.home-assistant.io/docs/configuration/events/
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
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

    HaevloStart,
    HaevloStop,

    #[serde(other)]
    Unknown,
}

fn fmt_json(f: &mut fmt::Formatter<'_>, obj: &impl Serialize) -> fmt::Result {
    match serde_json::to_string(&obj) {
        Ok(s) => write!(f, "{}", s),
        Err(_) => Err(fmt::Error),
    }
}

impl fmt::Display for WsMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_json(f, &self)
    }
}

impl Default for EventType {
    fn default() -> Self { EventType::Unknown }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_json(f,&self)
    }
}

/// Serialize as JSON the given `WsMessage`.
pub fn serialize(msg: &WsMessage) -> Result<String, Error> {
    Ok(serde_json::to_string(&msg)?)
}

/// Deserialize from JSON a message.
pub fn deserialize(json: &str) -> Result<WsMessage, Error> {
    Ok(serde_json::from_str(json)?)
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
            id: 18,
            success: false,
            data: ResultBody::Result {
                result: None,
            }
        },
        "{
            \"id\": 18,
            \"type\": \"result\",
            \"success\": false,
            \"result\": null
        }");

    serde_test!(msg_auth_result_object,
        WsMessage::Result {
            id: 18,
            success: true,
            data: ResultBody::Result {
                result: Some(ResultObject::Object {
                    context: ContextObject {
                        id: String::from("326ef27d19415c60c492fe330945f954"),
                        parent_id: None,
                        user_id: Some(String::from("31ddb597e03147118cf8d2f8fbea5553"))
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

        serde_test!(msg_auth_result_array,
            WsMessage::Result {
                id: 18,
                success: true,
                data: ResultBody::Result {
                    result: Some(ResultObject::Array(vec![
                        serde_json::from_str("{\"some_field\": \"some_data\"}").unwrap(),
                        serde_json::from_str("{\"some_field\": \"some_other_data\"}").unwrap()
                    ])),
                }
            },
            "{
                \"id\": 18,
                \"type\": \"result\",
                \"success\": true,
                \"result\": [
                    {\"some_field\": \"some_data\"},
                    {\"some_field\": \"some_other_data\"}
                ]
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

    serde_test!(msg_subscribe_events,
        WsMessage::SubscribeEvents { id: 18, event_type: Some(EventType::StateChanged), },
        "{ \"id\": 18, \"type\": \"subscribe_events\", \"event_type\": \"state_changed\" }");

    serde_test!(msg_event,
        WsMessage::Event {
            id: 18,
            event: EventObj::Event {
                data: serde_json::from_str("{\"some_field\": \"some_data\"}").unwrap(),
                event_type: EventType::StateChanged,
                time_fired: DateTime::from(DateTime::parse_from_rfc3339("2022-01-09T10:33:04.391956+01:00").unwrap()),
                origin: String::from("LOCAL"),
                context: ContextObject {
                    id: String::from("9b263f9e4e899819a0515a97f6ddfb47"),
                    ..Default::default()
                },
            }
        },
        "{ \"id\": 18, \"type\": \"event\", \"event\": {
            \"data\": {\"some_field\": \"some_data\"},
            \"event_type\": \"state_changed\",
            \"time_fired\": \"2022-01-09T15:33:04.391956+06:00\",
            \"origin\": \"LOCAL\",
            \"context\": {
                \"id\": \"9b263f9e4e899819a0515a97f6ddfb47\"
            }
        }}");

    serde_test!(msg_unsubscribe_event,
        WsMessage::UnsubscribeEvents { id: 345, subscription: 234},
        "{\"id\": 345, \"type\": \"unsubscribe_events\", \"subscription\": 234}");

    serde_test!(msg_fire_event,
        WsMessage::FireEvent {
            id: 56412,
            event_type: EventType::HomeassistantStarted,
            event_data: None,
        },
        "{\"id\": 56412, \"type\": \"fire_event\",\"event_type\": \"homeassistant_started\"}");


    serde_test!(msg_get_states,
        WsMessage::GetStates { id: 78923 },
        "{\"id\": 78923, \"type\": \"get_states\"}");

    serde_test!(msg_ping,
        WsMessage::Ping { id: 789423 },
        "{\"id\": 789423, \"type\": \"ping\"}");

    serde_test!(msg_pong,
        WsMessage::Pong { id: 789423 },
        "{\"id\": 789423, \"type\": \"pong\"}");
}