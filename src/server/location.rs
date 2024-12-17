/*---------------------------------------------------------------*/
/*-------------------------[ LOCATIONS ]-------------------------*/
/*---------------------------------------------------------------*/

use std::{collections::HashMap, path::PathBuf, sync::Arc};


use super::parsing;

use crate::{
    request::request::Method,
    traits::config::Config,
    LocationBlock,
};

use super::server::Server;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Location {
    internal: bool,
    exact_path: bool,
    auto_index: Option<bool>,
    path: PathBuf,
    root: Option<PathBuf>,
    alias: Option<PathBuf>,
    max_body_size: Option<u64>,
    redirect: Option<String>,
    index: Option<String>,
    return_: Option<(u16, Option<String>)>,
    methods: Option<Vec<Method>>,
    error_pages: HashMap<u16, String>,
    error_redirect: HashMap<u16, (Option<u16>, String)>,
    cgi: HashMap<String, PathBuf>,
    infos: HashMap<String, Vec<String>>,
    server: Option<Arc<Server>>,
}

impl Config for Location {
    fn path(&self) -> &PathBuf {
        &self.path
    }
    fn internal(&self) -> bool {
        self.internal | self.server.as_ref().unwrap().internal()
    }
    fn methods(&self) -> Option<&Vec<Method>> {
        self.methods.as_ref()
    }
    fn locations(&self) -> Option<&HashMap<PathBuf, Location>> {
        None
    }
    fn root(&self) -> Option<&PathBuf> {
        self.root.as_ref()
    }
    fn auto_index(&self) -> bool {
        self.auto_index
            .unwrap_or(self.server.as_ref().unwrap().auto_index())
    }
	fn alias(&self) -> Option<&PathBuf> {
		self.alias.as_ref()
	}
    fn cgi(&self) -> &HashMap<String, PathBuf> {
        &self.cgi
    }
    fn error_pages(&self) -> &HashMap<u16, String> {
        &self.error_pages
    }
    fn error_redirect(&self) -> &HashMap<u16, (Option<u16>, String)> {
        &self.error_redirect
    }
    fn max_body_size(&self) -> Option<&u64> {
        self.max_body_size.as_ref()
    }
    fn name(&self) -> Option<&Vec<String>> {
        None
    }
    fn index(&self) -> Option<&String> {
        self.index.as_ref()
    }
    fn port(&self) -> Option<&u16> {
        None
    }
    fn is_location(&self) -> bool {
        true
    }
    fn return_(&self) -> Option<&(u16, Option<String>)> {
        if let Some(n) = self.return_.as_ref() {
            Some(n)
        } else {
            self.server.as_ref().unwrap().return_()
        }
    }
}

