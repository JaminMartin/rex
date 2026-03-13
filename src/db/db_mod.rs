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
}

#[derive(Debug, Row, Serialize)]
pub struct SessionClickhouse {
    #[serde(with = "clickhouse::serde::uuid")]
    pub session_id: Uuid,
    #[serde(with = "clickhouse::serde::time::datetime64::micros")]
    pub start_time: OffsetDateTime,
    #[serde(with = "clickhouse::serde::time::datetime64::micros")]
    pub end_time: OffsetDateTime,
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
