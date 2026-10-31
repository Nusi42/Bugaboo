//! The time zones the server knows, as a constant for the browser client (M2-F72): the names of
//! the zone data that `bb-core` carries (`bb_core::time::zone::names`, which xtask links with the
//! feature `zone-data`), so the browser offers exactly the zones the server accepts.

use super::TS_HEADER;

/// The browser client's list of zones.
pub(super) fn time_zones() -> String {
    let names = bb_core::time::zone::names()
        .iter()
        .map(|name| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{TS_HEADER}/** The time zones the server knows (M2-F72): the setting *Time zone* takes one of these or `device`. */
export const timeZones: readonly string[] = [{names}];
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m2_f72_the_browser_gets_every_zone_the_server_knows() {
        let text = time_zones();
        assert!(text.contains("\"Europe/Berlin\""));
        assert!(text.contains("\"UTC\""));
        assert!(!text.contains("Atlantis"));
    }
}
