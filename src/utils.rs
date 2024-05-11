use crate::*;
use glob::glob;
use std::path::Path;

// use tokio::fs::OpenOptions;
// use tokio::io::AsyncBufReadExt;
// use tokio::io::AsyncWriteExt;
// use tokio::io::BufReader;
use tokio::process::Command;

use std::{
    process::{exit, ExitStatus, Output, Stdio, Termination},
    thread,
};

/// Helper type for Instrumentation handle
pub type TracingEnvFilterHandle =
    Handle<EnvFilter, Layered<Layer<Registry, DefaultFields, Format<Compact>>, Registry>>;


/// Initialize logger and tracingformatter and return a handle
#[instrument]
pub fn initialize_logger() -> TracingEnvFilterHandle {
    let env_log_filter = match EnvFilter::try_from_env("LOG") {
        Ok(env_value_from_env) => env_value_from_env,
        Err(_) => EnvFilter::from("info"),
    };
    let fmt = fmt()
        .compact()
        .with_target(true)
        .with_line_number(false)
        .with_file(false)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_ansi(true)
        .with_env_filter(env_log_filter)
        .with_filter_reloading();

    let handle = fmt.reload_handle();
    fmt.init();
    handle
}


/// gather the list of Shable files to sync
#[instrument]
pub async fn gather_files_to_sync() -> Result<Vec<String>, Error> {
    // sync: build the file list to sync:
    let mut files_to_sync: Vec<String> = vec![];
    for file in DEFAULT_FILES {
        glob(&format!("{DEFAULT_SHABLE_DIR}/{file}*"))?
            .filter_map(Result::ok)
            .filter(|file| file.is_file())
            .for_each(|file| {
                let a_file = file.into_os_string().into_string().unwrap_or_default();
                files_to_sync.push(a_file)
            });
    }
    for files in DEFAULT_DIRS {
        glob(&format!("{DEFAULT_SHABLE_DIR}/{files}/**/*"))?
            .filter_map(Result::ok)
            .filter(|file| file.is_file())
            .for_each(|file| {
                let a_file = file.into_os_string().into_string().unwrap_or_default();
                files_to_sync.push(a_file);
            });
    }
    trace!("Files to sync: {files_to_sync:#?}");
    Ok(files_to_sync)
}


/// helper to easily get value from env slice
#[instrument(skip(default_env))]
pub fn read_env(default_env: &[(&str, &str)], key: &str) -> String {
    default_env
        .iter()
        .find(|(k, _v)| *k == key)
        .unwrap_or(&(key, ""))
        .1
        .to_string()
}
