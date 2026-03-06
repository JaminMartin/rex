use crate::data_handler::data_mod::get_configuration;
use crate::data_handler::transport::Transport;
use crate::data_handler::DeviceData;
use crate::tui_tool::action::Action;
use crate::tui_tool::tabs::{chart::ChartTab, state::StateTab};
use crate::tui_tool::theme::AppTheme;

use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabView {
    Chart,
    State,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    #[serde(flatten)]
    pub response: HashMap<String, DeviceData>,
}

pub struct StreamReference {
    pub device_index: usize,
    pub stream_index: usize,
}

pub struct DataStream {
    pub name: String,
    pub points: Vec<(f64, f64)>,
}

pub struct Device {
    pub name: String,
    pub streams: Vec<DataStream>,
}

pub struct App<T: Transport> {
    pub devices: Vec<Device>,
    pub devices_state: ListState,
    pub streams_state: ListState,
    pub x_axis_stream: Option<StreamReference>,
    pub y_axis_stream: Option<StreamReference>,
    pub transport: T,
    pub connection_status: bool,
    pub current_device_streams: Vec<String>,
    pub show_popup: bool,
    pub session_running: bool,
    pub has_warned_disconnected: bool,
    pub active_tab: TabView,
    pub chart_tab: ChartTab,
    pub state_tab: StateTab,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub should_quit: bool,
    pub in_rerun: bool,
    pub theme: AppTheme,
}

impl<T: Transport> App<T> {
    pub fn new(remote: bool, transport: T, action_tx: mpsc::UnboundedSender<Action>) -> App<T> {
        let mut devices_state = ListState::default();
        devices_state.select(Some(0));
        let devices: Vec<Device> = vec![];
        let current_device_streams = if !devices.is_empty() {
            devices[0].streams.iter().map(|s| s.name.clone()).collect()
        } else {
            vec![]
        };

        // Load theme from rex config file, falling back to Dracula
        let theme = AppTheme::from_config(
            get_configuration()
                .ok()
                .and_then(|cfg| cfg.general.theme)
                .as_deref(),
        );

        App {
            devices,
            devices_state,
            streams_state: ListState::default(),
            x_axis_stream: None,
            y_axis_stream: None,
            transport,
            session_running: false,
            connection_status: true,
            show_popup: false,
            has_warned_disconnected: false,
            current_device_streams,
            active_tab: TabView::Chart,
            chart_tab: ChartTab::new(),
            state_tab: StateTab::new(remote),
            action_tx,
            should_quit: false,
            in_rerun: false,
            theme,
        }
    }

    pub fn clear_chart_state(&mut self) {
        self.x_axis_stream = None;
        self.y_axis_stream = None;
        self.devices.clear();
        self.devices_state.select(if !self.devices.is_empty() {
            Some(0)
        } else {
            None
        });
        self.streams_state.select(None);
        self.current_device_streams.clear();
        log::info!("Cleared chart state due to connection change");
    }

    pub fn handle_transport_error(&mut self, err: &(dyn std::error::Error + Send)) {
        let error_msg = err.to_string();

        let was_connected = self.connection_status;

        // Check if it's just "no session" vs actual connection problem
        if error_msg.contains("No active session") || error_msg.contains("502") {
            // This is expected when no session is running
            self.connection_status = false;

            if was_connected && !self.has_warned_disconnected {
                log::info!("No active session running");
                self.has_warned_disconnected = true;
            }
            return;
        }

        // For other errors, it's a real connection problem
        self.connection_status = false;

        if was_connected {
            log::warn!("Lost connection to server: {}", err);
        }

        self.has_warned_disconnected = true;
    }
    pub fn set_x_axis(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            if let Some(stream_idx) = self.streams_state.selected() {
                self.x_axis_stream = Some(StreamReference {
                    device_index: device_idx,
                    stream_index: stream_idx,
                });
                let device = &self.devices[device_idx];
                let stream = &device.streams[stream_idx];
                log::info!("Set X-axis: {} - {}", device.name, stream.name);
            }
        }
    }

    pub fn set_y_axis(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            if let Some(stream_idx) = self.streams_state.selected() {
                self.y_axis_stream = Some(StreamReference {
                    device_index: device_idx,
                    stream_index: stream_idx,
                });
                let device = &self.devices[device_idx];
                let stream = &device.streams[stream_idx];
                log::info!("Set Y-axis: {} - {}", device.name, stream.name);
            }
        }
    }

    pub fn next_device(&mut self) {
        let i = match self.devices_state.selected() {
            Some(i) => {
                if i >= self.devices.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.devices_state.select(Some(i));
        self.update_current_device_streams();
    }

    pub fn update_current_device_streams(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            if !self.devices.is_empty() && device_idx < self.devices.len() {
                self.current_device_streams = self.devices[device_idx]
                    .streams
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                self.streams_state
                    .select(if !self.current_device_streams.is_empty() {
                        Some(0)
                    } else {
                        None
                    });
            } else {
                self.current_device_streams = vec![];
                self.streams_state.select(None);
            }
        }
    }

    pub fn next_stream(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            let num_streams = self.devices[device_idx].streams.len();
            if num_streams == 0 {
                return;
            }
            let i = match self.streams_state.selected() {
                Some(i) => {
                    if i >= num_streams - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.streams_state.select(Some(i));
        }
    }

    pub fn previous_stream(&mut self) {
        if let Some(device_idx) = self.devices_state.selected() {
            let num_streams = self.devices[device_idx].streams.len();
            if num_streams == 0 {
                return;
            }
            let i = match self.streams_state.selected() {
                Some(i) => {
                    if i == 0 {
                        num_streams - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.streams_state.select(Some(i));
        }
    }

    pub fn previous_device(&mut self) {
        let i = match self.devices_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.devices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.devices_state.select(Some(i));
        self.update_current_device_streams();
    }

    pub fn switch_tab(&mut self) {
        self.active_tab = match self.active_tab {
            TabView::Chart => TabView::State,
            TabView::State => TabView::Chart,
        };
    }
}
