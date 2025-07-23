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
    pub experiment_meta_table: String,
    pub device_meta_table: String,
}

#[derive(Debug, Row, Serialize)]
pub struct ExperimentClickhouse {
    #[serde(with = "clickhouse::serde::uuid")]
    pub experiment_id: Uuid,
    pub start_time: String,
    pub end_time: String,
    pub name: String,
    pub email: String,
    pub experiment_name: String,
    pub experiment_description: String,
    pub experiment_meta: String,
}

#[derive(Debug, Row, Clone, Serialize)]
pub struct ClickhouseMeasurementPrimative {
    #[serde(with = "clickhouse::serde::uuid")]
    pub experiment_id: Uuid,
    pub device_name: String,
    pub channel_name: String,
    pub sample_index: u32,
    pub channel_index: u32,
    pub value: f64,
    #[serde(with = "clickhouse::serde::time::datetime64::micros")]
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Row, Clone, Serialize, Deserialize)]
pub struct ClickhouseDevicePrimative {
    #[serde(with = "clickhouse::serde::uuid")]
    pub experiment_id: Uuid,
    pub device_name: String,
    pub device_config: String,
}
#[derive(Debug, Row, Clone, Serialize, Deserialize)]
pub struct ClickhouseDevices {
    pub devices: Vec<ClickhouseDevicePrimative>,
}
pub struct ClickhouseMeasurements {
    pub measurements: Vec<ClickhouseMeasurementPrimative>,
}
