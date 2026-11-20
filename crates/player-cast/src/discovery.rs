//! User-triggered, bounded discovery of local Google Cast receivers.
use mdns_sd::{ScopedIp, ServiceDaemon, ServiceEvent};
use std::{
    collections::BTreeSet,
    io,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

const CAST_SERVICE: &str = "_googlecast._tcp.local.";
const CAST_PORT: u16 = 8009;
const MAX_RECEIVERS: usize = 32;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DiscoveredReceiver {
    pub name: String,
    pub address: SocketAddr,
}

/// Browse the local network for Cast receivers for at most 15 seconds.
/// This never inspects a certificate or creates a trusted connection.
pub fn discover_receivers(timeout: Duration) -> io::Result<Vec<DiscoveredReceiver>> {
    discover_with(timeout, MdnsBrowser::new)
}

trait Browser {
    fn next(&mut self, timeout: Duration) -> io::Result<BrowserPoll>;
    fn close(&mut self);
}

enum BrowserPoll {
    Receiver(DiscoveredReceiver),
    Pending,
    TimedOut,
}

struct MdnsBrowser {
    daemon: ServiceDaemon,
    events: mdns_sd::Receiver<ServiceEvent>,
}

impl MdnsBrowser {
    fn new() -> io::Result<Self> {
        let daemon = ServiceDaemon::new().map_err(|_| io::Error::other("mDNS is unavailable"))?;
        let events = daemon
            .browse(CAST_SERVICE)
            .map_err(|_| io::Error::other("Could not start Cast receiver discovery"))?;
        Ok(Self { daemon, events })
    }
}

impl Browser for MdnsBrowser {
    fn next(&mut self, timeout: Duration) -> io::Result<BrowserPoll> {
        match self.events.recv_timeout(timeout) {
            Ok(event) => Ok(poll_service_event(event)),
            Err(mdns_sd::RecvTimeoutError::Timeout) => Ok(BrowserPoll::TimedOut),
            Err(mdns_sd::RecvTimeoutError::Disconnected) => {
                Err(io::Error::other("Cast receiver discovery stopped"))
            }
        }
    }

    fn close(&mut self) {
        let _ = self.daemon.stop_browse(CAST_SERVICE);
        let _ = self.daemon.shutdown();
    }
}

fn poll_service_event(event: ServiceEvent) -> BrowserPoll {
    match event {
        ServiceEvent::ServiceResolved(service) if service.is_valid() => receiver_from_resolved(
            &service.fullname,
            service.is_valid(),
            service.port,
            service.addresses.iter().map(ScopedIp::to_ip_addr),
        )
        .map(BrowserPoll::Receiver)
        .unwrap_or(BrowserPoll::Pending),
        _ => BrowserPoll::Pending,
    }
}

fn discover_with<B: Browser>(
    timeout: Duration,
    create: impl FnOnce() -> io::Result<B>,
) -> io::Result<Vec<DiscoveredReceiver>> {
    if timeout.is_zero() || timeout > Duration::from_secs(15) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Discovery timeout must be between zero and 15 seconds",
        ));
    }
    let mut browser = create()?;
    let result = collect_receivers(&mut browser, timeout);
    browser.close();
    result
}

fn collect_receivers(
    browser: &mut impl Browser,
    timeout: Duration,
) -> io::Result<Vec<DiscoveredReceiver>> {
    let deadline = Instant::now() + timeout;
    let mut results = BTreeSet::new();
    while results.len() < MAX_RECEIVERS {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match browser.next(remaining)? {
            BrowserPoll::Receiver(receiver) => {
                results.insert(receiver);
            }
            BrowserPoll::Pending => continue,
            BrowserPoll::TimedOut => break,
        }
    }
    Ok(results.into_iter().collect())
}

