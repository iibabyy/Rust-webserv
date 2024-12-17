#[allow(non_snake_case)]
mod Parsing;
mod client;
mod request;
mod response;
mod server;
mod traits;
mod connection;

use server::server::Server;
use std::net::IpAddr;
use tokio::{signal, task::JoinSet};
use tokio_util::sync::CancellationToken;
use Parsing::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {

	//TODO: option '-t' for config file check

    let cancel_token = CancellationToken::new();
    let config = Parsing::get_config("conf.conf".to_owned()).await;
    let servers = match Server::init_servers(config) {
        Ok(vec) => vec,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    //TODO: check servers (listen obligatory (can be set to a default port) ) (only one default, ..? )

    tokio::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            if let Ok(()) = signal::ctrl_c().await {
                println!(" Server shutdown");
                cancel_token.cancel();
            }
        }
    });

    // println!(
    //     "--------------------[ CONFIG ]--------------------\n\n{:#?}",
    //     servers
    // );
    // println!("--------------------------------------------------\n");

    //TODO Refactor:
    //TODO	servers must not listen on ports themselves (multiple servers should be able to listen one 1 port)
    //TODO		-> Set listeners, who listen on ports, deserialize headers, then, choose the right server
    //TODO				.right server is choose in this order:
    //TODO						1. server_name
    //TODO						2. default server
    //TODO						3. first server in config

    let mut task = JoinSet::new();
    for serv in &servers {
        task.spawn(
            serv.to_owned()
                .run(IpAddr::from([127, 0, 0, 1]), cancel_token.clone()),
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
