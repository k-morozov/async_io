use async_io::executor::Executor;
use async_io::server::handle_connection;
use async_io::server::run_server;
use async_io::task::Task;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new()
        .env()
        .with_threads(true)
        .init()
        .unwrap();

    log::debug!("Hello, world!");

    let sfd = run_server(4243);
    let cfd = handle_connection(sfd);

    let mut ex = Executor::new();
    let reactor = ex.reactor();

    ex.start();

    ex.block_on(async move {
        Task::new(reactor, cfd).await;
    });

    // ex.block_on(async move {
    //     Task::new(reactor, cfd).await;
    // });

    log::debug!("main is finishing");

    Ok(())
}
