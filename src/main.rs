mod listener;
mod parsing;
mod request;
mod response;
mod server;
mod utils;

use crate::listener::Listener;
use crate::parsing as my_parsing;
use crate::utils::{listen_signals, Args};

use std::error::Error;
use std::net::IpAddr;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::new();
    let servers = my_parsing::handle_parsing(&args).await?;

    // -t/--test -> user only want to check the config file
    if args.test {
        eprintln!("---[ Config file Ok ! ]---");
        return Ok(());
    }

    // Listen for ctrl+C signals
    let cancel_token = CancellationToken::new();
    listen_signals(&cancel_token);

    let addr = IpAddr::from([127, 0, 0, 1]);
    let listeners = Listener::init_listeners(addr, servers, &cancel_token).await?;

    let mut tasks = JoinSet::new();
    for listener in listeners {
        tasks.spawn(listener.listen());
    }

    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            eprintln!("----[Error: {e}]----");
        }
    }

    Ok(())
}
