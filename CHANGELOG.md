## [1.1.0-alpha.1] - 2026-01-17

### 🚀 Features

- [**breaking**] Updated remote timestamp priority
- Added sub sampling
- Improve device data collection handling
- Added session config view and editing
- Alpha version of HTTP viewer (WIP)
- Configuration of sub sampling
- Removal of openssl

### 🐛 Bug Fixes

- Added better support for nix on MacOS
- Updated rex core to handle new TUI feats
- Better rerun support for local TCP connection
- Build pineline for python build
- Updated pyproject.toml to maturin docs
- Updated deploy ci/cd to deploy to test pypi

### 💼 Other

- Version bump to reflect small changes
- Updated changelog
- Improved macos support
- Updated changelog
- Removal of openssl in pipeline

### 🚜 Refactor

- Removal of session results
- Refactored into single crate, closes #13
- Refactored tests for new layout

### ⚙️ Miscellaneous Tasks

- Using cliff.toml for git cliff
- Start of async refactor for TUI veiwer
- Version bump
## [1.0.0] - 2025-09-15

### 🐛 Bug Fixes

- Fixed UV caching

### 💼 Other

- Updated *.nix files for nixos install
- Updated pyproject.toml to fix build
- Updated toml manifests
- Revised manifest
- Updated manifests for workspace
- Maturin include LICENSE file
- Update LICENSE
- Updated flake.
- Updated workflow to use UV
## [1.0.0-alpha.1] - 2025-09-14

### 🐛 Bug Fixes

- Fixed merge error and formatted

### 💼 Other

- Updated dashbaord
- Merge branch 'master' into devel
- Merge pull request #4 from JaminMartin/devel

Devel
- Updated docs
- Merge pull request #5 from JaminMartin/devel

updated docs
- Fixed typo in docs
## [0.9.4-alpha.2] - 2025-05-27

### 💼 Other

- Minimal database & viewing configuration
- Updated docs
- Updated docs
- Added port overides, config overides.
- Added dashboard template
## [0.9.4-alpha.1] - 2025-05-21

### 🐛 Bug Fixes

- Fixes memory issue listed in #2

### 💼 Other

- Version bump to reflect database changes
- Version bump
- House keeping, renaming etc
- Updated docs for database connection
- Updated to reflect refactor & memory issues in #2
Also added vec<vec<f64>> mappings for database insertion
Also added json data serialisation for device configurations.

### 🚜 Refactor

- Refactoring some code for readability.
## [0.9.3] - 2025-05-14

### 💼 Other

- Hot fix for versioning, and typos
- Pyproject.toml version bump
## [0.9.2] - 2025-05-14

### 🚀 Features

- [**breaking**] Added server support, metadata logging
- [**breaking**] Removal of python bindings
- [**breaking**] Added stricter data types for measurements
- [**breaking**] Added more endpoints
- [**breaking**] Remote overides

### 🐛 Bug Fixes

- Fixes memory issue listed in #2
- Fixed fallback port bugs
- Fixed errors in re-connection
added automatic axis scaling
- Fixed LICENSE
- Fixed tests
- Fixed build pipeline due to openssl...

### 💼 Other

- Updated to be more python agnostic
- Updated to respect XDG_CONFIG_HOME env on MacOS
- Initial implementation of clickhouse logging
- Database is now configurable.
- Merge pull request #1 from JaminMartin/devel

Merge database features into main branch.
- Hot fix for versioning, and typos
- Pyproject.toml version bump
- Version bump to reflect database changes
- Version bump
- House keeping, renaming etc
- Updated docs for database connection
- Updated to reflect refactor & memory issues in #2
Also added vec<vec<f64>> mappings for database insertion
Also added json data serialisation for device configurations.
- Minimal database & viewing configuration
- Updated docs
- Updated docs
- Added port overides, config overides.
- Added dashboard template
- Updated dashbaord
- Added flake
- Updated error handling and broken file paths
- Updated file name handling logic
- Updated tests to support timestamps
- Now supports timestamps
- Automatic timestamp generation and managment
- Added suport for time in clickhouse
- Version bump + dependency refactor
- Updated CLI printing
- Updated flakes
- Added server support for remote job execution
- Added new build stage
- Optional just file for running on nix easily
- Updated flakes
- Updated gitignore for nix builds
- New rust only workflow
- Merge branch 'devel' of github.com:JaminMartin/rex into devel
- Removal of python bindings
- Minor formatting
- Deprecation of pyo3 bindings for cli
- Patch to pipeline for windows
- Trying to fix github workflow for windows
- Reducing windows memory by building for release
- New workflows
- Pure rust build
- Updated flake for development
- Updated supporting db tools
- Version bump for v1.0.0
- Re-fixed tests
- Now works with upper and lower bounds
- Custom additional session validations
- Custom addtional session validations
- Improvements to validations logic
- Better validations
- Updated init.sql to be inline with rex structs
- Updated pyproject.toml
- Merge pull request #1 from JaminMartin/devel

Merge database features into main branch.

### 🚜 Refactor

- Refactoring some code for readability.
- Refactored cli

BREAKING CHANGE: Rex viewer is has been replaced, rex now uses
rex run in place of the original rex -p... and rex view for the viewer behavior
- Refactored
- Refactored for session rename
- Refactor for session
## [0.9.1] - 2025-05-07