#[allow(dead_code)]
impl Location {
    pub(super) fn new(location: LocationBlock, server: &Server) -> Result<Self, String> {
        let mut new_location = Location {
            path: PathBuf::from(location.path),
            exact_path: (location.modifier == Some("=".to_owned())),
            error_pages: HashMap::new(),
            error_redirect: HashMap::new(),
            max_body_size: None,
            return_: None,
            root: None,
            alias: None,
            index: None,
            methods: None,
            redirect: None,
            auto_index: None,
            internal: false,
            infos: HashMap::new(),
            cgi: HashMap::new(),
            server: None,
        };

        for (name, infos) in location.directives {
            match name.as_str() {
                "root" => {
                    // ROOT
                    if new_location.root.is_some() {
						return Err(format!(
							"invalid field: root: root cannot be set with alias"
						));
					}

					let root = parsing::extract_root(infos);
                    match root {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(path) => new_location.root = Some(path),
                    }
                }
				"alias" => {
					if new_location.root.is_some() {
						return Err(format!(
							"invalid field: alias: alias cannot be set with root"
						));
					} else {
						new_location.alias = Some(parsing::extract_alias(infos)?)
					}
				}
                "index" => {
                    let index = parsing::extract_index(infos);
                    match index {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(index) => new_location.index = Some(index),
                    }
                }
                "auto_index" => {
                    let auto_index = parsing::extract_auto_index(infos);
                    match auto_index {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(is_true) => new_location.auto_index = Some(is_true),
                    }
                }
                "client_max_body_size" => {
                    let max_body_size = parsing::extract_max_body_size(infos);
                    match max_body_size {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(max_size) => new_location.max_body_size = Some(max_size),
                    }
                }
                "cgi" => {
                    let (extension, path) = match parsing::extract_cgi(infos) {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(cgi) => cgi,
                    };
                    new_location.cgi.insert(extension, path);
                }
                "allowed_methods" => {
                    if infos.len() < 1 {
                        return Err(format!(
                            "location ({}) : invalid field: allowed_methods",
                            new_location.path.display()
                        ));
                    }
                    if new_location.methods.is_none() {
                        new_location.methods = Some(Vec::new())
                    }

                    new_location.methods.as_mut().unwrap().append(
                        &mut infos
                            .iter()
                            .map(|method| Method::from(&method[..]))
                            .collect(),
                    );
                }
                "redirect" => {
                    if infos.len() != 1 {
                        return Err(format!(
                            "location ({}) : invalid field: redirect",
                            new_location.path.display()
                        ));
                    }
                    new_location.redirect = Some(infos[0].clone());
                }
                "return" => {
                    new_location.return_ = match parsing::extract_return(infos) {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(res) => Some(res),
                    }
                }
                "internal" => {
                    new_location.internal = true;
                }
                "error_page" => {
                    let (pages, redirect) = parsing::extract_error_page(infos)?;
                    let hash = &mut new_location.error_pages;
                    if pages.is_some() {
                        pages
                            .unwrap()
                            .iter()
                            .map(|(code, url)| hash.insert(code.to_owned(), url.to_owned()))
                            .last();
                    }
                    let hash = &mut new_location.error_redirect;
                    if redirect.is_some() {
                        redirect
                            .unwrap()
                            .iter()
                            .map(|(code, url)| hash.insert(code.to_owned(), url.to_owned()))
                            .last();
                    }
                }
                _ => {
                    new_location.infos.insert(name, infos);
                }
            }
        }

        new_location.complete_with_server_directives(server);

        Ok(new_location)
    }

    fn complete_with_server_directives(&mut self, server: &Server) {
        self.internal = self.internal || server.internal();
        self.auto_index = self.auto_index.or(Some(server.auto_index()));

        if self.root.is_none() && server.root().is_some() {
            self.root = Some(server.root().unwrap().clone());
        }
        if self.index.is_none() && server.index().is_some() {
            self.index = Some(server.index().unwrap().clone());
        }
        if self.max_body_size.is_none() && server.max_body_size().is_some() {
            self.max_body_size = Some(server.max_body_size().unwrap().clone());
        }
        if self.methods.is_none() && server.methods().is_some() {
            self.methods = Some(server.methods().unwrap().clone());
        }
        if self.return_.is_none() && server.return_().is_some() {
            self.return_ = Some(server.return_().unwrap().clone());
        }

        if self.cgi.is_empty() && !server.cgi().is_empty() {
            self.cgi = server.cgi().clone();
        }
        if self.error_pages.is_empty() && !server.error_pages().is_empty() {
            self.error_pages = server.error_pages().clone();
        }
        if self.error_redirect.is_empty() && !server.error_redirect().is_empty() {
            self.error_redirect = server.error_redirect().clone();
        }
    }

    pub fn add_server_ref(&mut self, serv: Arc<Server>) {
        self.server = Some(serv);
    }

    pub fn find(&self, name: String) -> Option<&Vec<String>> {
        self.infos.get(&name)
    }
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
	
	pub fn exact_path(&self) -> bool {
		self.exact_path
	}
}
