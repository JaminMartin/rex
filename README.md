# Rex, the rust based experimental data manager

![Logo](https://raw.githubusercontent.com/JaminMartin/rex/refs/heads/master/assets/rex.svg)
Build deterministic experiment pipelines in the scripting language of your choice!
# Features
- Rich logging of data collection, both in a log format as well as an interactive interface
- Robust multi-threaded approach to data logging
- Fails safe to prevent data loss
- Human readable TOML data files that can be used to reproduce identical experiments
- Language agnostic — can in principle run and manage data from any scripting language that can send data in an appropriate form over TCP
    - First class support for Python via [rex_utils](https://github.com/JaminMartin/rex_utils)
    - Beta support for Rustscript
    - Alpha support for Matlab
- Supports sending results over email
- Remote start and monitor with HTTP API endpoints
- Interactive TUI with live plotting and session management
- Optional ClickHouse database backend for persistent storage

# See it in action:

![video](https://raw.githubusercontent.com/JaminMartin/rex/refs/heads/devel/assets/demo.gif)

# Architecture
```
    +-------------------------+                                                                                                       
    |                         |                                                                                                       
    |                         |                       +----------------------------------------------------+                          
    |Interactive TUI/Graphing |                       |         Scripting Language Environment              |                          
    |                         |                       |           (Python, Rust, Matlab...)                 |                          
    |                         |                       |                 +----------------+                  |                          
    |                         |                       |             +-- |Client Library  |-----+            |                          
    |                         |                       |             |   +----------------+     +-----------+-----------+               
    +---+---------------------+                       |             v                          |           |           |               
     ^  |                                             |  +-------------------------+           |           |           |               
     |  |                                             |  |        Rex (Rust)       |           |           |           |               
     |  | +-------------------------------------------+->| CLI interface           |           v           |           |               
     |  | |                                           |  |                         | Experiment Initialiser|           |               
     |  | |                     +--User Interaction---+--+ Interpreter manager     |           |           |           |               
     |  | |                     |                     |  |                         |           |           |           |               
     |  | |                     |                     |  | Thread pool management  |           |           |           |               
     |  | |                     |                     |  |                         |           v           |           |               
     |  | |                     v                     |  | TCP server              |    Device Drivers     |           |               
     |  v |               +------------+              |  |                         |           |           |           |               
+----+----+-------+       |            |<------+      |  | Mailer                  |           |           |           |               
|                 |<------| TCP Server |       |      |  |                         |           |           |           |               
|Triaged logging  |       |            +---+   |      |  | Loops/Delays            |           v           |  Library imports          
|                 |------>|            |   |   |      |  +------------------------++ VISA/USB/Serial/etc.  |           |               
+--------+--------+       +------------+   |   |      |                           |                        |           |               
    ^    |   ^                             |   |      +---------------------------+------------------------+           |               
    |    |   |                             |   |                                  |                                    |               
    |    |   |                             |   |                                  |                                    |               
    |    |   |                             |   |                                  |                                    |               
    |    v   |                             |   |                                  v                                    |               
    |  +-----+------------+                |   |                                +--------------------------+           |               
    |  | Data Validation  |                |   +--------------------------------+   Experiment script      |           |               
    |  |                  |                |    Real Time data exchange          |                          |           |               
    |  +-----------+------+                +----------------------------------->|  - Control flow          |           |               
    |              |                  +----------------------------+            |                          |           |               
    |              |                  |                            |            |  - Device initialisation |<----------+               
    |              v                  |     User Config File       +----------->|                          |                           
    |  +------------------+           | - Device configuration     |            |  - Relays experiment info|                           
    |  |      Storage     |           |                            |            |                          |                           
    +--+                  |           | - Experiment information   |            |                          |                           
       +------------------+           |                            |            |                          |                           
                                      +----------------------------+            +--------------------------+                          
```

# Install
Clone the repository and run:
```shell
cargo install --path .
```
Alternatively, if you are using `python` and `uv`, rex is packaged on `PyPi` as `rex-pycli`. You can simply install the CLI by running:

```
uv tool install rex-pycli
```

# Quick Start

1. **Create the config file** at `~/.config/rex/config.toml` (Linux), `~/Library/Application Support/rex/config.toml` (macOS), or `AppData/Roaming/rex/config.toml` (Windows):
```toml
[general]
port = "7676"
interpreter = "/usr/bin/python3"
theme = "dracula" # Optional. See "TUI Theming" for all available themes.
```

2. **Write an experiment script** using [rex_utils](https://github.com/JaminMartin/rex_utils) — it provides ready-made `Session`, `Device`, `Listener`, and `Result` classes for communicating with the rex TCP server. See the [examples](https://github.com/JaminMartin/rex_utils/tree/main/examples) directory for complete working scripts.

3. **Run it**:
```shell
rex run my_experiment.py
```

The output file will be written to the current directory as a TOML file named after your session.

# Usage
Once installed, `rex` can be invoked in the terminal with the command `rex`:
```
❯ rex
A commandline DAQ management tool

Usage: rex [OPTIONS] <COMMAND>

Commands:
  run    A commandline DAQ runner
  view   A commandline DAQ viewer
  serve  A commandline DAQ server
  help   Print this message or the help of the given subcommand(s)

Options:
  -v, --verbosity <VERBOSITY>  desired log level, info displays summary of connected instruments & recent data. debug will include all data, including standard output from Python [default: 2]
  -h, --help                   Print help
  -V, --version                Print version
```

## Configuration

Before rex can be used, you must set up its config file. Rex looks for its config file in:
- **Linux**: `~/.config/rex/config.toml`
- **macOS**: `~/Library/Application Support/rex/config.toml`
- **Windows**: `AppData/Roaming/rex/config.toml`

You can override the config directory on any platform by setting the `XDG_CONFIG_HOME` environment variable. For example, macOS users who prefer `~/.config/` can set `XDG_CONFIG_HOME=~/.config`.

The layout of the config file is as follows:
```toml
[general]
port = "7676" # Port for the TCP server to listen on — your experiment script will need to send data to this port.

interpreter = "/path/to/desired/interpreter" # e.g. python3 / matlab — this is what will be used to run your experiment scripts.

validations = ["some_key", "some_other_key"] # Optional. Ensures these keys exist and are non-empty in session metadata. If validation fails after 3 retries (5 seconds apart), the session is terminated early. See the Validation section below for details.

subsampling = true # Optional (default: true). Enables LTTB (Largest Triangle Three Buckets) downsampling for data streams in the TUI and /datastream endpoint. Set to false to receive the raw last 100 data points instead.

allowed_output_dirs = ["/path/to/allowed/dir1", "/path/to/allowed/dir2"] # Optional. Restricts where output files can be written. If omitted, defaults to the current working directory and home directory. Primarily useful when running `rex serve` to constrain remote callers.

theme = "dracula" # Optional (default: "dracula"). Color theme for the TUI. See "TUI Theming" below for all available themes.

[email_server]
security = true # If set to true, you must provide a username and password.
server = "smtp.server.com" # SMTP server address.
port = "587" # Optional. SMTP port — useful if your server uses a non-standard port.
from_address = "Rex <rex.experiment@rex.com>" # Configurable from address.

username = "rex_user" # Optional (required if security = true). Your email address / SMTP username.
password = "rex_admin" # Optional (required if security = true). Your email password. If using Google's SMTP server, this is your application password.

[click_house_server]
# you can inspect the table names in the db-support file if you are using it and use them here as well as the user name and password set in the docker compose file
server = "http://server_address"
port = "8123" # ClickHouse HTTP port
username = "your_username"
password = "a_strong_password"
database = "default"
measurement_table = "your_measurement_table" 
session_meta_table = "your_session_meta_data_table"
device_meta_table = "your_device_meta_table"
```
Both the email service and database backend are optional and not required for regular use. Documentation on how to set up the corresponding ClickHouse DB can be found [here](https://github.com/JaminMartin/rex/tree/master/db-support).

### Validation

When the `validations` field is set in `[general]`, rex checks that your session's metadata contains each listed key with a non-empty value. Validation runs every 3 seconds once data starts arriving. If the session metadata is missing or any required key is absent/empty, rex retries up to 3 times (5 seconds apart). If validation still fails after all retries, the session is terminated and no data is written.

This is useful for enforcing that critical metadata (e.g. sample IDs, operator names, calibration references) is always present before data is committed.

## Rex run

Rex run is the core command runner utility. It creates a TCP server and listens for data arriving from the corresponding script that is run (e.g. from Python or Matlab):
```
❯ rex run --help
A commandline DAQ runner

Usage: rex run [OPTIONS] <SCRIPT>

Arguments:
  <SCRIPT>  Path to script containing the session setup / control flow

Options:
  -e, --email <EMAIL>     Email address to receive results
  -d, --delay <DELAY>     Time delay in minutes before starting the session [default: 0]
  -l, --loops <LOOPS>     Number of times to loop the session [default: 1]
  -n, --dry-run           Dry run, will not log data. Can be used for long term monitoring
  -o, --output <OUTPUT>   Target directory for output path [default: current directory]
  -i, --interactive       Enable interactive TUI mode
  -P, --port <PORT>       Port override, allows for overriding default port. Will export this as environment variable for devices to utilise
  -c, --config <CONFIG>   Optional path to config file used by DAQ script (python, matlab etc). Useful when it is critical the script goes unmodified.
      --meta-json <JSON>  Additional metadata JSON that will be stored as part of the run
  -h, --help              Print help
  -V, --version           Print version
```

The `--port`, `--config`, `--meta-json` flags provide additional flexibility for dynamic or simplifying repeated measurements.

### Port overrides
Port overrides are primarily used for multiple instances of `rex` running on the same device. This can currently only be achieved through the `rex run` sub command, not via `rex serve` currently. It exports the `REX_PORT` environment variable with the port the TCP server is running on, so if your scripts read this variable you can configure multiple streams of measurements to run side by side.

### Config overrides

Config overrides allow a single Python or Matlab script to be written with a default config path that can be overridden by environment variables set by rex.

Rex sets one of two environment variables depending on what you pass to `--config`:

- **If `--config` is a file path** (the file exists on disk): Rex sets `REX_PROVIDED_CONFIG_PATH` to the path you provided.
- **If `--config` is a JSON string**: Rex deserialises it into a session + device configuration, writes it to a temporary TOML file, and sets `REX_PROVIDED_OVERWRITE_PATH` to that temp file's path.

Your script should check for these environment variables and fall back to its own default:
```python
import os

class Session:
    def __init__(self, config_path):
        self.name = "session"
        # Check for rex-provided config, fall back to the default
        self.config_path = os.environ.get(
            "REX_PROVIDED_CONFIG_PATH",
            os.environ.get("REX_PROVIDED_OVERWRITE_PATH", config_path)
        )
```
Devices can be configured in the same way.

The JSON form is particularly useful with `rex serve`, where a remote caller can POST session and device configuration without needing to place files on the server's filesystem.

### Additional metadata

The `--meta-json` argument takes a JSON string and includes it into the session data. It is primarily used within `rex serve` for accepting metadata from remote execution targets.

## Data structures

For complete, ready-to-use Python implementations of all the data structures below, see [rex_utils](https://github.com/JaminMartin/rex_utils). The following sections describe the underlying wire format for those building clients in other languages.

### Session
The minimal session payload that must be constructed:
```python
payload = {
    "info": {
        "name": info_data.get("name"),
        "email": info_data.get("email"),
        "session_name": info_data.get("session_name"),
        "session_description": info_data.get("session_description")
    }
}
```
Which will be deserialised into this Rust struct. If experiment, test or run would be your preferred internal naming scheme — you can use that instead :). A session info packet only needs to be sent once; subsequent packets will be rejected unless it is a session metadata packet.
```rust
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
```

Session metadata can be included as a sub dictionary with the field "meta" added:
```rust
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SessionMetadata {
    #[serde(flatten)]
    pub meta: HashMap<String, Value>,
}
```
### Devices
Devices have a slightly more complicated structure that is as flexible as possible so that you can have nested keys for more advanced device configuration.
```rust
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
```

This packet needs to be sent per measurement. This can look like so:
```python
self.measurements = {
    "counts": {"data": [], "unit": "dimensionless"},
    "current (mA)": {"data": [], "unit": "mA"}
}

payload = {
    "device_name": self.name,
    "device_config": self.config,
    "measurements": self.measurements,
}
```
where config is any nested key-value pair. It is important to note that the timestamps field in the device struct is automatically populated at the arrival time of the data.

The key complication here is the ability to send either a `Vec<f64>` or a `Vec<Vec<f64>>`. This is to allow for sending traces / packets of data like an entire oscilloscope trace. You cannot change this type during a session, so ensure your packets are either sent like `[0.111]` for single values per iteration or `[0.111, 0.444, 0.777]` for packets of data. This is automatically mapped into appropriate structs for the TUI and database backends.

### Listener
The Listener can trigger the interop pausing between Rex and the running script. See [rex_utils](https://github.com/JaminMartin/rex_utils) for a complete Python implementation. The wire format is:
```python
payload = {
    "name": "Listener",
    "id": "some_identifier",
}
```

When sent over TCP, the server responds with either `"Running\n"` or `"Paused\n"`. Your script should poll this and block while paused. This enables pausing and continuing through `rex view` and `rex serve`.

Note: the TCP socket, send and receive need to be implemented to your requirements yourself, or use the [rex_utils](https://github.com/JaminMartin/rex_utils) Python package which handles this for you.

## Output file format

Rex writes session data to a human-readable TOML file. The file is named `<session_name>_<timestamp>.toml` and placed in the output directory (current directory by default, or as specified by `--output`).

The structure looks like this:
```toml
[session]
start_time = "2024-06-15T10:30:00.000+12:00"
end_time = "2024-06-15T10:35:12.345+12:00"
UUID = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

[session.info]
name = "Alice"
email = "alice@example.com"
session_name = "my_experiment"
session_description = "Measuring voltage sweep"

[session.info.meta]
sample_id = "SAMPLE_001"
temperature = 293.15

[device.sensor_1]
device_name = "sensor_1"
gain = 1.5
integration_time = 0.1

[device.sensor_1.data.voltage]
unit = "V"
data = [0.0, 0.1, 0.2, 0.3, 0.4]

[device.sensor_1.data.current]
unit = "A"
data = [0.0, 0.01, 0.02, 0.03, 0.04]

[device.sensor_1.timestamps]
voltage = [
    "2024-06-15T10:30:01Z",
    "2024-06-15T10:30:02Z",
    "2024-06-15T10:30:03Z",
    "2024-06-15T10:30:04Z",
    "2024-06-15T10:30:05Z",
]
current = [
    "2024-06-15T10:30:01Z",
    "2024-06-15T10:30:02Z",
    "2024-06-15T10:30:03Z",
    "2024-06-15T10:30:04Z",
    "2024-06-15T10:30:05Z",
]
```

The file is periodically written during the session (every ~3 seconds) so that data is preserved even if the session crashes. On graceful shutdown, final end timestamps are appended and the file is written one last time.

## Rex view
Rex bundles an interactive TUI viewer that can connect to a running `rex` instance either directly via TCP or through a `rex serve` HTTP server. It can be used to remotely monitor, pause/continue, or kill sessions. The TUI also enables plotting any data currently held by the session on an X,Y graph.

```
❯ rex view -h
A commandline DAQ viewer

Usage: rex view [OPTIONS] <ADDRESS>

Arguments:
  <ADDRESS>  Address of the running rex instance (e.g. 127.0.0.1:7676 for TCP, or 127.0.0.1:9000 for HTTP)

Options:
  -b, --backend <BACKEND>  Network backend to use for connecting to the rex instance [default: tcp] [possible values: http, tcp]
  -h, --help               Print help
  -V, --version            Print version
```

- **TCP mode** (`--backend tcp`, the default): Connect directly to the TCP server started by `rex run`. Use the address and port from your `config.toml` `[general].port` (default `127.0.0.1:7676`). Supports full control — monitoring, pause, resume, kill, editing session/device config, loading local config and script files, and starting new local runs.
- **HTTP mode** (`--backend http`): Connect to a `rex serve` instance. Use the serve address (default `127.0.0.1:9000`). Supports the full feature set including editing session configuration, browsing registered scripts from the server's scripts directory, and starting new runs. When using HTTP, scripts must come from the server's registered [scripts directory](#scripts-directory).

### TUI keybindings

#### Chart tab
| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate devices |
| `←` / `→` | Navigate data streams |
| `x` | Set X axis |
| `y` | Set Y axis |
| `c` | Clear axis selection |
| `k` | Kill server |
| `p` | Pause server |
| `r` | Resume server |
| `n` | Start new run |

#### State tab
| Key | Action |
|-----|--------|
| `f` | Toggle between Session / Device sections |
| `↑` / `↓` | Navigate session fields or devices |
| `←` / `→` | Navigate device config fields |
| `e` | Edit selected field (when no session is running) |
| `l` | Load config and script files from disk |
| `n` | Start new run with current config |
| `k` | Kill server |
| `p` | Pause server |
| `r` | Resume server |

#### Global
| Key | Action |
|-----|--------|
| `Tab` | Switch between Chart / State tabs |
| `m` | Toggle help popup |
| `q` | Quit |

### Starting new runs from the TUI

The TUI allows you to start new sessions without leaving the interface:

1. Press `l` to load a config file (`.toml`) and then a script file (`.py`, `.rs`, `.m`). When connected via HTTP, scripts are fetched from the server's registered [scripts directory](#scripts-directory) instead.
2. In the **State** tab, you can view and edit session info and device configuration fields with `e`.
3. Press `n` to start a new run. A popup lets you configure output directory, loop count, delay, and dry-run mode before confirming.

When connected via TCP the new run is spawned locally. When connected via HTTP the run is dispatched to the `rex serve` instance via the `/run` endpoint.

### TUI Theming

The TUI supports configurable color themes via the [`ratatui-themes`](https://crates.io/crates/ratatui-themes) crate. Set the `theme` field in `[general]` of your `config.toml`:

```toml
[general]
theme = "tokyo-night"
```

If omitted, the default theme is **Dracula**. Theme names are specified in kebab-case.

#### Available themes

| Theme | Type | Config value |
|-------|------|-------------|
| Dracula | Dark | `dracula` |
| One Dark Pro | Dark | `one-dark-pro` |
| Nord | Dark | `nord` |
| Catppuccin Mocha | Dark | `catppuccin-mocha` |
| Catppuccin Latte | Light | `catppuccin-latte` |
| Gruvbox Dark | Dark | `gruvbox-dark` |
| Gruvbox Light | Light | `gruvbox-light` |
| Tokyo Night | Dark | `tokyo-night` |
| Solarized Dark | Dark | `solarized-dark` |
| Solarized Light | Light | `solarized-light` |
| Monokai Pro | Dark | `monokai-pro` |
| Rosé Pine | Dark | `rose-pine` |
| Kanagawa | Dark | `kanagawa` |
| Everforest | Dark | `everforest` |
| Cyberpunk | Dark | `cyberpunk` |

Each theme provides a consistent semantic color palette (accent, info, success, warning, error, muted, etc.) that is applied across the entire TUI — tabs, borders, lists, popups, chart data, and log output all adapt automatically.

## Scripts directory

When using `rex serve`, scripts can be registered by placing them in the rex scripts directory:
- **Linux**: `~/.config/rex/scripts/`
- **macOS**: `~/Library/Application Support/rex/scripts/`

Rex scans this directory (up to 4 levels deep) for `.py`, `.rs`, and `.m` files. These scripts are then made available through the `/allowed_scripts` API endpoint and in the TUI when connected via HTTP.

This can be overridden by setting the `XDG_CONFIG_HOME` environment variable, in which case scripts are read from `$XDG_CONFIG_HOME/rex/scripts/`.

## Rex Serve

Rex serve allows for remotely starting `rex run` and also provides remote control functionality found in the TUI. This is perfect for integration with more advanced graphical user interfaces.

```
❯ rex serve -h
A commandline DAQ server

Options:
  -p, --port <PORT>  Port to listen on for the HTTP API server [default: 9000]
  -h, --help         Print help
  -V, --version      Print version
```

### API Endpoints

Base URL: `http://localhost:<PORT>` (default port: 9000)

#### `GET /`

Check server status.

Returns: `"Server is up!"`

#### `POST /run`

Start a new session.

Body:
```json
{
  "path": "/path/to/script.py",
  "output": "/path/to/output/dir",
  "loops": 1,
  "delay": 0,
  "dry_run": false,
  "interactive": false,
  "email": null,
  "port": null,
  "config": null,
  "meta_json": null
}
```

Success (200):
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "message": "session started"
}
```

If session already running:
```json
{
  "id": "None",
  "message": "Session is already running, ignoring request"
}
```

#### `GET /datastream`

Fetch live data stream (the last 100 data points per channel, with LTTB downsampling if enabled).

Returns parsed JSON or 502 if no active session.

#### `GET /status`

Get current session state — session info, device configs, and run file path.

Returns parsed JSON or 502 if no active session.

#### `POST /pause`

Pause the session. Scripts using a Listener will block until resumed.

Returns plain text response. 502 on TCP error.

#### `POST /continue`

Resume a paused session.

Same response format as `/pause`.

#### `POST /kill`

Kill the current session — triggers a graceful shutdown so all data is stored.

Same response format as `/pause`.

#### `GET /status_check`

Lightweight check for whether a session is currently running.

Returns `200 OK` if a session is active, `204 No Content` if idle. Useful for polling without the overhead of fetching the full session state.

#### `GET /allowed_scripts`

Lists scripts registered in the rex scripts directory (see [Scripts directory](#scripts-directory)).

Returns:
```json
{
  "base_dir": "/home/user/.config/rex/scripts",
  "files": [
    "/home/user/.config/rex/scripts/my_experiment.py",
    "/home/user/.config/rex/scripts/calibration/cal_sweep.py"
  ]
}
```

#### `GET /allowed_output_dirs`

Returns the list of allowed output directories from the `allowed_output_dirs` config field (or the defaults if not set).

Returns:
```json
{
  "dirs": ["/home/user/data", "/home/user"]
}
```

## Environment variables

Rex sets several environment variables that your experiment scripts can read:

| Variable | Set when | Description |
|----------|----------|-------------|
| `REX_PORT` | Always during `rex run` | The port the TCP server is listening on. Useful for scripts that need to discover the port dynamically. |
| `REX_STORE` | Always during `rex run` | Path to the system temp directory. Used internally for intermediate storage. |
| `REX_UUID` | Always during `rex run` | The UUID for the current session. |
| `REX_PROVIDED_CONFIG_PATH` | `--config` points to an existing file | Path to the config file provided via `--config`. |
| `REX_PROVIDED_OVERWRITE_PATH` | `--config` is a JSON string | Path to a temporary TOML file generated from the JSON config override. |

Rex also respects:

| Variable | Description |
|----------|-------------|
| `XDG_CONFIG_HOME` | Overrides the default config directory location on any platform. Rex looks for its config at `$XDG_CONFIG_HOME/rex/config.toml` and scripts at `$XDG_CONFIG_HOME/rex/scripts/`. |

# Roadmap

- [ ] Universal scripting language support
- [ ] Configurable subsampling via the REST API
- [ ] Web based front end similar to the TUI view using Leptos & Charming (Apache Echarts)
- [ ] Define drivers in multiple languages and use them together through the universial scripting language
# Projects using Rex
To get some ideas of how to use rex, check out these projects using it.

- [spcs-instruments](https://github.com/JaminMartin/spcs_instruments/tree/master)
- [rex_utils](https://github.com/JaminMartin/rex_utils) — Python utilities for creating devices and sessions
