use std::sync::Arc;
use std::time::Duration;

use async_io::runtime::Runtime;
use async_io::server::handle_connection;
use async_io::server::run_server;
use async_io::task;
// use async_io::task::read::Read;
// use async_io::task::suspend::Suspend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new()
        .env()
        .with_threads(true)
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let sfd = run_server(4243);

    let rt = Arc::new(Runtime::new());
    let reactor = rt.reactor();

    rt.start();

    let inner_r = reactor.clone();
    let inner_rt = rt.clone();

    rt.block_on(async move {
        let mut handlers = Vec::new();
        loop {
            let cfd: i32 = handle_connection(sfd);
            if -1 == cfd {
                log::debug!("call suspend.");
                std::thread::sleep(Duration::from_secs(4));
                let res = task::suspend().await;
                log::debug!("return to loop");
                continue;
            }
            let inner_r = inner_r.clone();

            let h1 = inner_rt.spawn(async move {
                let result = task::read(inner_r, cfd, 4).await;
                let result = String::from_utf8_lossy(&result[..]).to_string();
                log::debug!("step was finished, buf: {:?}.", result);
                let code = unsafe { libc::close(cfd) };
                log::debug!("close socket {cfd} wit code {code}");
            });

            handlers.push(h1);
            task::suspend().await;
        }

        handlers.iter().for_each(|h| {
            h.wait_result();
        });
    });

    log::info!("main is finishing");

    Ok(())
}
