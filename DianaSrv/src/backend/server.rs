use crate::utils::readers;
use regex::Regex;
use serde::Deserialize;
use std::{error::Error, fmt, path::Path};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Deserialize)]
struct Server {
    ip: String,
    port: u16,
    full_addr: String,
    max_connected_hosts: u32,
    cur_connected_hosts: u32,
    timeout_in_secs: u32,
}

impl Server {
    #[tokio::main]
    pub async fn new(toml_config: &Path) -> Result<Self, io::Error> {
        println!("[INFO] Attempting to create an instance.");
        let cfg: Server = readers::read_toml(&toml_config)?;

        cfg.full_addr = format!("{0}:{1}", cfg.ip, cfg.port);
        /*
         * TODO: Validation of this data
         * */
        return Ok(cfg);
    }

    #[tokio::main]
    pub async fn run(&mut self) {
        let listener = TcpListener::bind(&self.full_addr).await.unwrap();
        println!("[INFO] Server is running on {0}\n\n", self.addr);
        loop {
            let (inc_stream, inc_addr) = listener.accept().await.unwrap();
            tokio::spawn(async move { self.conn_handler(inc_stream, inc_addr).await });
        }
    }

    async fn conn_handler(&mut self, mut inc_stream: TcpStream, mut inc_addr: SocketAddr) {
        /* Function that answers user's request.*/
        let content = "aaaaa";
        let sz = content.len();
        let response = format!(
            "{404}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {sz}\r\n\r\n{content}"
        );
        println!("Sending the following response:\n{}", response);
        stream.write_all(response.as_bytes()).await.unwrap();
    }
}
