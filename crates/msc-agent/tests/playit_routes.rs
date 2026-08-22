//! P9.7's public-boundary check.  Route mounting follows in P9.13; this
//! test pins the already-frozen contract's most important safety rule now:
//! Playit addresses are player addresses and cannot be loopback management
//! addresses.

use msc_domain::networking::safe_player_address;

#[test]
fn playit_connection_details_cannot_be_a_management_address() {
    assert_eq!(safe_player_address("127.0.0.1", Some(3000)), None);
    assert_eq!(safe_player_address("localhost", Some(3000)), None);
    assert_eq!(
        safe_player_address("join.example.joinmc.link", None).as_deref(),
        Some("join.example.joinmc.link")
    );
}