fn receiver_from_service(
    name: &str,
    is_valid: bool,
    port: u16,
    addresses: impl IntoIterator<Item = IpAddr>,
) -> Option<DiscoveredReceiver> {
    if !is_valid {
        return None;
    }
    receiver_from_records(name, port, addresses)
}

fn receiver_from_resolved(
    fullname: &str,
    is_valid: bool,
    port: u16,
    addresses: impl IntoIterator<Item = IpAddr>,
) -> Option<DiscoveredReceiver> {
    let name = mdns_sd::instance_name_unescaped(fullname);
    receiver_from_service(&name, is_valid, port, addresses)
}

fn receiver_from_records(
    name: &str,
    port: u16,
    addresses: impl IntoIterator<Item = IpAddr>,
) -> Option<DiscoveredReceiver> {
    if port != CAST_PORT {
        return None;
    }
    let address = addresses
        .into_iter()
        .filter(|ip| match ip {
            IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
            IpAddr::V6(ip) => ip.is_unique_local() || ip.is_loopback(),
        })
        .min()?;
    let name = name
        .chars()
        .filter(|ch| !ch.is_control())
        .take(80)
        .collect::<String>();
    if name.trim().is_empty() {
        return None;
    }
    Some(DiscoveredReceiver {
        name,
        address: SocketAddr::new(address, port),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Browser, BrowserPoll, DiscoveredReceiver, MdnsBrowser, discover_with, poll_service_event,
        receiver_from_records, receiver_from_resolved, receiver_from_service,
    };
    use std::{
        cell::Cell,
        collections::VecDeque,
        io,
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
        rc::Rc,
        time::Duration,
    };

    struct FakeBrowser {
        events: VecDeque<io::Result<BrowserPoll>>,
        closed: Rc<Cell<bool>>,
    }

    impl Browser for FakeBrowser {
        fn next(&mut self, _: Duration) -> io::Result<BrowserPoll> {
            self.events.pop_front().unwrap_or(Ok(BrowserPoll::TimedOut))
        }

        fn close(&mut self) {
            self.closed.set(true);
        }
    }

    fn fake(
        events: impl IntoIterator<Item = io::Result<BrowserPoll>>,
        closed: Rc<Cell<bool>>,
    ) -> FakeBrowser {
        FakeBrowser {
            events: events.into_iter().collect(),
            closed,
        }
    }

    #[test]
    fn accepts_only_local_cast_receivers_and_sanitizes_names() {
        let receiver = receiver_from_records(
            " Living\n Room ",
            8009,
            [
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
                IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            ],
        )
        .unwrap();
        assert_eq!(receiver.name, " Living Room ");
        assert_eq!(receiver.address.to_string(), "192.168.1.20:8009");
        assert!(receiver_from_records("wrong", 1234, [receiver.address.ip()]).is_none());
        assert!(receiver_from_records("empty", 8009, []).is_none());
        assert!(receiver_from_records("\n", 8009, [IpAddr::V4(Ipv4Addr::LOCALHOST)]).is_none());
        assert!(
            receiver_from_records(
                "public",
                8009,
                [IpAddr::V6(
                    "2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap()
                )]
            )
            .is_none()
        );
        assert!(
            receiver_from_service(
                "unresolved",
                false,
                8009,
                [IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))],
            )
            .is_none()
        );
        assert!(
            receiver_from_service(
                "resolved",
                true,
                8009,
                [IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))],
            )
            .is_some()
        );
    }

    #[test]
    fn accepts_local_address_ranges_and_caps_receiver_names() {
        let local_addresses = [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::new(169, 254, 10, 20)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V6("fd00::20".parse().unwrap()),
        ];
        for address in local_addresses {
            assert!(receiver_from_records("Local receiver", 8009, [address]).is_some());
        }

        let receiver = receiver_from_records(
            &"R".repeat(100),
            8009,
            [IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))],
        )
        .unwrap();
        assert_eq!(receiver.name.len(), 80);
    }

    #[test]
    fn resolves_service_instance_names_before_displaying_receivers() {
        let receiver = receiver_from_resolved(
            r"Living\.Room._googlecast._tcp.local.",
            true,
            8009,
            [IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))],
        )
        .unwrap();
        assert_eq!(receiver.name, "Living.Room");
    }

    #[test]
    fn timeout_bounds_and_factory_failures_are_reported() {
        let mut created = false;
        assert!(
            discover_with(Duration::ZERO, || {
                created = true;
                Ok(fake([], Rc::new(Cell::new(false))))
            })
            .is_err()
        );
        assert!(
            discover_with(Duration::from_secs(16), || {
                Ok(fake([], Rc::new(Cell::new(false))))
            })
            .is_err()
        );
        assert!(!created);
        assert!(
            discover_with::<FakeBrowser>(Duration::from_secs(1), || {
                Err(io::Error::other("create failed"))
            })
            .is_err()
        );
    }

    #[test]
    fn public_discovery_rejects_a_zero_timeout_before_starting_mdns() {
        assert_eq!(
            super::discover_receivers(Duration::ZERO)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn returns_unique_receivers_and_closes_the_browser() {
        let receiver = DiscoveredReceiver {
            name: "Receiver".into(),
            address: SocketAddr::from(([192, 168, 1, 20], 8009)),
        };
        let closed = Rc::new(Cell::new(false));
        let in_factory = closed.clone();
        let result = discover_with(Duration::from_millis(100), || {
            Ok(fake(
                [
                    Ok(BrowserPoll::Receiver(receiver.clone())),
                    Ok(BrowserPoll::Receiver(receiver.clone())),
                    Ok(BrowserPoll::TimedOut),
                ],
                in_factory,
            ))
        })
        .unwrap();
        assert_eq!(result, [receiver]);
        assert!(closed.get());
    }

    #[test]
    fn continues_after_search_started_until_a_receiver_is_resolved() {
        let receiver = DiscoveredReceiver {
            name: "Receiver".into(),
            address: SocketAddr::from(([192, 168, 1, 20], 8009)),
        };
        let closed = Rc::new(Cell::new(false));
        let in_factory = closed.clone();
        let search_started = poll_service_event(mdns_sd::ServiceEvent::SearchStarted(
            "_googlecast._tcp.local.".into(),
        ));
        let result = discover_with(Duration::from_millis(100), || {
            Ok(fake(
                [
                    Ok(search_started),
                    Ok(BrowserPoll::Receiver(receiver.clone())),
                    Ok(BrowserPoll::TimedOut),
                ],
                in_factory,
            ))
        })
        .unwrap();
        assert_eq!(result, [receiver]);
        assert!(closed.get());
    }

    #[test]
    fn closes_browser_when_receiving_fails() {
        let closed = Rc::new(Cell::new(false));
        let in_factory = closed.clone();
        assert!(
            discover_with(Duration::from_secs(1), || {
                Ok(fake([Err(io::Error::other("receive failed"))], in_factory))
            })
            .is_err()
        );
        assert!(closed.get());
    }

    #[test]
    fn stops_at_the_receiver_limit() {
        let events = (1..=40).map(|last| {
            Ok(BrowserPoll::Receiver(DiscoveredReceiver {
                name: format!("Receiver {last}"),
                address: SocketAddr::from(([10, 0, 0, last], 8009)),
            }))
        });
        let mut browser = fake(events, Rc::new(Cell::new(false)));
        assert_eq!(
            super::collect_receivers(&mut browser, Duration::from_secs(1))
                .unwrap()
                .len(),
            32
        );
        assert!(!browser.events.is_empty());
    }

    #[test]
    fn mdns_adapter_starts_and_shuts_down_when_the_host_allows_discovery() {
        if let Ok(mut browser) = MdnsBrowser::new() {
            assert!(browser.next(Duration::from_millis(50)).is_ok());
            browser.close();
        }
    }
}
