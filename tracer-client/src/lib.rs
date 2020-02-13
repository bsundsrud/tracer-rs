pub mod client;
pub mod connectors;
pub mod dns;

pub use crate::client::Client;
use std::future::Future;
use std::pin::Pin;
pub(crate) type FutureResponse<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;
