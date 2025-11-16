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
    let cfd1 = handle_connection(sfd);
    // let cfd2 = handle_connection(sfd);

    let mut ex = Executor::new();
    let reactor = ex.reactor();

    ex.start();

    let r = reactor.clone();
    let h1 = ex.spawn(async move {
        log::debug!("msg 1");
        Task::new(r, cfd1).await;
    });

    log::debug!("step");

    // let r = reactor.clone();
    // let h2 = ex.spawn(async move {
    //     log::debug!("msg 2");
    //     Task::new(r, cfd2).await;
    // });

    h1.wait_result();
    // h2.wait_result();

    log::debug!("main is finishing");

    Ok(())
}
