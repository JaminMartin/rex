

SET enable_json_type = 1;

CREATE TABLE IF NOT EXISTS session_info (
    session_id UUID,
    start_time DateTime64(6, 'UTC'),
    end_time DateTime64(6, 'UTC'),
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
