pub mod executor;
pub mod task;
pub mod reactor;
pub mod server;

use executor::Executor;
use task::Task;
use server::handle_connection;
use server::run_server;

// async fn proccess() {
//     let task = Task::new(reactor, cfd);
//     task.await;
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    log::debug!("Hello, world!");

    let sfd = run_server(4243);
    let cfd = handle_connection(sfd);

    let mut ex = Executor::new();
    let reactor = ex.reactor();

    ex.run_events_loop();

    ex.block_on(async move {
        let task = Task::new(reactor, cfd);
        task.await;
    });

    Ok(())
}
