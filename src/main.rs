mod client;
mod listener;
#[allow(non_snake_case)]
#[allow(dead_code)]
mod parsing;
#[allow(dead_code)]
mod request;
#[allow(dead_code)]
mod response;
mod server;

use listener::Listener;
use parsing::*;
use server::server::Server;
use std::{env, net::IpAddr};
use tokio::{signal, task::JoinSet};
use tokio_util::sync::CancellationToken;

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

fn get_args() -> Result<(Option<String>, bool), String> {
    let args = env::args();
    let mut option_t = false;
    let mut config = None;

    if args.len() > 3 {
        return Err("Error: too many arguments".to_owned());
    }

    let mut i = 0;
    for arg in args {
        if i == 0 {
            i += 1;
            continue;
        }
        if arg == "t" {
            if option_t == true {
                eprintln!("Warning: duplicate option")
            } else {
                option_t = true
            }
        } else {
            config = Some(arg);
        }
    }

    Ok((config, option_t))
}

#[tokio::main]
async fn main() {
    //TODO: option '-t' for config file check

    let (config_file, option_t) = match get_args() {
        Ok(res) => res,
        Err(err) => return eprintln!("{err}"),
    };

    let config_file = if config_file.is_some() {
        config_file.unwrap()
    } else {
        "conf.conf".to_owned()
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

    if option_t == true { return println!("----[Parsing réussi !]----") } // parsing fini
    ;

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
        match res {
            Err(e) => {
                eprintln!("----[Error: {e}]----")
            }
            Ok(_) => {}
        }
    }
}
