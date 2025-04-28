use diana_srv::backend::server::Server;
use diana_srv::utils::configs::server::config_toml;
use std::env;
use std::path::Path;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cfg_path: &String = &args[1];
    let cfg: &Path = config_toml(cfg_path);
    let srv = Arc::new(match Server::new(cfg) {
        Ok(srv_instance) => srv_instance,
        Err(_) => panic!("[ERROR] Server couldn't be created"),
    });
    srv.run();
}
/*
* TODO:
* Validate data in the TOML config file.
* Fix the reading of the file: [server] or missing value should be handled.
* Workout the proper responses.
*/
