use governor::clock::{Clock, DefaultClock};
use governor::state::keyed::DashMapStateStore;
use governor::{NotUntil, Quota, RateLimiter};
use std::borrow::Cow;
use std::net::{IpAddr, Ipv6Addr};

type RateLimiterInner = RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock>;

#[derive(Debug)]
pub(crate) struct ClientRateLimiter(RateLimiterInner);

/// Truncates a v6 address to a /64 subnet
fn normalise_ip_addr(addr: &IpAddr) -> Cow<IpAddr> {
    match addr {
        IpAddr::V4(_) => Cow::Borrowed(addr),
        IpAddr::V6(inner) => {
            let v6_64 = inner.segments();
            Cow::Owned(IpAddr::V6(Ipv6Addr::new(
                v6_64[0], v6_64[1], v6_64[2], v6_64[3], 0, 0, 0, 0,
            )))
        }
    }
}

impl ClientRateLimiter {
    pub(crate) fn new(quota: Quota) -> Self {
        Self(governor::RateLimiter::dashmap(quota))
    }

    /// Check if an `IpAddr` should be allowed to complete another request.
    ///
    /// Uses V4 addresses directly, and truncates V6 addresses to their /64 subnet (as creating more
    /// valid IPv6 addresses is trivial for a malicious client)
    pub(crate) fn check_key(
        &self,
        key: &IpAddr,
    ) -> Result<(), NotUntil<'_, <DefaultClock as Clock>::Instant>> {
        self.0.check_key(&normalise_ip_addr(key))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn truncates_ipv6() {
        let ip = IpAddr::V6("2001:470:6bd2::41:1".parse().unwrap());
        let truncated = normalise_ip_addr(&ip);
        assert_eq!(*truncated, IpAddr::V6("2001:470:6bd2::".parse().unwrap()));
    }
}
