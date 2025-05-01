use crate::utils::readers::buffers::constants::{
    CONTENT_LENGTH_FIELD, GET_REQUEST, POST_REQUEST, SITE_NOT_FOUND, SPACE,
};
use crate::utils::readers::buffers::{extract_number, find_in_buffer, read_tcpstream};
use crate::utils::readers::files::{check_if_file_exists, read_to_bytes, read_toml};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
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

#[derive(PartialEq, Debug)]
enum RequestType {
    /* Those specify how many positions to skip, not including whitespaces. */
    Get = 0,
    Post = 1,
    Invalid = -1,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThreadSharedState {
    /*
     *  Structure for safe thread sharing.
     *
     *  Attributes:
     *      cur_connected_hosts: The tracker of the number of concurrent hosts.
     *      This will be used for logic of disconnecting the users.
     *      cached_sites: Keeps recently visited sites for better and faster
     *      search results.
     *      resource_html_dir: Holds name of the resource directory in bytes.
     */
    #[serde(skip)]
    pub cur_connected_hosts: u32,
    #[serde(skip)]
    pub cached_sites: HashMap<Vec<u8>, Vec<u8>>,
    #[serde(skip)]
    pub resource_html_dir: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    /*
     *  The implementation of server instance, responsible for:
     *      - handling incoming connections,
     *      - validating, sanitizing user requests,
     *      - responding to the requests,
     *      - fetching correct sites.
     *
     *  Attributes:
     *      ip: Keeps host's ip, that is used to connect to this server.
     *      port: Keeps host's port, that will be used to connect to this server.
     *      max_connected_hosts: The maximum number of hosts (users) that
     *      can be connected at one time.h If the current number of hosts
     *      connected exceeds this number, the server will refuse further
     *      attempts of connections.
     *      timeout_in_secs: The maximum time for host connection if it
     *      doesn't respond
     *      shared_state: Structure that is needed for safe thread sharing.
     *
     */
    ip: String,
    port: u16,
    max_connected_hosts: u32,
    timeout_in_secs: u32,

    #[serde(skip)]
    shared_state: ThreadSharedState,
}

impl Server {
    #[tokio::main]
    pub async fn new(toml_config: &Path) -> Result<Self, io::Error> {
        /*
         *  Constructor of the server instance.
         *
         *  Arguments:
         *      toml_config: Path for the server's config, it must contain all
         *      attributes listed in the structure definition, except
         *      those marked with #[serde(skip)].
         *
         *  Returns:
         *      It returns Result<...> since the function might return
         *      the server instance or fail due to the incorrect configuration.
         */

        let mut cfg: Server = read_toml(toml_config)?;
        let mut ss: ThreadSharedState = ThreadSharedState {
            cur_connected_hosts: 0,
            cached_sites: HashMap::new(),
            resource_html_dir: vec![
                114, 101, 115, 111, 117, 114, 99, 101, 47, 104, 116, 109, 108, 47,
            ],
        };

        /* TODO: TEMP */
        ss.cached_sites
            .insert(SITE_NOT_FOUND.to_vec(), "Hello World".as_bytes().to_vec());

        cfg.shared_state = ss;

        return Ok(cfg);
    }

    #[tokio::main]
    pub async fn run(&mut self) {
        /*
         * The main function, that creates TCPListener based on the full address,
         * accepts incoming connections and moves it onto light threads.
         * The incoming streams and addresses are moved to the function,
         * that handles the connections.
         */

        /* Construct full address */
        let full_addr: String = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&full_addr).await.unwrap();
        loop {
            let (inc_stream, inc_addr) = listener.accept().await.unwrap();
            self.conn_handler(inc_stream, inc_addr).await;
        }
    }
    pub fn read_request_type(&self, buffer: &Vec<u8>) -> RequestType {
        /*
         *  Get the type of the request.
         *
         *  Parameters:
         *      buffer: Bytes of the stream, that was read into the vector.
         *
         *  Returns:
         *      It returns either GET or POST enum.
         */

        if buffer[0..3] == *GET_REQUEST {
            return RequestType::Get;
        }

        if buffer[0..4] == *POST_REQUEST {
            return RequestType::Post;
        }
        RequestType::Invalid
    }

    pub fn read_resource(&self, buffer: &Vec<u8>, req_type: &RequestType) -> Vec<u8> {
        /*
         *  Read what resource user requests.
         *
         *  Parameters:
         *      buffer: Bytes of the stream, that was read into the vector.
         *      req_type: Get the request type.
         *
         *  Returns:
         *      Resource in bytes.
         */

        /* Extract the number */
        // TODO: Write accessor to the values
        let request_offset: usize = match req_type {
            RequestType::Get => 3,
            RequestType::Post => 4,
            RequestType::Invalid => usize::MAX,
        };
        let mut vec_to_return: Vec<u8> = Vec::new();
        for byte in buffer[request_offset + 1..].iter() {
            if *byte == SPACE {
                return vec_to_return;
            }
            vec_to_return.push(*byte);
        }
        Vec::new()
    }

