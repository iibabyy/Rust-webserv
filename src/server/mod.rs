pub mod location;
pub mod server;
pub mod traits;

/*------------------------------------------------------------*/
/*-------------------[ Config Parsing ]-----------------------*/
/*------------------------------------------------------------*/

pub mod parsing {
    use std::{collections::HashMap, path::PathBuf};

    use nom::InputIter;

    pub fn extract_root(value: Vec<String>) -> Result<PathBuf, String> {
        if value.len() != 1 {
            return Err("invalid field: root".to_owned());
        }

        let path = PathBuf::from(&value[0]);
        if path.is_dir() == false {
            return Err(value[0].clone() + ": invalid root directory");
        }

        Ok(path)
    }

    pub fn extract_alias(value: Vec<String>) -> Result<PathBuf, String> {
        if value.len() != 1 {
            return Err("invalid field: root".to_owned());
        }

        let path = PathBuf::from(&value[0]);

        if path.to_str().unwrap().iter_elements().last() != Some('/') {
            return Err(value[0].clone() + ": alias must ends with '/'");
        }

        Ok(path)
    }

    pub fn extract_upload_folder(value: Vec<String>) -> Result<PathBuf, String> {
        if value.len() != 1 {
            return Err("invalid field: upload_folder".to_owned());
        }

        let path = PathBuf::from(&value[0]);
        if path.is_dir() == false {
            return Err(format!("invalid upload folder: {}", value[0]));
        }

        Ok(path)
    }

    pub fn extract_max_body_size(value: Vec<String>) -> Result<usize, String> {
        if value.len() != 1 {
            return Err("invalid field: client_max_body_size".to_owned());
        }

        let num = value[0].parse::<usize>();

        return match num {
            Ok(num) => Ok(num),
            Err(e) => Err(format!("invalid field: client_max_body_size: {e}")),
        };
    }

    pub fn extract_error_page(
        value: Vec<String>,
    ) -> Result<
        (
            Option<HashMap<u16, String>>,
            Option<HashMap<u16, (Option<u16>, String)>>,
        ),
        String,
    > {
        if value.is_empty() {
            return Err(format!("invalid field: error_page: empty"));
        }

        let mut pages = HashMap::new();
        let mut redirect = HashMap::new();

        let mut it = value.iter();
        while let Some(str) = it.next() {
            let code = match str.parse::<u16>() {
                Ok(num) => num,
                Err(e) => return Err(format!("invalid field: error_page: {str}: {e}")),
            };

            let str = match it.next() {
                Some(str) => str,
                None => {
                    return Err(format!(
                        "invalid field: error_page: {} have no corresponding page",
                        code
                    ))
                }
            };

            if str.starts_with("=") {
                let redirect_code = if str.len() > 1 {
                    match str.as_str()[1..].parse::<u16>() {
                        Ok(num) => Some(num),
                        Err(e) => return Err(format!("invalid field: error_page: {str}: {e}")),
                    }
                } else {
                    None
                };

                let str = match it.next() {
                    Some(str) => str,
                    None => {
                        return Err(format!(
                            "invalid field: error_page: {} have no corresponding redirect",
                            code
                        ))
                    }
                };

                let url = str.to_owned();

                redirect.insert(code, (redirect_code, url));
            } else {
                pages.insert(code, str.clone());
            }
        }

        Ok((
            if pages.is_empty() { None } else { Some(pages) },
            if redirect.is_empty() {
                None
            } else {
                Some(redirect)
            },
        ))
    }

    pub fn extract_return(value: Vec<String>) -> Result<(u16, Option<String>), String> {
        if value.len() < 1 || value.len() > 2 {
            return Err("invalid field: return".to_owned());
        }

        let status_code = match value[0].parse::<u16>() {
            Ok(num) => num,
            Err(e) => return Err(format!("invalid field: return: {e}")),
        };

        let url = if value.len() == 2 {
            match is_redirect_status_code(status_code) {
                true => Some(value[1].clone()),
                false => {
                    println!(
                        "'return' field: not redirect code, url ignored ({status_code} {})",
                        value[1]
                    );
                    None
                }
            }
        } else {
            None
        };

        Ok((status_code, url))
    }

    pub fn extract_listen(value: Vec<String>) -> Result<(Option<u16>, bool), String> {
        if value.len() < 1 || value.len() > 2 {
            return Err("invalid field: port".to_owned());
        }

        let default = value.len() == 2 && value[1] == "default";

        let port = value[0].parse::<u16>();

        return match port {
            Ok(num) => Ok((Some(num), default)),
            Err(err) => Err(format!("invalid field: port: {}", err)),
        };
    }

    pub fn extract_index(value: Vec<String>) -> Result<String, String> {
        if value.len() != 1 {
            return Err("invalid field: index".to_owned());
        }

        Ok(value[0].clone())
    }

    pub fn extract_auto_index(value: Vec<String>) -> Result<bool, String> {
        if value.len() != 1 {
            return Err("invalid field: auto_index".to_owned());
        }

        match &value[0][..] {
            "on" => Ok(true),
            "off" => Ok(false),
            _ => Err(format!(
                "invalid field: auto_index: expected 'on' or 'off', found {}",
                value[0]
            )),
        }
    }

    pub fn extract_cgi(value: Vec<String>) -> Result<(String, PathBuf), String> {
        if value.len() != 2 {
            return Err("invalid field: cgi".to_owned());
        }

        let extension = value[0].clone();
        let extension = extension.trim_start_matches(".").to_string();
        let path = PathBuf::from(&value[1]);

        if path.is_file() == false {
            return Err(format!("invalid field: cgi: invalid path: {}", value[1]));
        }
        Ok((extension, path))
    }

    pub fn is_redirect_status_code(code: u16) -> bool {
        code == 301 || code == 302 || code == 303 || code == 307
    }
}
