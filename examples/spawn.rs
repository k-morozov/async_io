use async_io::coro_step::CoroStep;
use async_io::coro_step::read::CoroStepRead;
use async_io::runtime::Runtime;
use async_io::server::handle_connection;
use async_io::server::run_server;

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

    let mut rt = Runtime::new();
    let reactor = rt.reactor();

    rt.start();

    let r = reactor.clone();
    let h1 = rt.spawn(async move {
        log::info!("coro: 1, step #1");

        let result = CoroStepRead::execute(r, cfd1, 4).await;

        let result = String::from_utf8_lossy(&result[..]).to_string();
        log::debug!("step was finished, buf: {:?}.", result);

        log::info!("coro: 1, step #2");
    });

    log::info!("h1 was spawned");

    let r = reactor.clone();
    let h2 = rt.spawn(async move {
        log::info!("coro: 2, step #1");

        let result = CoroStepRead::execute(r, cfd2, 4).await;

        let result = String::from_utf8_lossy(&result[..]).to_string();
        log::debug!("step was finished, buf: {:?}.", result);

        log::info!("coro: 2, step #2");
    });

    log::info!("h2 was spawned");

    log::info!("wait h1");
    h1.wait_result();

    log::info!("wait h2");
    h2.wait_result();

    log::info!("main is finishing");

    Ok(())
}
