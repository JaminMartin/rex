use crate::data_handler::transport::Transport;
use crate::data_handler::DeviceData;
use crate::tui_tool::tabs::{chart::ChartTab, state::StateTab};
use itertools::Itertools;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

impl<T: Transport> App<T> {
    pub fn new(remote: bool, transport: T) -> App<T> {
        let mut devices_state = ListState::default();
        devices_state.select(Some(0));
        let devices: Vec<Device> = vec![];
        let current_device_streams = if !devices.is_empty() {
            devices[0].streams.iter().map(|s| s.name.clone()).collect()
        } else {
            vec![]
        };

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
    fn handle_transport_error(&mut self, err: &dyn std::error::Error) {
        let error_msg = err.to_string();

        let was_connected = self.connection_status;
        self.connection_status = false;

        if was_connected {
            if error_msg.contains("502") || error_msg.contains("Bad Gateway") {
                log::warn!("No active session found. Connection to HTTP server is OK, but no session is running.");
            } else {
                log::warn!("Disconnected from server: {}", err);
            }
        }

        self.has_warned_disconnected = true;
    }
    pub fn fetch_server_data(&mut self) {
        match self.transport.send_command("GET_DATASTREAM\n") {
            Ok(response) => {
                if !self.connection_status {
                    log::info!("Reconnected to server");
                    self.clear_chart_state();
                    self.connection_status = true;
                    self.has_warned_disconnected = false;
                }

                // Check if we actually got data (session is running)
                if !response.is_empty() {
                    if !self.session_running {
                        log::info!("Session detected as running");
                        self.session_running = true;
                    }

                    match serde_json::from_str::<ServerResponse>(&response) {
                        Ok(server_response) => {
                            self.devices = server_response
                                .response
                                .into_iter()
                                .sorted_by_key(|(k, _)| k.clone())
                                .map(|(device_key, device_data)| {
                                    let streams = device_data
                                        .measurements
                                        .into_iter()
                                        .sorted_by_key(|(k, _)| k.clone())
                                        .map(|(name, values)| DataStream {
                                            name,
                                            points: values
                                                .into_iter()
                                                .enumerate()
                                                .map(|(i, v)| (i as f64, v))
                                                .collect(),
                                        })
                                        .collect();
                                    Device {
                                        name: device_key,
                                        streams,
                                    }
                                })
                                .collect();
                        }
                        Err(e) => {
                            log::warn!("Failed to parse server response: {}", e);
                        }
                    }
                } else {
                    // Empty response means no session running
                    if self.session_running {
                        log::info!("Session ended");
                        self.session_running = false;
                    }
                }
            }
            Err(e) => {
                self.handle_transport_error(&*e);
                self.session_running = false; // If we can't connect, no session is running
            }
        }
    }

    pub fn fetch_state_data(&mut self) {
        match self.transport.send_command("STATE\n") {
            Ok(response) => {
                if !self.connection_status {
                    log::info!("Reconnected to server");
                    self.connection_status = true;
                    self.has_warned_disconnected = false;
                }

                if !response.is_empty() {
                    if !self.session_running {
                        log::info!("Session detected as running");
                        self.session_running = true;
                    }
                    let _ = self.state_tab.update_from_json(&response);
                } else {
                    // Empty response means no session running
                    if self.session_running {
                        log::info!("Session ended");
                        self.session_running = false;
                    }
                }
            }
            Err(e) => {
                self.handle_transport_error(&*e);
                self.session_running = false;
            }
        }
    }

    pub fn kill_server(&mut self) {
        let response = self.transport.send_command("KILL\n");
        log::info!("Kill command response: {:?}", response);
    }

    pub fn pause_server(&mut self) {
        let response = self.transport.send_command("PAUSE_STATE\n");
        log::info!("Pause command response: {:?}", response);
    }

    pub fn resume_server(&mut self) {
        let response = self.transport.send_command("RESUME_STATE\n");
        log::info!("Resume command response: {:?}", response);
    }

    pub fn disconnect(&mut self) {
        if let Some(d) = self.transport.disconnect() {
            log::info!("Disconnected: {:?}", d);
        }
        self.connection_status = false;
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
    pub fn on_tick(&mut self) {
        match self.active_tab {
            TabView::Chart => {
                let _ = self.fetch_server_data();
            }
            TabView::State => {
                if self.connection_status {
                    let _ = self.fetch_state_data();
                }
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

impl<T: Transport> Drop for App<T> {
    fn drop(&mut self) {
        self.disconnect();
    }
}
