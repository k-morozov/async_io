pub mod read;
pub mod suspend;

use std::sync::Arc;

use crate::reactor::Reactor;

pub trait CoroStep {
    fn execute(reactor: Arc<Reactor>, sock: i32, nbytes: usize) -> Self;
}
