use crate::data_handler::{DeviceData, ServerState, SessionInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;

pub const LIVE_PROTOCOL_VERSION: u16 = 1;

/// Messages a live client may send after opening `/ws`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe {
        projection: Option<crate::data_handler::StreamProjectionConfig>,
    },
    Ping,
}

/// Versioned, server-originated messages shared by terminal and browser clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Hello {
        protocol_version: u16,
        server_version: String,
        session_id: Option<uuid::Uuid>,
    },
    Snapshot {
        revision: u64,
        session: Option<SessionSnapshot>,
        stream: Option<StreamSnapshot>,
    },
    RunStarted {
        revision: u64,
        session_id: uuid::Uuid,
    },
    StreamUpdated {
        revision: u64,
        stream: StreamSnapshot,
        session: Option<SessionSnapshot>,
    },
    SessionStateChanged {
        revision: u64,
        session_running: bool,
        paused: bool,
        session: Option<SessionSnapshot>,
    },
    RunFinished {
        revision: u64,
        session_id: Option<uuid::Uuid>,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamSnapshot {
    pub devices: HashMap<String, DeviceData>,
    pub session_running: bool,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSnapshot {
    pub session_info: SessionInfo,
    pub devices: HashMap<String, HashMap<String, Value>>,
    pub run_file: String,
    pub session_running: bool,
    pub paused: bool,
}

impl StreamSnapshot {
    pub fn from_server_state(state: &ServerState, max_data_points_override: Option<usize>) -> Self {
        Self {
            devices: state.send_stream(max_data_points_override),
            session_running: state.get_session_info().is_some(),
            paused: !state.internal_state,
        }
    }

    pub fn from_server_state_with_projection(
        state: &ServerState,
        projection: &crate::data_handler::StreamProjectionConfig,
    ) -> Self {
        Self {
            devices: state.send_stream_with_projection(projection),
            session_running: state.get_session_info().is_some(),
            paused: !state.internal_state,
        }
    }
}

impl SessionSnapshot {
    pub fn from_server_state(state: &ServerState) -> Option<Self> {
        let session_info = state.get_session_info()?.info.clone();
        let devices = state.device_configs();

        Some(Self {
            session_info,
            devices,
            run_file: state.run_file.clone(),
            session_running: true,
            paused: !state.internal_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_handler::{
        DataSession, Device, Entity, Measurement, MeasurementData, SessionMetadata,
        StreamProjectionConfig,
    };
    use uuid::Uuid;

    fn build_server_state() -> ServerState {
        let mut state = ServerState::new(
            Uuid::new_v4(),
            String::new(),
            "example.rs".to_string(),
            StreamProjectionConfig::default(),
        );

        state.update_entity(
            "session".to_string(),
            Entity::Session(DataSession {
                start_time: None,
                end_time: None,
                uuid: None,
                info: SessionInfo {
                    name: "Operator".to_string(),
                    email: "operator@example.com".to_string(),
                    session_name: "session-a".to_string(),
                    session_description: "desc".to_string(),
                    meta: Some(SessionMetadata {
                        meta: HashMap::from([(String::from("sample"), Value::String("A1".to_string()))]),
                    }),
                },
            }),
        );

        state.update_entity(
            "device-a".to_string(),
            Entity::Device(Device {
                device_name: "device-a".to_string(),
                device_config: HashMap::from([(String::from("gain"), Value::Integer(10))]),
                measurements: HashMap::from([
                    (
                        String::from("x"),
                        Measurement {
                            data: MeasurementData::Single(vec![1.0, 2.0, 3.0]),
                            unit: "s".to_string(),
                        },
                    ),
                    (
                        String::from("y"),
                        Measurement {
                            data: MeasurementData::Single(vec![2.0, 4.0, 6.0]),
                            unit: "V".to_string(),
                        },
                    ),
                ]),
                timestamps: HashMap::from([
                    (
                        String::from("x"),
                        vec![
                            "2024-01-01T00:00:00Z".to_string(),
                            "2024-01-01T00:00:01Z".to_string(),
                            "2024-01-01T00:00:02Z".to_string(),
                        ],
                    ),
                    (
                        String::from("y"),
                        vec![
                            "2024-01-01T00:00:00Z".to_string(),
                            "2024-01-01T00:00:01Z".to_string(),
                            "2024-01-01T00:00:02Z".to_string(),
                        ],
                    ),
                ]),
            }),
        );

        state
    }

    #[test]
    fn test_session_snapshot_from_server_state() {
        let state = build_server_state();
        let snapshot = SessionSnapshot::from_server_state(&state).unwrap();

        assert_eq!(snapshot.session_info.session_name, "session-a");
        assert_eq!(snapshot.run_file, "example.rs");
        assert!(snapshot.session_running);
        assert!(!snapshot.paused);
        assert_eq!(snapshot.devices["device-a"]["gain"], Value::Integer(10));
    }

    #[test]
    fn test_stream_snapshot_from_server_state() {
        let state = build_server_state();
        let snapshot = StreamSnapshot::from_server_state(&state, None);

        assert!(snapshot.session_running);
        assert!(!snapshot.paused);
        assert!(snapshot.devices.contains_key("device-a"));
        assert!(snapshot.devices["device-a"].measurements.contains_key("x"));
    }

    #[test]
    fn test_session_snapshot_missing_session_returns_none() {
        let state = ServerState::new(
            Uuid::new_v4(),
            String::new(),
            "example.rs".to_string(),
            StreamProjectionConfig::default(),
        );
        assert!(SessionSnapshot::from_server_state(&state).is_none());
    }

    #[test]
    fn test_stream_snapshot_respects_override_max_points() {
        let state = build_server_state();
        let snapshot = StreamSnapshot::from_server_state(&state, Some(2));

        assert_eq!(snapshot.devices["device-a"].measurements["x"].len(), 2);
    }
}
