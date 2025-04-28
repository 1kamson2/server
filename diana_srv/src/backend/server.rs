use crate::utils::readers::buffers::constants::CONTENT_LENGTH_FIELD;
use crate::utils::readers::buffers::{extract_number, find_in_buffer, read_tcpstream};
use crate::utils::readers::files::read_toml;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{io, path::Path};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

enum HttpResponseStatus {
    Ok = 200,
    NoContent = 204,
    NotModified = 304,
    BadRequest = 400,
    Forbidden = 403,
    NotFound = 404,
    IamATeapot = 418,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    /*
     *  Main structure for Server implementation.
     *
     *  Attributes:
     *      ip: Keeps host's ip, that is used to connect to this server.
     *      port: Keeps host's port, that will be used to connect to this server.
     *      full_addr: Combines both ip and port, to make full address,
     *      that will allow user to connect and use the server.
     *      max_connected_hosts: The maximum number of hosts (users) that
     *      can be connected at one time.h If the current number of hosts
     *      connected exceeds this number, the server will refuse further
     *      attempts of connections.
     *      cur_connected_hosts: The tracker of the number of concurrent hosts.
     *      This will be used for logic of disconnecting the users.
     *      timeout_in_secs: The maximum time for host connection if it
     *      doesn't respond
     */
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
        /*
         *  Constructor of the server instance.
         *
         *  Arguments:
         *      toml_config: Path for the server's config, it must contain all
         *      attributes listed in the structure definition.
         *
         *  Returns:
         *      It returns Result<...> since the function might return
         *      the server instance or fail due to the incorrect configuration.
         */
        let mut cfg: Server = read_toml(toml_config)?;

        cfg.full_addr = format!("{0}:{1}", cfg.ip, cfg.port);
        return Ok(cfg);
    }

    #[tokio::main]
    pub async fn run(self: &Arc<Self>) {
        /*
         *  The main function, that creates TCPListener based on the full address,
         *  accepts incoming connections and moves it onto light threads.
         *  The incoming streams and addresses are moved to the function,
         *  that handles the connections.
         */
        let listener = TcpListener::bind(&self.full_addr).await.unwrap();
        loop {
            let (inc_stream, inc_addr) = listener.accept().await.unwrap();
            let thread = Arc::clone(self);
            tokio::spawn(async move { thread.conn_handler(inc_stream, inc_addr).await });
        }
    }

    pub fn read_request_body(&self, buffer: &Vec<u8>) -> Vec<u8> {
        /*
         *  Get the actual request body, by reading two consecutive \r\n sequences.
         *
         *  Parameters:
         *      buffer: Bytes of the stream, that was read into the vector.
         *
         *  Returns:
         *      Returns vector with body or empty vector that indicates
         *      the fail to read or might mean the handshake.
         */
        let pattern: &[u8] = CONTENT_LENGTH_FIELD;
        /* Find the position in the buffer of Content-Length field. */
        let content_field_idx: usize = find_in_buffer(buffer, pattern);
        if content_field_idx == usize::MAX {
            return Vec::new();
        }

        let offset_start: usize = pattern.len();
        /* Pass only the slice, since the function definition requires this */
        let body_length = extract_number(&buffer[content_field_idx + offset_start..]);

        /* Too big body */
        if body_length > 8192 {
            return Vec::new();
        }

        let buffer_sz: usize = buffer.len();
        // TODO: Very vulnerable, we assume that the content is valid.
        buffer[(body_length as usize - buffer_sz)..].to_vec()
    }

    async fn conn_handler(&self, mut inc_stream: TcpStream, inc_addr: SocketAddr) {
        /*
         *  Handles each incoming connection. It will read the incoming requests,
         *  create appropiate responses and send them out.
         *
         *  Arguments:
         *      inc_stream: Incoming stream from the host's request.
         *      inc_addr: The address, that the request comes from.
         */

        /* Make sure, that the incoming stream is readable */
        let _ = inc_stream.readable().await;

        /* Try to read the content, if fail exit earlier */
        let vec_buf: Vec<u8> = match read_tcpstream(&inc_stream) {
            Ok(vec) => vec,
            Err(e) => {
                println!("[ERROR] {e}");
                return;
            }
        };

        let read_result = self.read_request_body(&vec_buf);
        if read_result.is_empty() {
            println!("[WARNING] Failed to read the body. Assume the handshake.");
            // do something here
            return;
        }

        let content = "Hello World!";
        let sz = content.len();
        let status = 404;
        let response = format!(
            "HTTP/1.1 {status} BAD\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {sz}\r\n\r\n{content}"
        );
        inc_stream.write_all(response.as_bytes()).await.unwrap();
    }
}

mod tests {
    use std::env;

    use crate::backend::server::Server;
    use crate::utils;

    use super::*;
    fn server_init() -> Server {
        let args: Vec<String> = env::args().collect();
        let cfg_path: &String = &args[1];
        let cfg: &Path = utils::configs::server::config_toml(cfg_path);
        Server::new(cfg).unwrap()
    }

    fn read_request_body_test() {
        const TEST_REQUEST: &[u8] = b"POST /api/data HTTP/1.1\r\n\
            Host: example.com\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 27\r\n\
            \r\n\
            {\"key\":\"value\",\"number\":42}";
        let srv = server_init();
        let res = srv.read_request_body(&Vec::from(TEST_REQUEST));
        let request_body: Vec<u8> = Vec::from(b"{\"key\":\"value\",\"number\":42}");
        assert_eq!(res, request_body);
    }
}
