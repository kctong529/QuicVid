use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCandidate {
    pub interface_name: String,
    pub local_ip: Ipv4Addr,
}

pub fn discover_ipv4_candidates(active_ip: Ipv4Addr) -> anyhow::Result<Vec<PathCandidate>> {
    let interfaces = if_addrs::get_if_addrs()?;

    let discovered = interfaces.into_iter().filter_map(|interface| {
        let IpAddr::V4(local_ip) = interface.ip() else {
            return None;
        };

        Some(PathCandidate {
            interface_name: interface.name,
            local_ip,
        })
    });

    Ok(filter_ipv4_candidates(active_ip, discovered))
}

fn filter_ipv4_candidates(
    active_ip: Ipv4Addr,
    candidates: impl IntoIterator<Item = PathCandidate>,
) -> Vec<PathCandidate> {
    let mut candidates: Vec<_> = candidates
        .into_iter()
        .filter(|candidate| is_usable_alternative(active_ip, candidate.local_ip))
        .collect();

    candidates.sort_by(|left, right| {
        left.local_ip
            .octets()
            .cmp(&right.local_ip.octets())
            .then_with(|| left.interface_name.cmp(&right.interface_name))
    });

    candidates.dedup_by(|left, right| left.local_ip == right.local_ip);

    candidates
}

fn is_usable_alternative(active_ip: Ipv4Addr, candidate_ip: Ipv4Addr) -> bool {
    candidate_ip != active_ip
        && !candidate_ip.is_loopback()
        && !candidate_ip.is_unspecified()
        && !candidate_ip.is_broadcast()
        && !candidate_ip.is_multicast()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(interface_name: &str, local_ip: &str) -> PathCandidate {
        PathCandidate {
            interface_name: interface_name.to_string(),
            local_ip: local_ip.parse().unwrap(),
        }
    }

    #[test]
    fn excludes_current_active_address() {
        let candidates = filter_ipv4_candidates(
            "10.0.1.2".parse().unwrap(),
            [
                candidate("client-eth0", "10.0.1.2"),
                candidate("client-eth1", "10.0.2.2"),
            ],
        );

        assert_eq!(candidates, vec![candidate("client-eth1", "10.0.2.2")]);
    }

    #[test]
    fn excludes_loopback_and_unspecified_addresses() {
        let candidates = filter_ipv4_candidates(
            "10.0.1.2".parse().unwrap(),
            [
                candidate("lo", "127.0.0.1"),
                candidate("unknown", "0.0.0.0"),
                candidate("client-eth1", "10.0.2.2"),
            ],
        );

        assert_eq!(candidates, vec![candidate("client-eth1", "10.0.2.2")]);
    }

    #[test]
    fn excludes_broadcast_and_multicast_addresses() {
        let candidates = filter_ipv4_candidates(
            "10.0.1.2".parse().unwrap(),
            [
                candidate("broadcast", "255.255.255.255"),
                candidate("multicast", "224.0.0.1"),
                candidate("client-eth1", "10.0.2.2"),
            ],
        );

        assert_eq!(candidates, vec![candidate("client-eth1", "10.0.2.2")]);
    }

    #[test]
    fn sorts_candidates_deterministically_by_address() {
        let candidates = filter_ipv4_candidates(
            "10.0.1.2".parse().unwrap(),
            [
                candidate("eth2", "192.168.1.20"),
                candidate("client-eth1", "10.0.2.2"),
                candidate("eth3", "172.20.10.4"),
            ],
        );

        assert_eq!(
            candidates,
            vec![
                candidate("client-eth1", "10.0.2.2"),
                candidate("eth3", "172.20.10.4"),
                candidate("eth2", "192.168.1.20"),
            ]
        );
    }

    #[test]
    fn removes_duplicate_addresses() {
        let candidates = filter_ipv4_candidates(
            "10.0.1.2".parse().unwrap(),
            [
                candidate("eth1", "10.0.2.2"),
                candidate("eth1-alias", "10.0.2.2"),
            ],
        );

        assert_eq!(candidates.len(), 1);

        let expected: Ipv4Addr = "10.0.2.2".parse().unwrap();

        assert_eq!(candidates[0].local_ip, expected);
    }

    #[test]
    fn returns_empty_list_when_no_alternative_exists() {
        let candidates = filter_ipv4_candidates(
            "10.0.1.2".parse().unwrap(),
            [
                candidate("lo", "127.0.0.1"),
                candidate("client-eth0", "10.0.1.2"),
            ],
        );

        assert!(candidates.is_empty());
    }

    #[test]
    #[ignore]
    fn prints_discovered_local_candidates() {
        let candidates = discover_ipv4_candidates("10.0.1.2".parse().unwrap()).unwrap();

        println!("{candidates:#?}");
    }
}
