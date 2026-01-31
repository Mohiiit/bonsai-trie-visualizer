use clap::Parser;
use tracing::Level;

mod app;
mod bonsai;
mod db;
mod model;
mod ui;
mod util;

#[derive(Debug, Parser)]
#[command(name = "bonsai-trie-visualizer")]
#[command(about = "Visualize Madara Bonsai tries", long_about = None)]
pub struct Args {
    /// Path to Madara RocksDB directory
    #[arg(long, value_name = "PATH")]
    db_path: Option<String>,

    /// Optional block number to view
    #[arg(long, value_name = "N")]
    block: Option<u64>,

    /// Optional diff range: start end
    #[arg(long, num_args = 2, value_names = ["START", "END"])]
    diff: Option<Vec<u64>>,
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args = Args::parse();
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Bonsai Trie Visualizer",
        native_options,
        Box::new(|cc| Ok(Box::new(app::BonsaiApp::new(cc, args)))),
    )
}
