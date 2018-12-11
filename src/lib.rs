#![allow(dead_code)]
extern crate crossbeam;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_rustls;
extern crate rustls;
extern crate tokio;
extern crate tokio_tcp;
extern crate webpki;
extern crate webpki_roots;

mod client;
mod connectors;
mod dns;
mod events;
