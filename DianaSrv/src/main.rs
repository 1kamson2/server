use crate::backend::server::Server;

fn main() {
    let srv = Server::new();
    srv.run()
}
