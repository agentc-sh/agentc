// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::client::policy::AddressFilter;

/// Permits only public unicast addresses.
///
/// Loopback, private, link-local, unique-local, unspecified, broadcast, and documentation
/// addresses are refused unless individually permitted.
#[derive(Default)]
pub struct PublicAddressFilter {
    loopback: bool,
    private: bool,
    link_local: bool,
}

impl PublicAddressFilter {
    fn permits_v4(&self, address: Ipv4Addr) -> bool {
        if address.is_unspecified() || address.is_broadcast() || Self::is_documentation(address) {
            return false;
        }

        if address.is_loopback() {
            return self.loopback;
        }

        if address.is_private() {
            return self.private;
        }

        if address.is_link_local() {
            return self.link_local;
        }

        true
    }

    fn permits_v6(&self, address: Ipv6Addr) -> bool {
        if address.is_unspecified() {
            return false;
        }

        if address.is_loopback() {
            return self.loopback;
        }

        // `is_unique_local` and `is_unicast_link_local` are unstable, so the prefixes are matched
        // directly: fc00::/7 for unique local and fe80::/10 for link local.
        match address.segments()[0] {
            segment if segment & 0xfe00 == 0xfc00 => self.private,
            segment if segment & 0xffc0 == 0xfe80 => self.link_local,
            _ => true,
        }
    }

    // `Ipv4Addr::is_documentation` is unstable, so the three reserved ranges are matched here:
    // 192.0.2.0/24, 198.51.100.0/24, and 203.0.113.0/24.
    fn is_documentation(address: Ipv4Addr) -> bool {
        matches!(address.octets(), [192, 0, 2, _] | [198, 51, 100, _] | [203, 0, 113, _])
    }

    /// Additionally permits loopback addresses.
    pub fn allow_loopback(mut self) -> Self {
        self.loopback = true;
        self
    }

    /// Additionally permits private addresses.
    pub fn allow_private(mut self) -> Self {
        self.private = true;
        self
    }

    /// Additionally permits link-local addresses, including the cloud metadata address.
    pub fn allow_link_local(mut self) -> Self {
        self.link_local = true;
        self
    }
}

impl AddressFilter for PublicAddressFilter {
    fn name(&self) -> &'static str {
        "public-addresses"
    }

    fn allows(&self, _host: &str, address: SocketAddr) -> bool {
        match address.ip() {
            IpAddr::V4(address) => self.permits_v4(address),
            IpAddr::V6(address) => self.permits_v6(address),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(value: &str) -> SocketAddr {
        format!("{value}:443")
            .parse()
            .expect("test address parses")
    }

    #[test]
    fn public_addresses_are_permitted() {
        assert!(PublicAddressFilter::default().allows("example.com", address("93.184.216.34")));
    }

    #[test]
    fn loopback_and_private_addresses_are_refused() {
        let filter = PublicAddressFilter::default();

        assert!(!filter.allows("example.com", address("127.0.0.1")));
        assert!(!filter.allows("example.com", address("10.0.0.1")));
        assert!(!filter.allows("example.com", address("192.168.1.1")));
    }

    #[test]
    fn the_cloud_metadata_address_is_refused() {
        assert!(!PublicAddressFilter::default().allows("example.com", address("169.254.169.254")));
    }

    #[test]
    fn refused_ranges_can_be_permitted_individually() {
        assert!(
            PublicAddressFilter::default()
                .allow_loopback()
                .allows("localhost", address("127.0.0.1"))
        );
    }
}
