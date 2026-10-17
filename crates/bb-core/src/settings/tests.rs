//! The tests of the settings registry.

use std::collections::HashSet;

use super::*;
use crate::message::Message;

#[test]
fn auth_06_the_sign_up_domains_have_limits_that_refuse_and_cut() {
    let domains = AUTH_SIGN_UP_DOMAINS.definition();
    assert_eq!(domains.list_limits(), Some((100, 253)));
    let hundred: Vec<String> = (0..100).map(|n| format!("d{n}.test")).collect();
    assert!(domains.list_fits(&hundred.join(",")));
    assert!(domains.list_fits(""));
    assert!(!domains.list_fits(&format!("{},one-more.test", hundred.join(","))));
    assert!(!domains.list_fits(&format!("{}.test", "a".repeat(250))));
    // Reading cuts the list to what fits: the first items, without the ones that are too long.
    let long = format!("{}.test", "a".repeat(250));
    let stored = format!("first.test,{long},{},last.test", hundred.join(","));
    let cut = domains.cap_list(&stored);
    let items: Vec<&str> = cut.split(',').collect();
    assert_eq!(items.len(), 100);
    assert_eq!(items[0], "first.test");
    assert!(!cut.contains("aaaa"));
    // A setting without limits is left alone.
    assert!(APPEARANCE_MODE.definition().list_fits(&"x,".repeat(1000)));
    assert_eq!(APPEARANCE_MODE.definition().cap_list("a, b"), "a, b");
}

#[test]
fn sec_13_the_image_hosts_start_empty_and_are_a_list_the_administrators_change() {
    let hosts = CONTENT_IMAGE_HOSTS.definition();
    assert_eq!(hosts.key(), "content.image-hosts");
    assert_eq!(hosts.list_limits(), Some((50, 253)));
    assert!(hosts.list_fits(""));
    assert!(hosts.list_fits("images.example.com,cdn.example.com"));
    assert!(!hosts.list_fits(&format!("{}.test", "a".repeat(250))));
    assert_eq!(hosts.levels(), [Level::Organization]);
}

#[test]
fn auth_01_the_session_times_have_bounds_and_values_are_cut_to_them() {
    let idle = SECURITY_SESSION_IDLE_MINUTES.definition();
    let absolute = SECURITY_SESSION_ABSOLUTE_HOURS.definition();
    assert_eq!(idle.bounds(), Some((5, 1440)));
    assert_eq!(absolute.bounds(), Some((1, 720)));
    assert_eq!(
        [idle.clamp(0), idle.clamp(30), idle.clamp(10_000)],
        [5, 30, 1440]
    );
    assert_eq!(
        [
            absolute.clamp(-1),
            absolute.clamp(24),
            absolute.clamp(9_999)
        ],
        [1, 24, 720]
    );
    // The defaults are inside their own bounds, and a setting without bounds is left alone.
    assert_eq!(idle.clamp(30), 30);
    assert_eq!(APPEARANCE_MODE.definition().bounds(), None);
    assert_eq!(APPEARANCE_MODE.definition().clamp(-99), -99);
}

#[test]
fn keys_are_unique_topic_dot_name_words() {
    let mut seen = HashSet::new();
    for definition in registry() {
        assert!(
            seen.insert(definition.key()),
            "{} is declared twice",
            definition.key()
        );
        let (topic, name) = definition.key().split_once('.').unwrap();
        for part in [topic, name] {
            assert!(
                part.starts_with(|c: char| c.is_ascii_lowercase())
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{}",
                definition.key()
            );
        }
        assert_eq!(definition.topic(), topic);
    }
}

#[test]
fn every_setting_has_a_name_explains_itself_and_has_a_level() {
    for definition in registry() {
        for message in [definition.help(), definition.name()] {
            assert!(
                !Message::from(message).render().is_empty(),
                "{}",
                definition.key()
            );
        }
        assert!(!definition.levels().is_empty(), "{}", definition.key());
        assert!(
            definition.levels().windows(2).all(|pair| pair[0] < pair[1]),
            "{}: list the levels from organization down to person",
            definition.key()
        );
    }
}

#[test]
fn a_lockable_setting_has_a_lower_level_to_lock() {
    for definition in registry().filter(|definition| definition.lockable()) {
        assert!(
            definition.levels().len() > 1 && definition.levels()[0] == Level::Organization,
            "{}",
            definition.key()
        );
    }
}

#[test]
fn the_default_has_the_declared_type() {
    assert_eq!(APPEARANCE_MODE.default_value().as_deref(), Ok("device"));
    assert_eq!(SECURITY_SESSION_IDLE_MINUTES.default_value(), Ok(30));
    assert_eq!(SECURITY_TWO_STEP_REQUIRED.default_value(), Ok(false));
}

