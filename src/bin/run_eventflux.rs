// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::Parser;

use eventflux_rust::core::config::ConfigManager;
use eventflux_rust::core::eventflux_manager::EventFluxManager;
use eventflux_rust::core::persistence::{
    FilePersistenceStore, PersistenceStore, SqlitePersistenceStore,
};
use eventflux_rust::core::stream::output::LogStreamCallback;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(about = "Run a EventFluxQL file", author, version)]
struct Cli {
    /// EventFluxQL file to execute
    eventflux_file: PathBuf,

    /// Directory for file based persistence snapshots
    #[arg(long)]
    persistence_dir: Option<PathBuf>,

    /// Path to a SQLite DB file for persistence
    #[arg(long)]
    sqlite: Option<PathBuf>,

    /// Dynamic extension libraries to load
    #[arg(short = 'e', long)]
    extension: Vec<PathBuf>,

    /// Custom config file or identifier
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut store: Option<Arc<dyn PersistenceStore>> = None;
    if let Some(dir) = cli.persistence_dir {
        match FilePersistenceStore::new(dir.to_str().unwrap()) {
            Ok(s) => store = Some(Arc::new(s)),
            Err(e) => eprintln!("Failed to initialize file store: {e}"),
        }
    } else if let Some(db) = cli.sqlite {
        match SqlitePersistenceStore::new(db.to_str().unwrap()) {
            Ok(s) => store = Some(Arc::new(s)),
            Err(e) => eprintln!("Failed to initialize sqlite store: {e}"),
        }
    }

    let content = match fs::read_to_string(&cli.eventflux_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read {}: {}", cli.eventflux_file.display(), e);
            std::process::exit(1);
        }
    };

    let mut manager = EventFluxManager::new();
    if let Some(s) = store {
        if let Err(e) = manager.set_persistence_store(s) {
            eprintln!("Failed to set persistence store: {e}");
            std::process::exit(1);
        }
    }
    if let Some(cfg) = cli.config {
        let config_manager = ConfigManager::from_file(cfg);
        if let Err(e) = manager.set_config_manager(config_manager) {
            eprintln!("Failed to apply config: {e}");
        }
    }
    for lib in cli.extension {
        if let Some(p) = lib.to_str() {
            let name = lib.file_stem().and_then(|s| s.to_str()).unwrap_or("ext");
            if let Err(e) = manager.set_extension(name, p.to_string()) {
                eprintln!("Failed to load extension {p}: {e}");
            }
        }
    }
    let runtime = match manager
        .create_eventflux_app_runtime_from_string(&content)
        .await
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create runtime: {e}");
            std::process::exit(1);
        }
    };

    for stream_id in runtime.stream_junction_map.keys() {
        let _ = runtime.add_callback(
            stream_id,
            Box::new(LogStreamCallback::new(stream_id.clone())),
        );
    }

    // Start the runtime after adding callbacks
    if let Err(e) = runtime.start() {
        eprintln!("Failed to start runtime: {e}");
        std::process::exit(1);
    }

    println!(
        "Running EventFlux app '{}'. Press Ctrl+C to exit",
        runtime.name
    );

    // Keep the main thread alive until interrupted.
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
