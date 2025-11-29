mod read;
mod suspend;

use std::sync::Arc;

use crate::reactor::Reactor;

use self::read::Read;
use self::suspend::Suspend;

pub fn read(
    reactor: Arc<Reactor>,
    cfd: i32,
    nbytes: usize,
) -> impl Future<Output = <Read as Future>::Output> {
    Read::new(reactor, cfd, nbytes)
}

pub fn suspend() -> impl Future<Output = <Suspend as Future>::Output> {
    Suspend::new()
}
