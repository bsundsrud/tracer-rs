use failure::Fail;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::time::{Duration, Instant};
use tracer_client::{Event, EventSet};

#[derive(Debug)]
pub struct Timing {
    events: EventSet,
    pub initiated: Instant,
    pub dns_resolution: Option<Duration>,
    pub connection: Option<Duration>,
    pub tls_negotiation: Option<Duration>,
    pub headers: Option<Duration>,
    pub full_response: Option<Duration>,
}

#[derive(Debug, Fail)]
#[fail(display = "Invalid Event Set: {}", _0)]
pub struct InvalidEventSetError(String);

impl Timing {
    pub fn from_events(ev: EventSet) -> Result<Timing, InvalidEventSetError> {
        let initiated = if let Some(i) = ev.initiated_at() {
            i
        } else {
            return Err(InvalidEventSetError("No Initiated event".into()));
        };
        let dns_resolution =
            ev.time_between(Event::DnsResolutionStarted, Event::DnsResolutionFinished);
        let connection = ev.time_between(Event::ConnectionStarted, Event::Connected);
        let tls_negotiation = ev.time_between(Event::TlsNegotiationStarted, Event::TlsNegotiated);
        let headers = ev.time_between(Event::Initiated, Event::HeadersReceived);
        let full_response = ev.time_between(Event::Initiated, Event::FullResponse);
        let timing = Timing {
            events: ev,
            initiated,
            dns_resolution,
            connection,
            tls_negotiation,
            headers,
            full_response,
        };
        Ok(timing)
    }
}

impl Display for Timing {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let mut d = f.debug_struct("Timing");
        if let Some(dr) = self.dns_resolution {
            d.field("dns_resolution", &dr);
        }
        if let Some(dr) = self.connection {
            d.field("connection", &dr);
        }
        if let Some(dr) = self.tls_negotiation {
            d.field("tls_negotiation", &dr);
        }
        if let Some(dr) = self.headers {
            d.field("headers", &dr);
        }
        if let Some(dr) = self.full_response {
            d.field("full_response", &dr);
        }
        d.finish()
    }
}