### 🐛 Bug Fixes

- Fixed pyproject.toml and readme.md

### 💼 Other

- Updated README.md
- Minor bug fixes and version bump
- Tidied up workflow and warnings
## [0.9.0] - 2025-05-07

### 💼 Other

- Full deploy
- Updated package name
## [0.9.0-alpha.1] - 2025-05-06

### 🐛 Bug Fixes

- Fixed. Was testing linter inv python venv
- Fixed logic error in looped file names
- Fixed compiling issues aftrer merge.

Loading data is still broken after v0.3.0
- Fixed typo in cli_mod.rs
- Fixed tui control scheme
- Fixed up dry run warning
- Fixes #45
- Fixed bugs in email sender

### 💼 Other

- Added initial basic rust backend for future cli
- Added clap :)
- Cleaned up cli
- Basis of cli structure setup
- Got most features bar attachment working.
- Refactored and started work on file parsing in Rs
- Updated some rust deps to try get tests to work
- Working towards toml outpu
- Added more toml support
- Formatted mailer function to be more idiomatic rust
- Version bumps for 0.2.0 and new api
- Tidied up cli tool and exported pyfunctions
- Added complete file handling and data management
Now writes files to desired path name is inspired by confile + a time stamp.
- Added tests.
Must run with
cargo test --verbose --no-default-features -- --test-threads=1
- Removed unused imports
- Version bump to reflect fixed so loops
- Updated to support multiple identical instruments
- Merge pull request #4 from JaminMartin/spcs-devel

Fixes to looping and file naming
- Added data reader function
- Version bump for new data loading
- Version bump to 0.2.2
- Merge remote-tracking branch 'origin/spcs-devel' into spcs-devel
- Hotfix for 17c0b4c

all tests now passing!
- Version bump to 0.3.1
- Merge pull request #6 from JaminMartin/spcs-devel

Merging devel into master
- Added support for windows+rye pathing
- Merge pull request #11 from JaminMartin/spcs-devel-windows

Spcs devel windows
- Version bump to 0.3.2 to reflect Windows support
- Fixed std out piping issue
- Updated to 0.3.3
- Started adding TCP support
- Updated to 0.4.0
- Started new test suite for tcp data handling
- Upated tests
- Added TCP handler module
- Formatting
- Removal of non-needed python functions
- Updated data handling for TCP sever
- Updated to support tcp data streaming
- Updated tcp code & coresponding working examples
- Better path support for windows
- Merge pull request #20 from JaminMartin/devel-tcp-piping

Devel tcp piping merge into standard dev branch
- Added tui interactive mode
- Updated to support TUI mode
- Updated readme to include new demo example gif
+ version bump to reflect TUI inclusion
- Merge pull request #25 from JaminMartin/devel-tui

Merging tui-devel into spcs-devel
- Merge pull request #26 from JaminMartin/spcs-devel

merging new communication protocol, bug fixes and TUI
- Standalone is now usable
- Minor changes for better standalone support
- Version bump for standalone TUI
- Ground work for pyfex <-> python coms
- Version bump to 0.6 due to breaking changes with required configs
- Fixed bug that causes cli to crash on windows.
resolves #28
- Updated to have better logging
- Improved log handling and python crash dumps
- House keeping, more idiomatic rust
- Merge pull request #29 from JaminMartin/spcs-devel

Updating main from devel for general use as lots has stabilised.
- Merge branch 'spcs-devel' into devel-horiba
- Merge pull request #34 from JaminMartin/devel-horiba

Adding Horiba spectrometer.
- Fixed port to better support windows / lower chances of port clashes
- Added tmp file storage for external real time analysis
- Ensured temp files work on windows as expected
- Updating TUU
- Updated TUI to have better usability.
- Added intial support for trace data
- Updated to v0.7.0
- Merge pull request #36 from JaminMartin/spcs-devel

added new instruments and real time logging features
- Merge branch 'master' into spcs-devel
- Merge pull request #37 from JaminMartin/spcs-devel

Merging new features into main
- Added / updated tests
- Updated data loader for trace data
updated tui menu
tidied cli code
- Version bump to 0.7.1
- Merge pull request #38 from JaminMartin/spcs-devel

Merging latest v0.7.1 features into main
- Updated to v0.7.2
- Added pause/continue to pyfex
- Update to v0.7.3
- Updated command menu
- Merge pull request #41 from JaminMartin/spcs-devel

Merging version 0.7.3 changes into the main branch in preparation for a full release
- V1 of dry run support
- Updated to 0.7.4
- Added config dir support & email configuration
- Version bump for next alpha release
- Version bump fix in line with semver
- Merge pull request #47 from JaminMartin/spcs-devel

BREAKING CHANGES: The email server must now be configured
- Version bump to v0.8.1
- Merge pull request #48 from JaminMartin/spcs-devel

[HOT FIX] fixing email capability and updating corresponding docs
- Upated gitignore
- Complete overhaul for interpreter support

Signed-off-by: JaminMartin <jamin.martin1@gmail.com>
- Dupdated assets

Signed-off-by: JaminMartin <jamin.martin1@gmail.com>
- Updated gitignore
- First deploy of rex.

Signed-off-by: Jamin Martin <jamin.martin1@gmail.com>

### 🚜 Refactor

- Refactor tui to accept standalone config
- Refactored and added standalone tui
