/*
 * Copyright (c) 2023 David Dunwoody.
 *
 * All rights reserved.
 */

use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::format::{Compact, DefaultFields, Format};
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::{fmt, EnvFilter};

#[must_use]
pub fn layer<S>(
    with_thread_names: bool,
    ansi: Option<bool>,
) -> Layer<S, DefaultFields, Format<Compact>> {
    #[cfg(target_os = "windows")]
    let ansi_default = false;
    #[cfg(not(target_os = "windows"))]
    let ansi_default = true;

    let use_ansi = ansi.unwrap_or(ansi_default);

    fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(with_thread_names)
        .with_target(false)
        .with_ansi(use_ansi)
}

#[must_use]
pub fn env_filter(var: Option<&str>) -> EnvFilter {
    let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());
    if let Some(var) = var {
        builder.with_env_var(var).from_env_lossy()
    } else {
        builder.from_env_lossy()
    }
}
