mod config_parsing;

use std::{collections::HashMap, path::PathBuf};

use config_parsing::config;
use tokio::{fs::File, io::AsyncReadExt as _};

#[derive(Debug, Clone)]
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

pub async fn get_config(path: String) -> Result<Vec<ServerBlock>, String> {
    let mut file = match File::open(path.as_str()).await {
        Ok(file) => file,
        Err(err) => return Err(format!("{path}: {err}")),
    };
    let mut content = String::new();
    match file.read_to_string(&mut content).await {
        Ok(_) => (),
        Err(err) => return Err(format!("failed to read {path}: {err}")),
    }

    let (_, servers) = match config(content.as_str()) {
        Ok(config) => config,
        Err(err) => return Err(format!("Bad config file: {err}")),
    };

    Ok(servers)
}

#[allow(unused)]
impl ServerBlock {
    pub fn get(&self, name: String) -> Vec<String> {
        let value = self.directives.get(&name);

        if value.is_some() {
            value.unwrap().clone()
        } else {
            vec![]
        }
    }

    pub fn get_location(&self, path: String) -> Option<LocationBlock> {
        let value = self.locations.get(&path);

        if value.is_some() {
            Some((*value.unwrap()).clone())
        } else {
            None
        }
    }
}
