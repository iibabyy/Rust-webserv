#[allow(non_snake_case)]
#[allow(dead_code)]
mod parsing;
mod client;
#[allow(dead_code)]
mod request;
#[allow(dead_code)]
mod response;
mod server;
mod listener;

use listener::Listener;
use server::server::Server;
use std::net::IpAddr;
use tokio::{signal, task::JoinSet};
use tokio_util::sync::CancellationToken;
use parsing::*;

fn listen_signals(cancel_token: &CancellationToken) {
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

#[tokio::main(flavor = "current_thread")]
async fn main() {

	//TODO: option '-t' for config file check

    let cancel_token = CancellationToken::new();
    let config = parsing::get_config("conf.conf".to_owned()).await;
    let servers = match Server::init_servers(config) {
        Ok(vec) => vec,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    //TODO: check servers (listen obligatory (can be set to a default port) ) (only one default, ..? )

	let addr = IpAddr::from([127, 0, 0, 1]);
	let listeners = match Listener::init_listeners(addr, servers, &cancel_token).await {
		Ok(listeners) => listeners,
		Err(err) => { return eprintln!("Error: {err}") }
	};

    listen_signals(&cancel_token);

    let mut task = JoinSet::new();
    for serv in listeners {
        task.spawn(
            serv.listen(),
        );
    }

    while let Some(res) = task.join_next().await {
        match res {
            Err(e) => {
                println!("----[Error: {e}]----")
            }
            Ok(_) => {}
        }
    }
}
