pub mod executor;
pub mod reactor;
pub mod server;
pub mod task;

use executor::Executor;
use server::handle_connection;
use server::run_server;
use task::Task;

// async fn proccess() {
//     let task = Task::new(reactor, cfd);
//     task.await;
// }

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

    // ex.run_events_loop();

    ex.block_on(async move {
        let task = Task::new(reactor, cfd);
        task.await;
    });

    // let h1 = ex.spawn(async { task1().await });
    // let h2 = ex.spawn(async { task2().await });
    
    // ex.block_on(async {
    //     let r1 = h1.await;
    //     let r2 = h2.await;
    //     (r1, r2)
    // });

    log::debug!("main is finishing");

    Ok(())
}
