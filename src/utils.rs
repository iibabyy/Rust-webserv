use std::env;
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

pub fn get_args() -> Result<(Option<String>, bool), String> {
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
            if option_t {
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