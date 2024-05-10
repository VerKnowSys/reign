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