#[test]
fn cfg_04_every_default_is_a_value_the_setting_accepts_and_every_choice_has_words() {
    for definition in registry() {
        assert!(
            definition.accepts(&definition.default().to_value()),
            "{} does not accept its own default",
            definition.key()
        );
        for choice in definition.format().choices() {
            assert!(
                !Message::from(choice.label()).render().is_empty(),
                "{}: {}",
                definition.key(),
                choice.value()
            );
        }
    }
}

#[test]
fn cfg_04_a_value_is_accepted_by_type_bounds_list_and_format() {
    let text = |value: &str| SettingValue::Text(value.to_owned());
    let mode = APPEARANCE_MODE.definition();
    assert!(mode.accepts(&text("dark")));
    assert!(!mode.accepts(&text("Dusk")));
    assert!(!mode.accepts(&SettingValue::Bool(true)));
    assert!(mode.has_type_of(&text("anything")));
    assert!(!mode.has_type_of(&SettingValue::Integer(1)));
    let idle = SECURITY_SESSION_IDLE_MINUTES.definition();
    assert!(idle.accepts(&SettingValue::Integer(30)));
    assert!(!idle.accepts(&SettingValue::Integer(4)));
    assert!(!idle.accepts(&text("30")));
    assert!(
        SECURITY_TWO_STEP_REQUIRED
            .definition()
            .accepts(&SettingValue::Bool(false))
    );
    let domains = AUTH_SIGN_UP_DOMAINS.definition();
    assert!(domains.accepts(&text("a.test,b.test")));
    assert!(!domains.accepts(&text(&"x,".repeat(101))));
    assert_eq!(mode.format().choices().len(), 3);
}

#[test]
fn cfg_06_only_settings_of_signing_in_and_safety_touch_security() {
    assert!(SECURITY_TWO_STEP_REQUIRED.definition().touches_security());
    assert!(
        SECURITY_SESSION_IDLE_MINUTES
            .definition()
            .touches_security()
    );
    assert!(AUTH_SIGN_UP_ENABLED.definition().touches_security());
    assert!(!APPEARANCE_MODE.definition().touches_security());
    assert!(!SHORTCUTS_KEYS.definition().touches_security());
    assert!(!WINDOW_SIZES.definition().touches_security());
}

#[test]
fn cfg_04_the_test_settings_are_in_the_registry_only_for_tests_and_are_not_in_all() {
    assert!(find("test.project-note").is_some());
    assert!(
        ALL.iter()
            .all(|definition| !definition.key().starts_with("test."))
    );
}

#[test]
fn resolve_uses_the_stored_value_and_reports_one_of_the_wrong_type() {
    let stored = SettingValue::Integer(10);
    assert_eq!(SECURITY_SESSION_IDLE_MINUTES.resolve(Some(&stored)), Ok(10));
    assert_eq!(SECURITY_SESSION_IDLE_MINUTES.resolve(None), Ok(30));
    let wrong = SettingValue::Text("soon".to_owned());
    assert_eq!(
        SECURITY_SESSION_IDLE_MINUTES.resolve(Some(&wrong)),
        Err(WrongType {
            key: "security.session-idle-minutes"
        })
    );
}

#[test]
fn cfg_06_the_layers_give_a_setting_as_its_type_and_the_default_when_nothing_usable_is_stored() {
    let mut layers = Layers::default();
    // Nothing stored: the defaults of the registry, with no default written anywhere else.
    assert_eq!(layers.number(&SECURITY_LOG_KEEP_DAYS), 365);
    assert_eq!(layers.number(&LOGS_SERVER_LOG_FILES), 5);
    assert!(!layers.yes(&MAIL_ENABLED));
    assert_eq!(layers.text(&MAIL_HOST), "");
    // A stored value counts, cut to the bounds of the setting.
    layers.organization.insert(
        SECURITY_LOG_KEEP_DAYS.definition().key().to_owned(),
        OrganizationValue {
            value: SettingValue::Integer(5),
            locked: false,
        },
    );
    assert_eq!(
        layers.number(&SECURITY_LOG_KEEP_DAYS),
        30,
        "the least it may be"
    );
    // A value of another type, written by hand, counts as the default.
    layers.organization.insert(
        LOGS_SERVER_LOG_FILES.definition().key().to_owned(),
        OrganizationValue {
            value: SettingValue::Text("many".to_owned()),
            locked: false,
        },
    );
    assert_eq!(layers.number(&LOGS_SERVER_LOG_FILES), 5);
}

#[test]
fn find_looks_a_key_up_in_the_registry() {
    assert_eq!(find("appearance.mode"), Some(APPEARANCE_MODE.definition()));
    assert_eq!(find("appearance.nothing"), None);
}

#[test]
fn values_are_stored_as_plain_json() {
    assert_eq!(
        serde_json::to_string(&SettingValue::Bool(true)).unwrap(),
        "true"
    );
    assert_eq!(
        serde_json::from_str::<SettingValue>("30").unwrap(),
        SettingValue::Integer(30)
    );
    assert_eq!(
        serde_json::from_str::<SettingValue>("\"dusk\"").unwrap(),
        SettingValue::Text("dusk".to_owned())
    );
}