    pub fn fetch_resource(&mut self, resource_path: &Vec<u8>) -> &Vec<u8> {
        /*
         *  Fetch the data requested by user.
         *
         *  Parameters:
         *      resource_path: Resource path from the request.
         *
         *  Returns:
         *      The contents of the resource.
         */

        // TODO: Check all files beforehand
        // TODO: Add bad site handling, for now it returns nothing.
        if resource_path.is_empty() {
            // TODO: Change it to the welcome site later
            return &self.shared_state.cached_sites[SITE_NOT_FOUND];
        }

        if !self.shared_state.cached_sites.contains_key(resource_path) {
            let mut path_on_server: Vec<u8> = self.shared_state.resource_html_dir.clone();
            path_on_server.extend_from_slice(resource_path);

            let path: String = String::from(std::str::from_utf8(&path_on_server).unwrap());
            if !check_if_file_exists(&path) {
                return &self.shared_state.cached_sites[SITE_NOT_FOUND];
            }

            let site: Vec<u8> = read_to_bytes(Path::new(&path));

            /* Failed to read */
            if site.is_empty() {
                return &self.shared_state.cached_sites[SITE_NOT_FOUND];
            }
            /*
             * We can allow for to_vec, because loading will occurr
             * limited number of times
             */
            self.shared_state
                .cached_sites
                .insert(resource_path.to_vec(), site);
        }
        &self.shared_state.cached_sites[resource_path]
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
        buffer[(buffer_sz - body_length as usize)..].to_vec()
    }

    async fn conn_handler(&mut self, mut inc_stream: TcpStream, inc_addr: SocketAddr) {
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

        /* Try to read the body */
        let read_body_result: Vec<u8> = self.read_request_body(&vec_buf);
        if read_body_result.is_empty() {
            println!("[WARNING] Failed to read the body. Assume the handshake.");
            let site_content = self.fetch_resource(&read_body_result);
            let response: Vec<u8> = format_message(site_content);
            inc_stream.write_all(&response).await.unwrap();
            return;
        }

        /* Fetch the rest now, since body should be valid */
        /* Try to read the request type */
        let request_type: RequestType = self.read_request_type(&vec_buf);
        if request_type == RequestType::Invalid {
            println!("[ERROR] Invalid request type.");
            return;
        }

        /* Try to read the resource path */
        let resource_path: Vec<u8> = self.read_resource(&vec_buf, &request_type);
        if resource_path.is_empty() {
            println!("[ERROR] Failed to read the resource.");
            return;
        }

        let site_content: &Vec<u8> = self.fetch_resource(&resource_path);
        let response: Vec<u8> = format_message(site_content);
        inc_stream.write_all(&response).await.unwrap();
    }
}

pub fn format_message(site_content: &Vec<u8>) -> Vec<u8> {
    let sz = site_content.len();
    let status = 200;
    let mut response: Vec<u8> = format!(
        "HTTP/1.1 {status} OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {sz}\r\n\r\n"
    )
    .as_bytes()
    .to_vec();
    response.extend(site_content);
    return response;
}

mod tests {
    use std::env;

    use crate::backend::server::Server;
    use crate::utils;

    use super::*;
    const TEST_POST_REQUEST: &[u8] = b"POST /api/data HTTP/1.1\r\n\
            Host: example.com\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 27\r\n\
            \r\n\
            {\"key\":\"value\",\"number\":42}";
    const TEST_POST_RESOURCE: &[u8] = b"/api/data";

    fn server_init() -> Server {
        let cfg_name: String = String::from("resource/ServerConfig.toml");
        let cfg: &Path = utils::configs::server::config_toml(&cfg_name);
        Server::new(cfg).unwrap()
    }

    #[test]
    fn read_request_body_test() {
        let srv = server_init();
        let res = srv.read_request_body(&Vec::from(TEST_POST_REQUEST));
        let request_body: Vec<u8> = Vec::from(b"{\"key\":\"value\",\"number\":42}");
        assert_eq!(res, request_body);
    }

    #[test]
    fn read_request_type_test() {
        let srv = server_init();
        let test_req_as_buffer: Vec<u8> = Vec::from(TEST_POST_REQUEST);
        assert_eq!(
            srv.read_request_type(&test_req_as_buffer),
            RequestType::Post
        );
    }

    #[test]
    fn read_resource_test() {
        let srv = server_init();
        let test_req_as_buffer: Vec<u8> = Vec::from(TEST_POST_REQUEST);
        assert_eq!(
            srv.read_resource(&test_req_as_buffer, &RequestType::Post),
            Vec::from(TEST_POST_RESOURCE)
        );
    }
}
