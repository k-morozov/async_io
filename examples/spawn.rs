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

    log::info!("Hello, world!");

    let sfd = run_server(4243);
    let cfd1 = handle_connection(sfd);
    let cfd2 = handle_connection(sfd);

    let mut ex = Executor::new();
    let reactor = ex.reactor();

    ex.start();

    let r = reactor.clone();
    let h1 = ex.spawn(async move {
        log::info!("msg 1");
        Task::new(r, cfd1).await;
    });

    log::info!("h1 was spawned");

    let r = reactor.clone();
    let h2 = ex.spawn(async move {
        log::info!("msg 2");
        Task::new(r, cfd2).await;
    });

    log::info!("h2 was spawned");

    log::info!("wait h1");
    h1.wait_result();

    log::info!("wait h2");
    h2.wait_result();

    log::info!("main is finishing");

    Ok(())
}
