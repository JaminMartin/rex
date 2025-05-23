-- Enable JSON data support
SET enable_json_type = 1;

-- Create primary experiment reference point
CREATE TABLE IF NOT EXISTS experiment_info (
    experiment_id UUID,
    start_time String,
    end_time String,
    name String,
    email String,
    experiment_name String,
    experiment_description String
) ENGINE = MergeTree ORDER BY experiment_id;

-- Create measurement data that links to the experiment ID
CREATE TABLE IF NOT EXISTS measurement_data (
    experiment_id UUID,
    device_name String,
    channel_name String,
    sample_index UInt32,
    channel_index UInt32,
    value Float64
) ENGINE = MergeTree ORDER BY (experiment_id, channel_name, sample_index);

-- Create device meta data for the associated experiment and measurement data
CREATE TABLE IF NOT EXISTS device_info (
    experiment_id UUID,
    device_name String,
    device_config JSON
) ENGINE = MergeTree ORDER BY experiment_id;
