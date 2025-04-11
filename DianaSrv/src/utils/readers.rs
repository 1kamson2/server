use serde::de::DeserializeOwned;
use std::fs::File;
use std::io;
use std::io::{BufReader, prelude::*};
use std::path::Path;
use toml;

pub fn read_file_to_str(file_path: &Path) -> Result<String, io::Error> {
    let file_state = File::open(file_path);
    let file_content = match file_state {
        Ok(file) => file,
        Err(error) => return Err(error),
    };
    let mut buf = BufReader::new(file_content);
    let mut contents = String::new();
    let did_finish = buf.read_to_string(&mut contents);
    match did_finish {
        Ok(_) => return Ok(contents),
        Err(error) => return Err(error),
    };
}

pub fn read_toml<T: DeserializeOwned>(file_path: &Path) -> Result<T, io::Error> {
    let data = read_file_to_str(&file_path).unwrap();
    let toml = toml::from_str(&data).unwrap();
    return Ok(toml);
}
