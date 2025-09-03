use clickhouse::Row;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickhouseServer {
    pub server: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub measurement_table: String,
    pub session_meta_table: String,
    pub device_meta_table: String,
    pub results_table: String,
}

#[derive(Debug, Row, Serialize)]
pub struct SessionClickhouse {
    #[serde(with = "clickhouse::serde::uuid")]
    pub session_id: Uuid,
    pub start_time: String,
    pub end_time: String,
    pub name: String,
    pub email: String,
    pub session_name: String,
    pub session_description: String,
    pub session_meta: String,
}

#[derive(Debug, Row, Clone, Serialize)]
pub struct ClickhouseMeasurementPrimative {
    #[serde(with = "clickhouse::serde::uuid")]
    pub session_id: Uuid,
    pub device_name: String,
    pub channel_name: String,
    pub channel_unit: String,
    pub sample_index: u32,
    pub channel_index: u32,
    pub value: f64,
    #[serde(with = "clickhouse::serde::time::datetime64::micros")]
    pub timestamp: OffsetDateTime,
}
pub struct ClickhouseMeasurements {
    pub measurements: Vec<ClickhouseMeasurementPrimative>,
}

#[derive(Debug, Row, Clone, Serialize, Deserialize)]
pub struct ClickhouseDevicePrimative {
    #[serde(with = "clickhouse::serde::uuid")]
    pub session_id: Uuid,
    pub device_name: String,
    pub device_config: String,
}
#[derive(Debug, Row, Clone, Serialize, Deserialize)]
pub struct ClickhouseDevices {
    pub devices: Vec<ClickhouseDevicePrimative>,
}

#[derive(Debug, Row, Clone, Serialize, Deserialize)]
pub struct ClickhouseResultsPrimative {
    #[serde(with = "clickhouse::serde::uuid")]
    pub session_id: Uuid,
    pub result_type: String,
    pub result_info: String,
    pub result_status: bool,
    pub limit_value: f64,
    pub measured_value: f64,
    pub result_meta: String,
}
#[derive(Debug, Row, Clone, Serialize, Deserialize)]
pub struct ClickhouseResults {
    pub results: Vec<ClickhouseResultsPrimative>,
}
