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
    - beta support for Matlab.
- Supports sending results over email
# Road Map
- first class backend database & Grafana support    
# Install 
clone the repository and run 
```shell
cargo install --path cli/
```
Alternatively, you can embed this in a python project eithe
# Usage
Once installed `rex` can be invoked in the terminal with the command `rex`
```
❯ rex -h
A commandline experiment manager

Usage: rex [OPTIONS] --path <PATH>

Options:
  -v, --verbosity <VERBOSITY>  desired log level, info displays summary of connected instruments & recent data. debug will include all data, including standard output from scripts [default: 2]
  -e, --email <EMAIL>          Email address to receive results
  -d, --delay <DELAY>          Time delay in minutes before starting the experiment [default: 0]
  -l, --loops <LOOPS>          Number of times to loop the experiment [default: 1]
  -p, --path <PATH>            Path to the python file containing the experimental setup
  -n, --dry-run                Dry run, will not log data. Can be used for long term monitoring
  -o, --output <OUTPUT>        Target directory for output path [default: /home/jamin/Documents/rex]
  -i, --interactive            Enable interactive TUI mode
  -h, --help                   Print help
  -V, --version                Print version
```  

However, before it can be used - you must setup its config file. Rex looks for its config file in `.config/rex` on Linux, `Application Support/rex` on Mac and `AppData/Roaming/rex`
the layout of the config file is as such:
```toml
[general]
port = "7676" # port for tcp server to listen on, change as required - note your experiment script will need to send data to this port.

interpreter = "/path/to/desired/interpreter" #e.g. python / matlab this is what will be used to run your experiment scripts 
[email_server]
security = true # if set to true, you must provide a user name and password
server = "smtp.server.com" # smtp server
from_address = "Rex <rex.experiment@rex.com>" # configurable from email
 
username = "rex_user" # your email address
password = "rex_admin" # your email password, if this is using google's smtp server - then it is your application password
```

As rex provides either an interactive mode or logging mode, rex also bundles a seperate binary called rex-viewer. It is a interative mode only experience. It can be used to remotely kill or pause/continue scripts. Rex-viewer only accepts one argument which is the ip address and port of the instance currently running rex-cli.
```
❯ rex-viewer -h
A commandline experiment viewer

Usage: rex-viewer [OPTIONS] --address <ADDRESS>

Options:
  -a, --address <ADDRESS>
  -v, --verbosity <VERBOSITY>  desired log level, info displays summary of connected instruments & recent data. debug will include all data, including standard output from Python [default: 2]
  -h, --help                   Print help
  -V, --version                Print version
```
  
# Projects using Rex
To get some ideas of how to use rex, check out these projects using it.

- [spcs-instruments](https://github.com/JaminMartin/spcs_instruments/tree/master)
