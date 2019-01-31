use err_derive::Error;
use serde_yaml::Value;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(display = "Failed to read config file: {}", err)]
    ReadConfigFile { err: io::Error },
    #[error(display = "Failed to parse config: {}", err)]
    ParseConfig { err: serde_yaml::Error },
    #[error(
        display = "The structure of \"{}\" is not valid, please refer to:\n{}",
        path_str,
        template
    )]
    StructureValidation { path_str: String, template: String },
}

#[derive(Debug)]
pub struct Config {
    conf: Value,
}

impl Config {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(path: impl AsRef<Path>, template_src: &str) -> Result<Config, ConfigError> {
        let conf_src = match fs::read_to_string(&path) {
            Ok(conf_src) => conf_src,
            Err(err) => {
                return Err(ConfigError::ReadConfigFile { err });
            }
        };

        let conf = match serde_yaml::from_str(&conf_src) {
            Ok(conf) => conf,
            Err(err) => return Err(ConfigError::ParseConfig { err }),
        };

        let template = match serde_yaml::from_str(template_src) {
            Ok(template) => template,
            Err(err) => panic!("Template is invalid: {}", err),
        };

        if normalize_value(&conf) != normalize_value(&template) {
            Err(ConfigError::StructureValidation {
                path_str: path.as_ref().to_str().unwrap().into(),
                template: template_src.to_owned(),
            })
        } else {
            Ok(Config { conf })
        }
    }

    pub fn get(&self) -> &Value {
        &self.conf
    }
}

fn normalize_value(value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(_) => Value::Bool(false),
        Value::Number(_) => Value::Number(serde_yaml::Number::from(0)),
        Value::String(_) => Value::String(String::new()),
        Value::Sequence(seq) => {
            Value::Sequence(seq.iter().map(|val| normalize_value(val)).collect())
        }
        // In this case we only normalize the value on the right
        Value::Mapping(map) => Value::Mapping(
            map.iter()
                .map(|(val1, val2)| (val1.clone(), normalize_value(val2)))
                .collect(),
        ),
    }
}
