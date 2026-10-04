//! A short description of where somebody is signed in, for *Where you're signed in*.
//!
//! Only the browser and the system are kept, as plain names, never the full `User-Agent` text
//! (PRIV-01: it identifies a device more precisely than the list needs).

/// Browsers, most specific first: Edge and Opera also say "Chrome" in their text.
const BROWSERS: &[(&str, &str)] = &[
    ("Edg/", "Edge"),
    ("OPR/", "Opera"),
    ("Firefox/", "Firefox"),
    ("Chrome/", "Chrome"),
    ("Safari/", "Safari"),
];

/// Systems, most specific first: Android also says "Linux", and iPhone says "Mac OS".
const SYSTEMS: &[(&str, &str)] = &[
    ("Android", "Android"),
    ("iPhone", "iPhone"),
    ("iPad", "iPad"),
    ("Windows", "Windows"),
    ("Macintosh", "Mac"),
    ("CrOS", "ChromeOS"),
    ("Linux", "Linux"),
];

/// The browser and the system, as far as they are known.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Device {
    /// The name of the browser, like `Firefox`.
    pub browser: Option<&'static str>,
    /// The name of the system, like `Windows`.
    pub system: Option<&'static str>,
}

impl Device {
    /// What the `User-Agent` header says, reduced to two names.
    #[must_use]
    pub fn from_user_agent(text: &str) -> Self {
        let find = |table: &[(&'static str, &'static str)]| {
            table
                .iter()
                .find(|(marker, _)| text.contains(marker))
                .map(|(_, name)| *name)
        };
        Self {
            browser: find(BROWSERS),
            system: find(SYSTEMS),
        }
    }

    /// The form that is stored: `Firefox|Windows`, with an empty part for an unknown one.
    #[must_use]
    pub fn to_stored(&self) -> String {
        format!(
            "{}|{}",
            self.browser.unwrap_or_default(),
            self.system.unwrap_or_default()
        )
    }

    /// Reads the stored form again. Names this version does not know are dropped.
    #[must_use]
    pub fn from_stored(text: &str) -> Self {
        let (browser, system) = text.split_once('|').unwrap_or_default();
        let known = |table: &[(&'static str, &'static str)], wanted: &str| {
            table
                .iter()
                .find(|(_, name)| *name == wanted)
                .map(|(_, name)| *name)
        };
        Self {
            browser: known(BROWSERS, browser),
            system: known(SYSTEMS, system),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priv_01_only_the_names_of_browser_and_system_are_kept() {
        let firefox =
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:130.0) Gecko/20100101 Firefox/130.0";
        let edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0 Safari/537.36 Edg/129.0";
        let phone = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/129.0 Mobile Safari/537.36";
        assert_eq!(
            Device::from_user_agent(firefox).to_stored(),
            "Firefox|Windows"
        );
        assert_eq!(Device::from_user_agent(edge).to_stored(), "Edge|Windows");
        assert_eq!(Device::from_user_agent(phone).to_stored(), "Chrome|Android");
        assert_eq!(Device::from_user_agent("curl/8").to_stored(), "|");
    }

    #[test]
    fn priv_01_the_stored_form_reads_back_and_drops_what_it_does_not_know() {
        let device = Device::from_user_agent("Firefox/130.0 (Macintosh)");
        assert_eq!(Device::from_stored(&device.to_stored()), device);
        assert_eq!(Device::from_stored("Netscape|Amiga"), Device::default());
        assert_eq!(Device::from_stored("garbage"), Device::default());
    }
}
