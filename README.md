# Rex, the rust based experimental data manager

![Logo](https://raw.githubusercontent.com/JaminMartin/rex/refs/heads/master/assets/rex.svg)
Build deterministic experiment pipelines in the scripting language of your choice!
# Features
- Rich logging of data collection, both in a log format as well as an interactive interface
- Robust multi-threaded approach to data logging
- Fails safe to prevent data loss
- Human readable data files that can be used to reproduce identical experiments.
- language agnostic, can in principle run and manage data from any scripting language that can send data in an appropriate form over TCP.
    - First class support for python
    - beta support for rustscript
    - alpha support for Matlab.

- Supports sending results over email
- Remote start and monitor with API endpoints.
# Install
clone the repository and run
```shell
cargo install --path cli/
```
Alternatively, you can embed this in a python project, as python bindings are exposed and packaged on `PyPi` as `rex-pycli`.
# Usage
Once installed `rex` can be invoked in the terminal with the command `rex`
```
❯ rex
A commandline experiment management tool

Usage: rex [OPTIONS] <COMMAND>

Commands:
  run    A commandline experiment runner
  view   A commandline experiment viewer
  serve  A commandline experiment server
  help   Print this message or the help of the given subcommand(s)

Options:
  -v, --verbosity <VERBOSITY>  desired log level, info displays summary of connected instruments & recent data. debug will include all data, including standard output from Python [default: 2]
  -h, --help                   Print help
  -V, --version                Print version
```

However, before it can be used - you must setup its config file. Rex looks for its config file in `.config/rex` on Linux, `Application Support/rex` on Mac and `AppData/Roaming/rex`
the layout of the config file is as such:
```toml
[general]
port = "7676" # port for tcp server to listen on, change as required - note your experiment script will need to send data to this port.

interpreter = "/path/to/desired/interpreter" #e.g. python / matlab this is what will be used to run your experiment scripts

validations = ["some_key", "some_other_key"] # Validations ensure certain keys in the session metadata are checked, if they are not there the session is terminated early.
[email_server]
security = true # if set to true, you must provide a user name and password
server = "smtp.server.com" # smtp server
from_address = "Rex <rex.experiment@rex.com>" # configurable from email

username = "rex_user" # your email address
password = "rex_admin" # your email password, if this is using google's smtp server - then it is your application password

[click_house_server]
server = "server_address"
port = "clickhouse_http_port"
password = "a_strong_password"
username = "your_username"
database = "default"
measurement_table = "your_measurement_table"
experiment_meta_table ="your_experiment_meta_data_table"
device_meta_tables = "your_device_meta_table"
```
Both the email service and database backend are optional and not required for regular use. Documentaion on how to setup the coresponding clickhousedb can be found [here](https://github.com/JaminMartin/rex/tree/master/db-support).
## Rex run

Rex run the core command runner utility. It creates the TCP server and listens for data arriving from the coresponding script that is run (e.g. from python or matlab). It is relatively straight forward to use.
```
❯ rex run --help
A commandline DAQ runner

Usage: rex run [OPTIONS] --path <PATH>

Options:
  -e, --email <EMAIL>     Email address to receive results
  -d, --delay <DELAY>     Time delay in minutes before starting the session [default: 0]
  -l, --loops <LOOPS>     Number of times to loop the session [default: 1]
  -p, --path <PATH>       Path to script containing the session setup / control flow
  -n, --dry-run           Dry run, will not log data. Can be used for long term monitoring
  -o, --output <OUTPUT>   Target directory for output path [default: /home/jamin/Documents/Programming/Rust/rex]
  -i, --interactive       Enable interactive TUI mode
  -P, --port <PORT>       Port overide, allows for overiding default port. Will export this as environment variable for devices to utilise
  -c, --config <CONFIG>   Optional path to config file used by DAQ script (python, matlab etc). Useful when it is critical the script goes unmodified.,
      --meta-json <JSON>
  -h, --help              Print help
  -V, --version           Print version
```

The `--port`,`--config`,`--meta-json` flags provide additional flexability for dynamic or simplifying repeated measurements.

### Port overides
Port overides are primarily used for multiple instances of `rex` running on the same device. This can currently only be achieved through the `rex run` sub command, not via `rex serve` currently. It exports an environment variable of the port the TCP server is running on, so if your scripts accept this as the default port you can configure multiple streams of measurements to run side by side.

### Config overides.

Config overides allow for a single `python` or `matlab` script to be written that has some default config path but also prioritises reading from environment variables.
for example, building a session:
```python
class Session:
    def __init__(self, config_path):
        self.name = "session"

        self.config_path = os.environ.get("REX_PROVIDED_CONFIG_PATH", config_path)
```
Devices can be configured in the same way.

The config overide, in order to work synergistically with the serve functionality can also accept a `JSON` string. This gets deserialised into a minimal session configuration and can also accept any additional device configurations.


### Additional metadata.

This argument takes a `JSON` string and includes it into the session data. It is primarily used within `rex serve` for accepting metadata from remote execution targets.

## Data structures

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
```
Which will be deserialised into this rust struct. If experiment, test or run would be your prefered internal naming scheme - you can use that instead :). A session info packet only needs to be sent once, subsequent packets will be rejected unless it is a session metadata packet.
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

Session metadata can be included as a sub dictionary with he field "meta" added.
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
where config is any nested keyvalue pair. It is improtant to note, that the timestamps field in the device struct is automatically populated at the arrival time of the data.

The key complication here is the ability to send either, a Vec<64> or a Vec<Vec<f64>>. This is to allow for sending traces / packets of data like an entire oscilliscope trace. You cannot change this type during a session, so ensure your packets are either sent like `[0.111]` for single values per iteration or `[0.111, 0.444, 0.777]` for packets of data. This is automatically mapped into appropriate structs for the TUI and database backends.

### Results
Results are not part of the device struct as they can be constructed after run time and be formed by post processing on agregated data. The results struct is totally optional and the session will not terminate or fail to write final values if it is not present. The lower bound, upper bound and value fields are all optional. A result status could be that a fit to the captured data was successful or that a validation passed. The result metadata could contain paths to post processed figures. These are all stored in the output data file.

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionResults {
    pub result_name: String,
    pub result_description: String,
    pub result_status: bool,
    #[serde(alias = "upper_bound", alias = "ub")]
    pub result_upper_bound: Option<f64>,
    #[serde(alias = "lower_bound", alias = "lb")]
    pub result_lower_bound: Option<f64>,
    pub result_value: Option<f64>,
    #[serde(default)]
    pub result_meta: HashMap<String, Value>,
}
```

```py
self.payload = {
            "result_name": self.name,
            "result_description": description,
            "result_status": status, # bool
            "lower_bound" : lower_bound,
            "upper_bound" : upper_bound,
            "result_value": value,
        }
# and any key value dict of additional metadata
self.payload["result_meta"] = result_meta
```
### Listener
The Listener can trigger the interop pausing between Rex and the running script. An example implementation of this in python is provided here:
```python
class Listener:
    def __init__(self):
        self.name = "Listener"

        self.sock = self.tcp_connect()
        self.start()
    def start(self):
        self.send()

    def send(self):
        self.payload = {
                "name": "Listener",
                "id": "eaoifhja3por13", # These ID's are likely to be used in future for security purposes provided by another REX ENV VAR
        }

    def check_state(self) -> bool:

        response = self.tcp_send(self.payload,self.sock).strip()
        match response:

            case "Paused":
                while self.tcp_send(self.payload,self.sock).strip() == "Paused":
                    time.sleep(1)

            case "Running":
                  return
```

This can be put into any looping script to enable pausing and continuting through `rex view` and `rex serve`

Note, the TCP socket, send and receive need to be implemented to your requirements yourself.
## Rex view
As rex provides either an interactive mode or logging mode, rex also bundles a TUI viewer. It is an interative mode only experience. It can be used to remotely kill or pause/continue scripts. Rex-viewer only accepts one argument which is the ip address and port of the instance currently running rex-cli. Secure instance connection is a work in progress. The `rex view` TUI enables plotting any data currently held by the session on an X,Y graph.
```
❯ rex view -h
A commandline experiment viewer

Usage: rex-viewer [OPTIONS] --address <ADDRESS>

Options:
  -a, --address <ADDRESS>
  -v, --verbosity <VERBOSITY>  desired log level, info displays summary of connected instruments & recent data. debug will include all data, including standard output from Python [default: 2]
  -h, --help                   Print help
  -V, --version                Print version
```


## Rex Serve

Rex serve allows for remotely starting `rex run` and also provides remote control functionality found in the TUI. This is perfect for integration with more advanced graphical user interfaces.

API Endpoints

Base URL: `http://localhost:<PORT>`

`GET /`

Check server status
Returns: `"Server is up!"`

`POST /run`

Start a new session

Body:
```json
{
  "port": 7676 // optional, overrides default
  // other RunArgs fields...
}
```

Success (200):
```json
{
  "id": "uuid",
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
`GET /datastream`

Fetch live data stream (from TCP server, currently just the last 100 datapoints)

Returns parsed JSON or 502 on error.

`GET /status`

Get current state, e.g. all the current session info.

Returns parsed JSON or 502 on error.

`POST /pause`

Pause the session

Returns plain text response

502 on TCP error

`POST /continue`

Resume a paused session

Same response format as /pause

`POST /kill`

Kill the current session - note this triggers the graceful shutdown so all data is stored.

Same response format as /pause

# Roadmap

- [ ] Tab support in the TUI, per tab figure type configuration
- [ ] Script registration, register a collection of scripts and default configs to run from the TUI or serve endpoint.
- [ ] rex-lite, for multi-node deployment. register scripts on a per node basis and run them via a single rex serve instance.

# Projects using Rex
To get some ideas of how to use rex, check out these projects using it.

- [spcs-instruments](https://github.com/JaminMartin/spcs_instruments/tree/master)
