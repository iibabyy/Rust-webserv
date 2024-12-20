use std::path::PathBuf;

pub mod response;

pub struct Body {
    string: Option<String>,
    path: Option<PathBuf>,
}

impl Body {
    pub fn from_string(content: String) -> Body {
        Body {
            string: Some(content),
            path: None,
        }
    }

    pub fn from_path(path: PathBuf) -> Body {
        Body {
            string: None,
            path: Some(path),
        }
    }
}
