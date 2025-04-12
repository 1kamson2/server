use crate::utils::readers;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{io, path::Path};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Deserialize)]
pub struct Server {
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
        let mut cfg: Server = readers::files::read_toml(toml_config)?;

        cfg.full_addr = format!("{0}:{1}", cfg.ip, cfg.port);
        return Ok(cfg);
    }

    #[tokio::main]
    pub async fn run(self: &Arc<Self>) {
        let listener = TcpListener::bind(&self.full_addr).await.unwrap();
        println!("[INFO] Server is running on {0}\n\n", self.full_addr);
        loop {
            let (inc_stream, inc_addr) = listener.accept().await.unwrap();
            let thread = Arc::clone(self);
            tokio::spawn(async move { thread.conn_handler(inc_stream, inc_addr).await });
        }
    }

    async fn conn_handler(&self, mut inc_stream: TcpStream, inc_addr: SocketAddr) {
        /* Function that answers user's request.*/
        let content = "aaaaa";
        let sz = content.len();
        let status = 404;
        let response = format!(
            "{status}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {sz}\r\n\r\n{content}"
        );
        println!("Sending the following response:\n{}", response);
        inc_stream.write_all(response.as_bytes()).await.unwrap();
    }
}
