use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pyd")]
#[command(about = "💀🦈 PyDead-BIB v6.0 - Python + Vulkan Acceleration", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compiles and executes a Python script on the GPU
    Run {
        /// Path to the .py file to execute
        #[arg(short, long)]
        file: String,
    },
    /// Shows Vulkan engine and compiler information
    Info,
}
