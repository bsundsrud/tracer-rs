extern crate futures;
extern crate tokio_io;
extern crate tokio_reactor;
extern crate tokio_tcp;
extern crate tokio_timer;
extern crate tower_service;
#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
extern crate futures_cpupool;
extern crate http;

mod dns;
mod events;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
