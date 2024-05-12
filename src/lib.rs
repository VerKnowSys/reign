//! Shable reign.rs

//! Crate docs

#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    unstable_features,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
#![allow(unused_imports)]


pub use anyhow::{anyhow, Error};
pub use tracing::{debug, error, info, instrument, trace, warn};
pub use tracing_subscriber::{fmt, EnvFilter};
pub use tracing_subscriber::{
    fmt::{
        format::{Compact, DefaultFields, Format},
        Layer, *,
    },
    layer::Layered,
    reload::*,
    Registry,
};

/// default dirs to sync
pub const DEFAULT_DIRS: [&str; 6] = ["bin", "facts", "lib", "reigns", "tasks", "templates"];

/// default file patterns to sync
pub const DEFAULT_FILES: [&str; 2] = ["inventory", "*.sql"];

/// default Shable repo to sync
pub const DEFAULT_SHABLE_DIR: &str = "/Volumes/Projects/Shable.Centra";

/// default compression type? - the fastest one
pub const DEFAULT_ARCHIVE_EXT: &str = ".tar.zst";


/// utility functions
pub mod utils;

pub use utils::*;

/// defines async API to invoke shell commands
pub mod commands;

pub use commands::*;


#[derive(Default, Clone, Debug)]
/// Shable Reign Operation
pub struct ReignOperation {
    /// operation UUID
    pub op_uuid: String,
    /// inventory file name and the group (optional)
    pub inventory: String,
    /// shable reign name
    pub reign_name: String,
    /// remote user to use when connecting via the SSH
    pub remote_user: String,
    /// remote host to run reign on
    pub remote_host: String,
    /// local environment from which we run the reign
    pub default_env: Vec<(String, String)>,
}

impl ReignOperation {
    /// Create a new Shable Reign operation
    pub fn new(reign_name: &str, inventory: &str, remote_host: &str) -> Self {
        let op_uuid = uuidv4::uuid::v4();
        let reign_name = reign_name.to_owned();
        let remote_host = remote_host.to_owned();
        let inventory = inventory.to_owned();

        // shable significant env variables:
        let remote_user = std::env::var("RUN_AS").unwrap_or_default();
        let debug_shable = std::env::var("DEBUG").unwrap_or_default();
        let validation_shable = std::env::var("SKIP_ENV_VALIDATION").unwrap_or_default();

        // initialize env for all future commands:
        let default_env = vec![
            (String::from("DEBUG"), debug_shable),
            (String::from("RUN_AS"), remote_user.to_owned()),
            (String::from("SKIP_ENV_VALIDATION"), validation_shable),
        ];
        Self {
            op_uuid,
            remote_user,
            remote_host,
            inventory,
            reign_name,
            default_env,
        }
    }

    /// produce a default remote path to sync to
    pub fn remote_project_path(&self) -> String {
        format!("/tmp/reigns_{}", self.op_uuid)
    }

    /// produce remote user prefix for ssh command
    pub fn remote_user_ssh(&self) -> String {
        if self.remote_user.is_empty() {
            String::new()
        } else {
            format!("{}@", self.remote_user)
        }
    }
}
