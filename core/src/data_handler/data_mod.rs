use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self};
use std::path::PathBuf;
use time::error::Parse;
use time::macros::format_description;
use time::OffsetDateTime;
use toml::{Table, Value};

use uuid::Uuid;

use crate::db::{
    ClickhouseDevicePrimative, ClickhouseDevices, ClickhouseMeasurementPrimative,
    ClickhouseMeasurements, ClickhouseResults, ClickhouseResultsPrimative, ClickhouseServer,
    SessionClickhouse,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub email_server: Option<EmailServer>,
    pub click_house_server: Option<ClickhouseServer>,
    pub general: GeneralConfig,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub port: String,
    pub interpreter: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailServer {
    pub server: String,
    pub security: bool,
    pub username: Option<String>,
    pub password: Option<String>,
    pub port: Option<String>,
    pub from_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Entity {
    Device(Device),
    Session(DataSession),
    Results(SessionResults),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Listner {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataSession {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub uuid: Option<Uuid>,
    pub info: SessionInfo,
}

impl DataSession {
    pub fn new(info: SessionInfo, uuid: Uuid) -> Self {
        DataSession {
            start_time: Some(create_time_stamp(false)),
            end_time: None,
            uuid: Some(uuid),
            info,
        }
    }
    fn append_end_time(&mut self) {
        self.end_time = Some(create_time_stamp(false));
    }

    pub fn to_clickhouse(&self, id: Uuid) -> Option<SessionClickhouse> {
        let start_time = self.start_time.as_ref()?;
        let end_time = self.end_time.as_ref()?;

        let exp = SessionClickhouse {
            session_id: id,
            start_time: start_time.clone(),
            end_time: end_time.clone(),
            name: self.info.name.clone(),
            email: self.info.email.clone(),
            session_name: self.info.session_name.clone(),
            session_description: self.info.session_description.clone(),
            session_meta: serde_json::to_string(&self.info.meta)
                .expect("Cannot unwrap config into valid json"),
        };
        Some(exp)
    }
}
impl Default for DataSession {
    fn default() -> Self {
        DataSession {
            start_time: Some(create_time_stamp(false)),
            end_time: None,
            uuid: None,
            info: SessionInfo::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionResults {
    pub result_name: String,
    pub result_description: String,
    pub result_status: bool,
    pub result_criteria: Option<f64>,
    pub result_value: Option<f64>,
    #[serde(default)]
    pub result_meta: HashMap<String, Value>,
}
impl SessionResults {
    pub fn default() -> Self {
        SessionResults {
            result_name: String::default(),
            result_description: String::default(),
            result_status: bool::default(),
            result_criteria: Some(f64::default()),
            result_value: Some(f64::default()),
            result_meta: HashMap::default(),
        }
    }

    pub fn to_clickhouse(&self, id: Uuid) -> Option<ClickhouseResultsPrimative> {
        let conf = ClickhouseResultsPrimative {
            session_id: id,
            result_type: self.result_name.clone(),
            result_info: self.result_description.clone(),
            result_status: self.result_status,
            limit_value: self.result_criteria.unwrap_or_default(),
            measured_value: self.result_value.unwrap_or_default(),
            result_meta: serde_json::to_string(&self.result_meta)
                .expect("failed to serialize result_meta"),
        };

        Some(conf)
    }
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SessionInfo {
    pub name: String,
    pub email: String,
    #[serde(alias = "experiment_name", alias = "test_name", alias = "run_name")]
    pub session_name: String,
    #[serde(
        alias = "experiment_description",
        alias = "test_description",
        alias = "run_description"
    )]
    pub session_description: String,
    pub meta: Option<SessionMetadata>,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SessionMetadata {
    #[serde(flatten)]
    pub meta: HashMap<String, Value>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MeasurementData {
    Single(Vec<f64>),
    Multi(Vec<Vec<f64>>),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Measurement {
    pub data: MeasurementData,
    pub unit: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub device_name: String,
    pub device_config: HashMap<String, Value>,
    pub measurements: HashMap<String, Measurement>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub timestamps: HashMap<String, Vec<String>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceData {
    pub device_name: String,
    pub measurements: HashMap<String, Vec<f64>>,
}
impl Device {
    fn update(&mut self, other: Self) {
        let timestamp = vec![create_log_timestamp()];
        for (measure_type, measurement) in other.measurements {
            match self.measurements.entry(measure_type.clone()) {
                Entry::Occupied(mut entry) => {
                    let existing_measurement = entry.get_mut();

                    if existing_measurement.unit != measurement.unit {
                        log::error!(
                            "Unit mismatch for '{}': existing '{}' vs new '{}'",
                            measure_type,
                            existing_measurement.unit,
                            measurement.unit
                        );
                        continue;
                    }

                    match (&mut existing_measurement.data, &measurement.data) {
                        (
                            MeasurementData::Single(existing),
                            MeasurementData::Single(new_values),
                        ) => {
                            existing.extend(new_values.clone());
                            if let Some(ts_vec) = self.timestamps.get_mut(&measure_type) {
                                if let Some(other_timestamps) = other.timestamps.get(&measure_type)
                                {
                                    ts_vec.extend(other_timestamps.clone());
                                }
                            }
                        }
                        (MeasurementData::Multi(existing), MeasurementData::Multi(new_values)) => {
                            existing.extend(new_values.clone());
                            if let Some(ts_vec) = self.timestamps.get_mut(&measure_type) {
                                if let Some(other_timestamps) = other.timestamps.get(&measure_type)
                                {
                                    ts_vec.extend(other_timestamps.clone());
                                }
                            }
                        }
                        _ => {
                            log::error!("Measurement type mismatch during update for '{measure_type}' - cannot change between Single and Multi variants");
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(measurement);
                    self.timestamps.insert(measure_type, timestamp.clone());
                }
            }
        }
    }

    fn latest_measurements_truncated(&self, max_measurements: usize) -> HashMap<String, Vec<f64>> {
        let truncated_measurements = self
            .measurements
            .iter()
            .map(|(key, measurement)| {
                let truncated = match &measurement.data {
                    MeasurementData::Single(single_values) => single_values
                        .iter()
                        .rev()
                        .take(max_measurements)
                        .cloned()
                        .collect::<Vec<f64>>()
                        .into_iter()
                        .rev()
                        .collect(),
                    MeasurementData::Multi(multi_values) => {
                        if let Some(latest_array) = multi_values.last() {
                            match latest_array.len() {
                                0..=100 => latest_array.clone(),
                                _ => {
                                    let chunk_size = div_ceil(latest_array.len(), 100);
                                    latest_array
                                        .chunks(chunk_size)
                                        .map(|chunk| chunk.iter().sum::<f64>() / chunk.len() as f64)
                                        .collect()
                                }
                            }
                        } else {
                            Vec::new()
                        }
                    }
                };
                (key.clone(), truncated)
            })
            .collect();
        truncated_measurements
    }
    fn latest_timestamps_truncated(&self, max_measurements: usize) -> HashMap<String, Vec<f64>> {
        self.timestamps
            .iter()
            .filter_map(|(key, values)| {
                let base_time = values.first().and_then(|s| {
                    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
                })?;

                let truncated: Vec<String> = values
                    .iter()
                    .rev()
                    .take(max_measurements)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();

                let rel_secs = truncated
                    .into_iter()
                    .filter_map(|ts| {
                        OffsetDateTime::parse(&ts, &time::format_description::well_known::Rfc3339)
                            .ok()
                            .map(|t| (t - base_time).as_seconds_f64())
                    })
                    .collect::<Vec<f64>>();
                let time_key = format!("Time since first {} measurement (s)", key.clone());
                Some((time_key, rel_secs))
            })
            .collect()
    }
    fn latest_data_truncated(&self, max_measurements: usize) -> DeviceData {
        let mut combined = self.latest_measurements_truncated(max_measurements);
        let timestamps_truncated = self.latest_timestamps_truncated(max_measurements);
        combined.extend(timestamps_truncated);
        DeviceData {
            device_name: self.device_name.clone(),
            measurements: combined,
        }
    }
    pub fn truncate(&mut self) {
        self.measurements
            .iter_mut()
            .for_each(|(_, measurement)| match &mut measurement.data {
                MeasurementData::Single(single_values) => {
                    let len = single_values.len();
                    if len > 100 {
                        single_values.drain(0..len - 100);
                    }
                }
                MeasurementData::Multi(multi_values) => {
                    let len_before = multi_values.len();
                    if len_before > 1 {
                        let last = multi_values.pop();
                        multi_values.clear();
                        if let Some(last) = last {
                            multi_values.push(last);
                        }
                    }
                }
            });
        self.timestamps.iter_mut().for_each(|(_, values)| {
            let len = values.len();
            if len > 100 {
                values.drain(0..len - 100);
            }
        });
    }
    pub fn to_clickhouse_measurements(&self, id: Uuid) -> Option<ClickhouseMeasurements> {
        let parsed_timestamps: HashMap<String, Vec<OffsetDateTime>> = self
            .timestamps
            .iter()
            .map(|(channel_name, ts_vec)| {
                let parsed = ts_vec
                    .iter()
                    .map(|ts| {
                        OffsetDateTime::parse(ts, &time::format_description::well_known::Rfc3339)
                            .expect("invalid timestamp")
                    })
                    .collect();
                (channel_name.clone(), parsed)
            })
            .collect();
        let measurements = self
            .measurements
            .iter()
            .flat_map(|(channel_name, measurement)| match &measurement.data {
                MeasurementData::Single(single_values) => single_values
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| ClickhouseMeasurementPrimative {
                        session_id: id,
                        device_name: self.device_name.clone(),
                        channel_name: channel_name.clone(),
                        channel_unit: measurement.unit.clone(),
                        sample_index: i as u32,
                        channel_index: 0,
                        value: v,
                        timestamp: parsed_timestamps
                            .get(channel_name)
                            .and_then(|ts| ts.get(i))
                            .copied()
                            .unwrap_or_else(OffsetDateTime::now_utc),
                    })
                    .collect::<Vec<_>>(),
                MeasurementData::Multi(multi_values) => multi_values
                    .iter()
                    .enumerate()
                    .flat_map(|(i, v)| {
                        v.iter().enumerate().map({
                            let ts_value = parsed_timestamps.clone();
                            let unit = measurement.unit.clone();
                            move |(j, &vv)| ClickhouseMeasurementPrimative {
                                session_id: id,
                                device_name: self.device_name.clone(),
                                channel_name: channel_name.clone(),
                                channel_unit: unit.clone(),
                                sample_index: j as u32,
                                channel_index: i as u32,
                                value: vv,
                                timestamp: ts_value
                                    .get(channel_name)
                                    .and_then(|ts| ts.get(i))
                                    .copied()
                                    .unwrap_or_else(OffsetDateTime::now_utc),
                            }
                        })
                    })
                    .collect::<Vec<_>>(),
            })
            .collect::<Vec<_>>();
        Some(ClickhouseMeasurements { measurements })
    }

    pub fn to_clickhouse_config(&self, id: Uuid) -> Option<ClickhouseDevicePrimative> {
        let conf = ClickhouseDevicePrimative {
            session_id: id,
            device_name: self.device_name.to_string(),
            device_config: serde_json::to_string(&self.device_config)
                .expect("Cannot unwrap config into valid json"),
        };

        Some(conf)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerState {
    pub entities: HashMap<String, Entity>,
    pub internal_state: bool,
    pub retention: bool,
    pub uuid: Uuid,
    pub external_metadata: Option<HashMap<String, Value>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub entities: DataSession,
}

impl ServerState {
    pub fn new(uuid: Uuid, external_metadata: String) -> Self {
        let external_metadata = parse_external_metadata(external_metadata);
        ServerState {
            entities: HashMap::new(),
            internal_state: true,
            retention: true,
            uuid,
            external_metadata,
        }
    }
    pub fn to_summary(&self) -> Option<Summary> {
        let entity = self
            .entities
            .iter()
            .filter_map(|(_, entity)| match entity {
                Entity::Session(data_session) => Some(data_session),
                Entity::Device(_) => None,
                Entity::Results(_) => None,
            })
            .next()?;

        Some(Summary {
            entities: entity.clone(),
        })
    }
    pub fn update_entity(&mut self, key: String, incoming: Entity) {
        match incoming {
            Entity::Device(incoming_device) => match self.entities.entry(key) {
                Entry::Occupied(mut entry) => {
                    if let Entity::Device(existing_device) = entry.get_mut() {
                        existing_device.update(incoming_device);
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(Entity::Device(incoming_device));
                }
            },
            Entity::Results(incoming_results) => {
                self.entities.insert(key, Entity::Results(incoming_results));
            }
            Entity::Session(session_setup) => match self.entities.entry(key) {
                Entry::Vacant(entry) => {
                    let mut data_session = DataSession::new(session_setup.info, self.uuid);

                    if let Some(external_meta) = &self.external_metadata {
                        if let Some(ref mut existing_meta) = data_session.info.meta {
                            for (key, value) in external_meta {
                                existing_meta.meta.insert(key.clone(), value.clone());
                            }
                        } else {
                            let meta_struct = SessionMetadata {
                                meta: external_meta.clone(),
                            };
                            data_session.info.meta = Some(meta_struct);
                        }
                    }

                    entry.insert(Entity::Session(data_session));
                }
                Entry::Occupied(_) => {
                    log::warn!("Can't create multiple sessions: ignoring");
                }
            },
        }

        if !self.retention {
            self.truncate_data();
        };
    }
    pub fn truncate_data(&mut self) {
        for value in self.entities.values_mut() {
            match value {
                Entity::Device(device_data) => {
                    device_data.truncate();
                }
                Entity::Session(_) => {}
                Entity::Results(_) => {}
            }
        }
    }
    pub fn finalise_time(&mut self) {
        for entity in self.entities.values_mut() {
            if let Entity::Session(data_session) = entity {
                data_session.append_end_time()
            }
        }
    }

    pub fn print_state(&self) {
        // To be deprecated or put behind a feature flag / or similified for tokio instrumentation + Otel
        log::info!("=== Current Server State ===");
        if self.entities.is_empty() {
            log::info!("No devices connected.");
            return;
        }

        for entity in self.entities.values() {
            match entity {
                Entity::Device(device) => {
                    let total_measurements: usize = device
                        .measurements
                        .values()
                        .map(|v| match &v.data {
                            MeasurementData::Single(data) => data.len(),
                            MeasurementData::Multi(data) => data.len(),
                        })
                        .sum();

                    let last_measurement = device
                        .measurements
                        .values()
                        .flat_map(|v| match &v.data {
                            MeasurementData::Single(data) => vec![data.last().cloned()],
                            MeasurementData::Multi(data) => {
                                vec![data.last().and_then(|inner| inner.last().cloned())]
                            }
                        })
                        .flatten()
                        .last();

                    log::info!(
                        "Device: {} - Total measurements: {}, Last measurement: {:?}",
                        device.device_name,
                        total_measurements,
                        last_measurement
                    );
                }
                Entity::Session(_session) => {}
                Entity::Results(_session) => {}
            }
        }
        log::info!("========================\n");
    }

    pub fn dump_to_toml(&self, file_path: &String) -> io::Result<()> {
        let mut root = Table::new();

        for (key, entity) in &self.entities {
            match entity {
                Entity::Results(_result_info) => {}
                Entity::Session(session_setup) => {
                    if !root.contains_key("session") {
                        root.insert("session".to_string(), Value::Table(Table::new()));
                    }

                    let session_table = root
                        .get_mut("session")
                        .and_then(|v| v.as_table_mut())
                        .unwrap();
                    session_table.insert(
                        "start_time".to_string(),
                        Value::String(session_setup.start_time.clone().unwrap_or_default()),
                    );
                    session_table.insert(
                        "end_time".to_string(),
                        Value::String(session_setup.end_time.clone().unwrap_or_default()),
                    );

                    session_table.insert(
                        "UUID".to_string(),
                        Value::String(
                            session_setup
                                .uuid
                                .map_or(String::new(), |uuid| uuid.to_string()),
                        ),
                    );
                    let mut session_config = Table::new();

                    session_config.insert(
                        "name".to_string(),
                        Value::String(session_setup.info.name.clone()),
                    );
                    session_config.insert(
                        "email".to_string(),
                        Value::String(session_setup.info.email.clone()),
                    );

                    session_config.insert(
                        "session_name".to_string(),
                        Value::String(session_setup.info.session_name.clone()),
                    );
                    session_config.insert(
                        "session_description".to_string(),
                        Value::String(session_setup.info.session_description.clone()),
                    );
                    if let Some(ref meta) = session_setup.info.meta {
                        let mut meta_table = Table::new();
                        for (key, value) in &meta.meta {
                            let toml_value = match value {
                                Value::String(s) => Value::String(s.clone()),
                                Value::Float(n) => Value::Float(*n),
                                Value::Integer(i) => Value::Integer(*i),
                                Value::Boolean(b) => Value::Boolean(*b),
                                Value::Array(arr) => {
                                    let array_values: Vec<Value> =
                                        arr.iter().map(|v| Value::String(v.to_string())).collect();
                                    Value::Array(array_values)
                                }
                                _ => Value::String(value.to_string()),
                            };
                            meta_table.insert(key.clone(), toml_value);
                        }
                        session_config.insert("meta".to_string(), Value::Table(meta_table));
                    }
                    session_table.insert("info".to_string(), Value::Table(session_config));
                }

                Entity::Device(device) => {
                    if !root.contains_key("device") {
                        root.insert("device".to_string(), Value::Table(Table::new()));
                    }

                    let device_table = root
                        .get_mut("device")
                        .and_then(|v| v.as_table_mut())
                        .unwrap();

                    let mut device_config = Table::new();
                    device_config.insert(
                        "device_name".to_string(),
                        Value::String(device.device_name.clone()),
                    );

                    for (config_key, config_value) in &device.device_config {
                        device_config.insert(config_key.clone(), config_value.clone());
                    }

                    let mut data_table = Table::new();
                    for (measurement_type, measurement) in &device.measurements {
                        let mut measurement_obj = Table::new();
                        measurement_obj
                            .insert("unit".to_string(), Value::String(measurement.unit.clone()));

                        match &measurement.data {
                            MeasurementData::Single(single_values) => {
                                measurement_obj.insert(
                                    "data".to_string(),
                                    Value::Array(
                                        single_values.iter().map(|&v| Value::Float(v)).collect(),
                                    ),
                                );
                            }
                            MeasurementData::Multi(multi_values) => {
                                let nested_arrays: Vec<Value> = multi_values
                                    .iter()
                                    .map(|inner_vec| {
                                        Value::Array(
                                            inner_vec.iter().map(|&v| Value::Float(v)).collect(),
                                        )
                                    })
                                    .collect();
                                measurement_obj
                                    .insert("data".to_string(), Value::Array(nested_arrays));
                            }
                        }

                        data_table.insert(measurement_type.clone(), Value::Table(measurement_obj));
                    }
                    let mut time_stamps = Table::new();
                    for (measurement_type, values) in &device.timestamps {
                        time_stamps.insert(
                            measurement_type.clone(),
                            Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()),
                        );
                    }
                    device_config.insert("data".to_string(), Value::Table(data_table));
                    device_config.insert("timestamps".to_string(), Value::Table(time_stamps));
                    device_table.insert(key.clone(), Value::Table(device_config));
                }
            }
        }
        let toml_string =
            toml::to_string_pretty(&root).map_err(|e| io::Error::other(e.to_string()))?;
        fs::write(file_path, toml_string.clone())?;
        let tmp_dir = env::temp_dir();
        let temp_path = tmp_dir.join("rex.toml");
        fs::write(&temp_path, toml_string)?;
        Ok(())
    }
    pub fn get_session_name(&self) -> Option<String> {
        self.entities.values().find_map(|entity| {
            if let Entity::Session(data_session) = entity {
                Some(data_session.info.session_name.clone())
            } else {
                None
            }
        })
    }

    pub fn session_data_ch(&self, id: Uuid) -> Option<SessionClickhouse> {
        self.entities.values().find_map(|entity| {
            if let Entity::Session(data_session) = entity {
                data_session.to_clickhouse(id)
            } else {
                None
            }
        })
    }
    pub fn device_data_ch(&self, id: Uuid) -> Option<Vec<ClickhouseMeasurements>> {
        let device_data: Vec<ClickhouseMeasurements> = self
            .entities
            .values()
            .filter_map(|entity| {
                if let Entity::Device(device) = entity {
                    device.to_clickhouse_measurements(id)
                } else {
                    None
                }
            })
            .collect();
        if device_data.is_empty() {
            None
        } else {
            Some(device_data)
        }
    }
    pub fn device_config_ch(&self, id: Uuid) -> Option<ClickhouseDevices> {
        let device_data: ClickhouseDevices = ClickhouseDevices {
            devices: self
                .entities
                .values()
                .filter_map(|entity| {
                    if let Entity::Device(device) = entity {
                        device.to_clickhouse_config(id)
                    } else {
                        None
                    }
                })
                .collect(),
        };
        if device_data.devices.is_empty() {
            None
        } else {
            Some(device_data)
        }
    }
    pub fn results_ch(&self, id: Uuid) -> Option<ClickhouseResults> {
        let result_data: ClickhouseResults = ClickhouseResults {
            results: self
                .entities
                .values()
                .filter_map(|entity| {
                    if let Entity::Results(result) = entity {
                        result.to_clickhouse(id)
                    } else {
                        None
                    }
                })
                .collect(),
        };
        if result_data.results.is_empty() {
            None
        } else {
            Some(result_data)
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        log::trace!("Validating state, entities: {:?}", self.entities);

        let has_session_setup = self
            .entities
            .values()
            .any(|entity| matches!(entity, Entity::Session(_)));
        if !has_session_setup {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No entity of type Session found",
            ));
        }
        Ok(())
    }

    pub fn send_stream(&self) -> HashMap<String, DeviceData> {
        let mut stream_contents = HashMap::new();
        for entity in self.entities.values() {
            match entity {
                Entity::Device(device) => {
                    stream_contents.insert(
                        device.device_name.clone(),
                        device.latest_data_truncated(100),
                    );
                }
                Entity::Session(_session) => {}
                Entity::Results(_session) => {}
            }
        }
        stream_contents
    }
}

impl Default for ServerState {
    fn default() -> Self {
        let uuid = Uuid::new_v4();
        let external_metadata = "".to_string();
        Self::new(uuid, external_metadata)
    }
}
pub fn sanitize_filename(name: String) -> String {
    name.replace([' ', '/'], "_")
}

pub fn parse_custom_timestamp(
    timestamp: &str,
    is_header_format: bool,
) -> Result<OffsetDateTime, Parse> {
    // Choose the format based on whether it uses dashes or underscores
    let format = if is_header_format {
        format_description!(
            "[day]_[month]_[year]_[hour repr:24]_[minute]_[second]_[subsecond digits:3]"
        )
    } else {
        format_description!(
            "[day]-[month]-[year] [hour repr:24]:[minute]:[second].[subsecond digits:3]"
        )
    };
    OffsetDateTime::parse(timestamp, &format)
}

pub fn custom_to_standard(timestamp: &str, is_header_format: bool) -> Result<String, Parse> {
    let dt = parse_custom_timestamp(timestamp, is_header_format)?;
    Ok(dt
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap())
}
pub fn create_time_stamp(header: bool) -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let format_file = match header {
        false => format_description!(
            "[day]-[month]-[year] [hour repr:24]:[minute]:[second].[subsecond digits:3]"
        ),
        true => format_description!(
            "[day]_[month]_[year]_[hour repr:24]_[minute]_[second]_[subsecond digits:3]"
        ),
    };

    now.format(&format_file).unwrap()
}
pub fn create_log_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

fn div_ceil(a: usize, b: usize) -> usize {
    a.div_ceil(b)
}

pub fn get_configuration() -> Result<Configuration, String> {
    let config_path = configurable_dir_path("XDG_CONFIG_HOME", dirs::config_dir)
        .map(|mut path| {
            path.push("rex");
            path.push("config.toml");
            path
        })
        .ok_or("Failed to get config directory, setup your config directory then run rex");
    let conf = match config_path {
        Ok(path) => path,
        Err(res) => {
            log::error!("{res}");
            return Err(res.to_string());
        }
    };
    let config_contents = fs::read_to_string(conf);

    let contents = match config_contents {
        Ok(contents) => toml::from_str(&contents),
        Err(e) => {
            log::error!("Could not read config.toml file, raised the following error: {e}");
            return Err(e.to_string());
        }
    };
    let rex_configuration: Configuration = match contents {
        Ok(config) => config,
        Err(e) => {
            log::error!("Could not read config.toml file, raised the following error: {e}");

            return Err(e.to_string());
        }
    };

    Ok(rex_configuration)
}

// allow for XDG_CONFIG_HOME env to allow MacOS users to have more granular control of config paths
pub fn configurable_dir_path(
    env_var: &str,
    dir: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    std::env::var(env_var)
        .ok()
        .and_then(|path| PathBuf::try_from(path).ok())
        .or_else(dir)
}
fn parse_external_metadata(meta_data: String) -> Option<HashMap<String, Value>> {
    match serde_json::from_str(&meta_data) {
        Ok(meta) => meta,
        Err(_) => None,
    }
}
