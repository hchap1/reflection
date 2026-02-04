use std::net::{IpAddr, Ipv4Addr};

use crate::{communication::server::SERVICE_TYPE, error::Res, frontend::application::ApplicationError};
use mdns_sd::ServiceDaemon;

pub struct Client {

}

impl Client {

    pub fn discover() -> Res<Ipv4Addr> {
        let mdns = ServiceDaemon::new()?;
        let receiver = mdns.browse(SERVICE_TYPE)?;

        while let Ok(event) = receiver.recv() {
            match event {
                mdns_sd::ServiceEvent::ServiceResolved(service) => return service
                    .addresses
                    .into_iter()
                    .filter_map(
                        |scoped_ip|
                        if let IpAddr::V4(ipv4) = scoped_ip.to_ip_addr() {
                            Some(ipv4)
                        } else { None }
                    )
                    .next()
                    .ok_or(ApplicationError::NoEndpoint.into()),
                _ => continue
            }
        };

        Err(ApplicationError::NoEndpoint.into())
    }
}
