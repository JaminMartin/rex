
SET enable_json_type = 1;

CREATE TABLE IF NOT EXISTS session_info (
    session_id UUID,
    start_time String,
    end_time String,
    name String,
    email String,
    session_name String,
    session_description String,
    session_meta JSON
) ENGINE = MergeTree ORDER BY session_id;


CREATE TABLE IF NOT EXISTS measurement_data (
    session_id UUID,
    device_name String,
    channel_name String,
    channel_unit String,
    sample_index UInt32,
    channel_index UInt32,
    value Float64,
    timestamp DateTime64(6, 'UTC')
) ENGINE = MergeTree ORDER BY (session_id, channel_name, sample_index);


CREATE TABLE IF NOT EXISTS device_info (
    session_id UUID,
    device_name String,
    device_config JSON
) ENGINE = MergeTree ORDER BY session_id;



CREATE TABLE IF NOT EXISTS results_store (
    session_id UUID,
    result_type String,
    result_info String,
    result_status Bool,
    measured_value Float64,
    limit_value Float64,
    result_meta JSON,
) ENGINE = MergeTree ORDER BY (session_id, result_type);
