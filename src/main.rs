pub mod executor;
pub mod handler;
pub mod reactor;
pub mod server;

use executor::Executor;
use handler::Handler;
use server::handle_connection;
use server::run_server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    log::debug!("Hello, world!");

    let sfd = run_server(4243);
    let cfd = handle_connection(sfd);

    let mut ex = Executor::new();
    let reactor = ex.reactor();

    ex.block_on(async {
        let f = Handler::new(reactor, cfd);
        f.await;
    });

    Ok(())
}
