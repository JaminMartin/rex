use crate::data_handler::{
    Entity, ServerEvent, ServerState, SessionSnapshot, StreamProjectionConfig, StreamSnapshot,
};
use std::sync::{atomic::{AtomicU64, Ordering}, Arc, Mutex};
use tokio::sync::{broadcast, Mutex as AsyncMutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerCommand {
    GetDatastream,
    State,
    Pause,
    Resume,
    Kill,
}

impl ServerCommand {
    pub fn parse(command: &str) -> Result<Self, String> {
        match command.trim() {
            "GET_DATASTREAM" => Ok(Self::GetDatastream),
            "STATE" => Ok(Self::State),
            "PAUSE_STATE" => Ok(Self::Pause),
            "RESUME_STATE" => Ok(Self::Resume),
            "KILL" => Ok(Self::Kill),
            other => Err(format!("Unknown command: {other}")),
        }
    }
}

#[derive(Clone)]
pub struct SessionController {
    state: Arc<AsyncMutex<ServerState>>,
    shutdown_tx: broadcast::Sender<()>,
    events: broadcast::Sender<ServerEvent>,
    revision: Arc<AtomicU64>,
}

impl SessionController {
    pub fn new(
        state: Arc<AsyncMutex<ServerState>>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        let (events, _) = broadcast::channel(64);
        Self::with_events(state, shutdown_tx, events, Arc::new(AtomicU64::new(0)))
    }

    pub fn with_events(
        state: Arc<AsyncMutex<ServerState>>,
        shutdown_tx: broadcast::Sender<()>,
        events: broadcast::Sender<ServerEvent>,
        revision: Arc<AtomicU64>,
    ) -> Self {
        Self { state, shutdown_tx, events, revision }
    }

    pub async fn stream_snapshot(&self, max_data_points_override: Option<usize>) -> StreamSnapshot {
        let state = self.state.lock().await;
        StreamSnapshot::from_server_state(&state, max_data_points_override)
    }

    pub async fn stream_snapshot_with_projection(
        &self,
        projection: Option<StreamProjectionConfig>,
    ) -> StreamSnapshot {
        let state = self.state.lock().await;
        match projection {
            Some(projection) => StreamSnapshot::from_server_state_with_projection(&state, &projection),
            None => StreamSnapshot::from_server_state(&state, None),
        }
    }

    pub async fn session_snapshot(&self) -> Result<SessionSnapshot, String> {
        let state = self.state.lock().await;
        SessionSnapshot::from_server_state(&state)
            .ok_or("No session summary available".to_string())
    }

    pub async fn pause(&self) -> String {
        {
            let mut state = self.state.lock().await;
            state.internal_state = false;
        }
        self.publish_state_changed().await;
        "Setting internal server state to paused...".to_string()
    }

    pub async fn resume(&self) -> String {
        {
            let mut state = self.state.lock().await;
            state.internal_state = true;
        }
        self.publish_state_changed().await;
        "Setting internal server state to start...".to_string()
    }

    pub async fn kill(&self) -> String {
        let _ = self.shutdown_tx.send(());
        "Shutting down server...".to_string()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.events.subscribe()
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    pub async fn session_id(&self) -> uuid::Uuid {
        self.state.lock().await.uuid
    }

    fn next_revision(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub async fn ingest(&self, key: String, entity: Entity) {
        let is_device = matches!(entity, Entity::Device(_));
        {
            let mut state = self.state.lock().await;
            state.update_entity(key, entity);
        }
        let revision = self.next_revision();
        if is_device {
            let stream = self.stream_snapshot(None).await;
            let session = self.session_snapshot().await.ok();
            let _ = self.events.send(ServerEvent::StreamUpdated { revision, stream, session });
        } else {
            self.publish_state_changed_with_revision(revision).await;
        }
    }

    pub async fn publish_started(&self) {
        let session_id = self.session_id().await;
        let _ = self.events.send(ServerEvent::RunStarted {
            revision: self.next_revision(),
            session_id,
        });
    }

    pub async fn publish_finished(&self) {
        let session_id = Some(self.state.lock().await.uuid);
        let _ = self.events.send(ServerEvent::RunFinished {
            revision: self.next_revision(),
            session_id,
        });
    }

    async fn publish_state_changed(&self) {
        self.publish_state_changed_with_revision(self.next_revision()).await;
    }

    async fn publish_state_changed_with_revision(&self, revision: u64) {
        let state = self.state.lock().await;
        let session = SessionSnapshot::from_server_state(&state);
        let _ = self.events.send(ServerEvent::SessionStateChanged {
            revision,
            session_running: session.is_some(),
            paused: !state.internal_state,
            session,
        });
    }

}

#[derive(Clone)]
pub struct SharedSessionController {
    inner: Arc<Mutex<Option<SessionController>>>,
    events: broadcast::Sender<ServerEvent>,
    revision: Arc<AtomicU64>,
}

impl Default for SharedSessionController {
    fn default() -> Self {
        let (events, _) = broadcast::channel(64);
        Self {
            inner: Arc::new(Mutex::new(None)),
            events,
            revision: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl SharedSessionController {
    pub fn set(&self, controller: SessionController) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(controller);
        }
    }

    pub fn new_session_controller(
        &self,
        state: Arc<AsyncMutex<ServerState>>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> SessionController {
        SessionController::with_events(state, shutdown_tx, self.events.clone(), self.revision.clone())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.events.subscribe()
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = None;
        }
    }

    pub fn get(&self) -> Option<SessionController> {
        self.inner.lock().ok().and_then(|guard| guard.clone())
    }

    fn require_controller(&self) -> Result<SessionController, String> {
        self.get().ok_or("No active session".to_string())
    }

    pub async fn stream_snapshot(
        &self,
        max_data_points_override: Option<usize>,
    ) -> Result<StreamSnapshot, String> {
        Ok(self
            .require_controller()?
            .stream_snapshot(max_data_points_override)
            .await)
    }

    pub async fn session_snapshot(&self) -> Result<SessionSnapshot, String> {
        self.require_controller()?.session_snapshot().await
    }

    pub async fn pause(&self) -> Result<String, String> {
        Ok(self.require_controller()?.pause().await)
    }

    pub async fn resume(&self) -> Result<String, String> {
        Ok(self.require_controller()?.resume().await)
    }

    pub async fn kill(&self) -> Result<String, String> {
        Ok(self.require_controller()?.kill().await)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_handler::{
        DataSession, Device, Entity, Measurement, MeasurementData, SessionInfo,
        StreamProjectionConfig,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_parse_server_command() {
        assert_eq!(ServerCommand::parse("GET_DATASTREAM\n").unwrap(), ServerCommand::GetDatastream);
        assert_eq!(ServerCommand::parse("STATE").unwrap(), ServerCommand::State);
        assert!(ServerCommand::parse("wat").is_err());
    }

    #[tokio::test]
    async fn test_session_controller_pause_resume() {
        let state = Arc::new(AsyncMutex::new(ServerState::new(
            Uuid::new_v4(),
            String::new(),
            "script.rs".to_string(),
            StreamProjectionConfig::default(),
        )));
        let (shutdown_tx, _) = broadcast::channel(1);
        let controller = SessionController::new(Arc::clone(&state), shutdown_tx);

        controller.pause().await;
        assert!(!state.lock().await.internal_state);

        controller.resume().await;
        assert!(state.lock().await.internal_state);
    }

    #[tokio::test]
    async fn test_session_controller_state_serializes_summary() {
        let mut server_state = ServerState::new(
            Uuid::new_v4(),
            String::new(),
            "script.rs".to_string(),
            StreamProjectionConfig::default(),
        );
        server_state.update_entity(
            "session".to_string(),
            Entity::Session(DataSession {
                start_time: None,
                end_time: None,
                uuid: None,
                info: SessionInfo {
                    name: "name".to_string(),
                    email: "email".to_string(),
                    session_name: "session_name".to_string(),
                    session_description: "desc".to_string(),
                    meta: None,
                },
            }),
        );

        let state = Arc::new(AsyncMutex::new(server_state));
        let (shutdown_tx, _) = broadcast::channel(1);
        let controller = SessionController::new(state, shutdown_tx);

        let summary = controller.session_snapshot().await.unwrap();
        assert_eq!(summary.session_info.session_name, "session_name");
        assert_eq!(summary.run_file, "script.rs");
    }

    #[tokio::test]
    async fn test_session_controller_stream_snapshot() {
        let mut server_state = ServerState::new(
            Uuid::new_v4(),
            String::new(),
            "script.rs".to_string(),
            StreamProjectionConfig::default(),
        );
        server_state.update_entity(
            "session".to_string(),
            Entity::Session(DataSession {
                start_time: None,
                end_time: None,
                uuid: None,
                info: SessionInfo {
                    name: "name".to_string(),
                    email: "email".to_string(),
                    session_name: "session_name".to_string(),
                    session_description: "desc".to_string(),
                    meta: None,
                },
            }),
        );

        let state = Arc::new(AsyncMutex::new(server_state));
        let (shutdown_tx, _) = broadcast::channel(1);
        let controller = SessionController::new(state, shutdown_tx);

        let snapshot = controller.stream_snapshot(None).await;
        assert!(snapshot.session_running);
        assert!(!snapshot.paused);
    }

    #[tokio::test]
    async fn test_session_controller_publishes_ingestion_events() {
        let state = Arc::new(AsyncMutex::new(ServerState::default()));
        let (shutdown_tx, _) = broadcast::channel(1);
        let controller = SessionController::new(state, shutdown_tx);
        let mut events = controller.subscribe();

        controller
            .ingest(
                "session".to_string(),
                Entity::Session(DataSession {
                    start_time: None,
                    end_time: None,
                    uuid: None,
                    info: SessionInfo::default(),
                }),
            )
            .await;
        assert!(matches!(
            events.recv().await.unwrap(),
            ServerEvent::SessionStateChanged { .. }
        ));

        controller
            .ingest(
                "device".to_string(),
                Entity::Device(Device {
                    device_name: "device".to_string(),
                    device_config: HashMap::new(),
                    measurements: HashMap::from([(
                        "value".to_string(),
                        Measurement {
                            data: MeasurementData::Single(vec![1.0]),
                            unit: "V".to_string(),
                        },
                    )]),
                    timestamps: HashMap::new(),
                }),
            )
            .await;
        assert!(matches!(
            events.recv().await.unwrap(),
            ServerEvent::StreamUpdated { .. }
        ));
    }
}
