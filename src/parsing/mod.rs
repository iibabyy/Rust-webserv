mod config_parsing;

use crate::{server::Server, utils::Args};

use std::{collections::HashMap, path::PathBuf};
use tokio::{fs::File, io::AsyncReadExt as _};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LocationBlock {
    pub modifier: Option<String>,
    pub path: String,
    pub directives: HashMap<String, Vec<String>>,
    pub cgi: HashMap<String, PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ServerBlock {
    pub directives: HashMap<String, Vec<String>>,
    pub locations: HashMap<String, LocationBlock>,
    pub cgi: HashMap<String, PathBuf>,
}

pub async fn handle_parsing(args: &Args) -> anyhow::Result<HashMap<u16, Vec<Server>>> {
    let config = get_config(&args.config_file)
        .await
        .map_err(|err| anyhow::anyhow!("Error: parsing: {err}"))?;

    let servers =
        Server::init_servers(config).map_err(|err| anyhow::anyhow!("Error: parsing: {err}"))?;

    Server::parse_servers(servers).map_err(|err| anyhow::anyhow!("Error: parsing: {err}"))
}

pub async fn get_config(path: &str) -> Result<Vec<ServerBlock>, String> {
    let mut file = match File::open(path).await {
        Ok(file) => file,
        Err(err) => return Err(format!("{path}: {err}")),
    };
    let mut content = String::new();
    match file.read_to_string(&mut content).await {
        Ok(_) => (),
        Err(err) => return Err(format!("failed to read {path}: {err}")),
    }

    let (_, servers) = match config_parsing::parse_config(content.as_str()) {
        Ok(config) => config,
        Err(err) => return Err(format!("Bad config file: {err}")),
    };

    Ok(servers)
}

#[allow(unused)]
impl ServerBlock {
    pub fn get(&self, name: String) -> Vec<String> {
        let value = self.directives.get(&name);

        if let Some(directive) = value {
            directive.clone()
        } else {
            vec![]
        }
    }

    pub fn get_location(&self, path: String) -> Option<LocationBlock> {
        self.locations.get(&path).cloned()
    }
}
