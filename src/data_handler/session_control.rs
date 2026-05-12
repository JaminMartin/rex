use crate::data_handler::{ServerState, SessionSnapshot, StreamSnapshot};
use std::sync::{Arc, Mutex};
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
}

impl SessionController {
    pub fn new(
        state: Arc<AsyncMutex<ServerState>>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        Self { state, shutdown_tx }
    }

    pub async fn stream_snapshot(&self, max_data_points_override: Option<usize>) -> StreamSnapshot {
        let state = self.state.lock().await;
        StreamSnapshot::from_server_state(&state, max_data_points_override)
    }

    pub async fn session_snapshot(&self) -> Result<SessionSnapshot, String> {
        let state = self.state.lock().await;
        SessionSnapshot::from_server_state(&state)
            .ok_or("No session summary available".to_string())
    }

    pub async fn pause(&self) -> String {
        let mut state = self.state.lock().await;
        state.internal_state = false;
        "Setting internal server state to paused...".to_string()
    }

    pub async fn resume(&self) -> String {
        let mut state = self.state.lock().await;
        state.internal_state = true;
        "Setting internal server state to start...".to_string()
    }

    pub async fn kill(&self) -> String {
        let _ = self.shutdown_tx.send(());
        "Shutting down server...".to_string()
    }

}

#[derive(Clone, Default)]
pub struct SharedSessionController {
    inner: Arc<Mutex<Option<SessionController>>>,
}

impl SharedSessionController {
    pub fn set(&self, controller: SessionController) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(controller);
        }
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
    use crate::data_handler::{DataSession, Entity, SessionInfo};
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
            true,
            100,
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
            true,
            100,
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
            true,
            100,
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
}
