use clap::Parser;
use tokio::signal;
use tokio_util::sync::CancellationToken;

pub fn listen_signals(cancel_token: &CancellationToken) {
    tokio::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            if let Ok(()) = signal::ctrl_c().await {
                println!(" Server shutdown");
                cancel_token.cancel();
            }
        }
    });
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Configuration file
    pub file: Option<String>,

    #[arg(short, long, default_value_t = false)]
    pub test: bool,
}

pub struct Args {
    pub config_file: String,
    pub test: bool,
}

impl Args {
    pub fn new() -> Self {
        let cli = CliArgs::parse();

        let config_file = cli.file.unwrap_or("default.conf".to_string());

        Args {
            config_file,
            test: cli.test,
        }
    }
}
