
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

extern crate serde;
extern crate serde_yaml;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;


#[derive(Debug, Deserialize, Clone)]
pub struct ConfigEntry {
    pub id: String,
    pub when: String,
    pub code: String,
    pub trigger: Option<String>, // Only used for 'keyboard' and 'term' types
}

#[derive(Debug, Deserialize)]
struct Config {
    config: Vec<ConfigEntry>,
}

fn read_yaml_file<T>(file_path: T) -> Result<Config, Box<dyn Error>>
where
    T: AsRef<Path>,
{
    let mut file = File::open(file_path)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;

    let data: Config = serde_yaml::from_str(&file_content)?;

    Ok(data)
}

pub fn parse_yaml_to_dict(file_path: &str) -> Result<HashMap<String, ConfigEntry>, Box<dyn Error>> {
    let config = read_yaml_file(file_path)?;
    let mut config_dict: HashMap<String, ConfigEntry> = HashMap::new();

    for entry in config.config {
        config_dict.insert(entry.id.clone(), entry);
    }

    Ok(config_dict)
}

