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

pub async fn get_config(path: String) -> Vec<ServerBlock> {
	let mut file = File::open(path.as_str())
		.await
		.expect(format!("failed to open {} !", path).as_str());
	let mut content = String::new();
	file.read_to_string(&mut content)
		.await
		.expect(format!("failed to read into {} !", path).as_str());
	let (_, servers) = config(content.as_str()).expect("----[Bad config file !]----");

	eprintln!("----[Parsing réussi !]----");
	servers
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
