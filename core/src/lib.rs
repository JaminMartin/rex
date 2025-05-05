pub mod cli_tool;
pub mod data_handler;
pub mod mail_handler;
pub mod tcp_handler;
pub mod tui_tool;
#[cfg(feature = "extension-module")]
use pyo3::prelude::*;

#[cfg(feature = "extension-module")]
use cli_tool::{cli_parser_py, cli_standalone};
#[cfg(feature = "extension-module")]
use data_handler::load_experimental_data;

#[cfg(feature = "extension-module")]
#[pymodule]
pub fn rex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cli_parser_py, m)?)?;
    m.add_function(wrap_pyfunction!(cli_standalone, m)?)?;
    m.add_function(wrap_pyfunction!(load_experimental_data, m)?)?;
    Ok(())
}
