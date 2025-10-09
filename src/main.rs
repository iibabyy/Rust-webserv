mod listener;
mod parsing;
mod request;
mod response;
mod server;
mod utils;

use crate::listener::Listener;
use crate::parsing::*;
use crate::server::Server;
use crate::utils::{get_args, listen_signals};

use std::net::IpAddr;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let (config_file, option_t) = match get_args() {
        Ok(res) => res,
        Err(err) => return eprintln!("{err}"),
    };

    let config_file = if config_file.is_some() {
        config_file.unwrap()
    } else {
        "default.conf".to_owned()
    };

    let cancel_token = CancellationToken::new();

    let config = match parsing::get_config(config_file).await {
        Ok(config) => config,
        Err(err) => return eprintln!("Error: {err}"),
    };

    let servers = match Server::init_servers(config) {
        Ok(vec) => vec,
        Err(e) => return eprintln!("Error: {}", e),
    };

    let servers = match Server::parse_servers(servers) {
        Ok(map) => map,
        Err(err) => return eprintln!("Error: parsing: {err}"),
    };

    if option_t {
        return println!("----[Parsing réussi !]----");
    } // parsing fini

    let addr = IpAddr::from([127, 0, 0, 1]);
    let listeners = match Listener::init_listeners(addr, servers, &cancel_token).await {
        Ok(listeners) => listeners,
        Err(err) => return eprintln!("Error: {err}"),
    };

    listen_signals(&cancel_token);

    let mut task = JoinSet::new();
    for serv in listeners {
        task.spawn(serv.listen());
    }

    while let Some(res) = task.join_next().await {
        if let Err(e) = res {
            eprintln!("----[Error: {e}]----");
        }
    }
}
