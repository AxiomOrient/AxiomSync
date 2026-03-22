use std::fs;
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

use crate::command_line::ConnectorName;
use crate::domain::{ConnectorBatchInput, CursorInput, RawEventInput};
use crate::error::{AxiomError, Result};
use crate::http_api;
use crate::kernel::AxiomSync;
use crate::logic::deterministic_directory_cursor;
use crate::ports::ConnectorPort;
use crate::print_json;

mod adapter;
mod parse;
mod runtime;

pub use adapter::ConnectorAdapter;
