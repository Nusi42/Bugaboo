//! The tests of `notifications`.

use super::*;
use crate::checks::arch_settings::CATALOG_PATH;

const KINDS: &str = r#"{"components":{"schemas":{"NotificationKind":{"type":"string","enum":["mentioned","followed_activity"]}}}}"#;

const NOTIFICATION: &str = "impl NotificationKind {
    pub const fn setting(self, via: Via) -> &'static Setting<Bool> {
        use settings::{A, B, C, D};
        match (self, via) {
            (Self::Mentioned, Via::App) => &A,
            (Self::Mentioned, Via::Mail) => &B,
            (Self::FollowedActivity, Via::App) => &C,
            (Self::FollowedActivity, Via::Mail) => &D,
        }
    }
}
";

const CATALOG: &str = "settings! {
    A: Bool = { key: \"notifications.mentioned-in-app\", default: true, }
    B: Bool = { key: \"notifications.mentioned-by-mail\", default: true, }
    C: Bool = { key: \"notifications.followed-activity-in-app\", default: true, }
    D: Bool = { key: \"notifications.followed-activity-by-mail\", default: false, }
}
";

fn sources() -> Vec<Source> {
    vec![
        (
            NOTIFICATION_SOURCE.to_owned(),
            format!(
                "pub const KEEP_DAYS: i64 = 90;\npub const AT_ONCE_MAX: usize = 100;\n{NOTIFICATION}"
            ),
        ),
        (CATALOG_PATH.to_owned(), CATALOG.to_owned()),
        (
            MAIL_SOURCE.to_owned(),
            "pub const TEST_EMAIL_EVERY_SECONDS: i64 = 10;".to_owned(),
        ),
        (
            SIGN_UP_SOURCE.to_owned(),
            "pub const HOURS: i64 = 24;\npub const RESEND_AFTER_MINUTES: i64 = 10;".to_owned(),
        ),
        (
            HANDLE_SOURCE.to_owned(),
            "pub const CHANGE_EVERY_DAYS: i64 = 7;".to_owned(),
        ),
        (
            COMMENT_SOURCE.to_owned(),
            "pub const HANDLE_MAX: usize = 64;".to_owned(),
        ),
    ]
}

#[test]
fn the_table_of_customize_comes_from_the_match_of_the_notification_kind() {
    let text = notifications(&sources(), KINDS).unwrap();
    assert!(
        text.contains(
            "mentioned: { app: \"notifications.mentioned-in-app\", mail: \"notifications.mentioned-by-mail\" },"
        ),
        "{text}"
    );
    assert!(
        text.contains("followed_activity: { app: \"notifications.followed-activity-in-app\""),
        "{text}"
    );
}

#[test]
fn the_numbers_of_the_texts_come_from_the_constants_of_the_core() {
    let text = notifications(&sources(), KINDS).unwrap();
    assert!(text.contains("notificationKeepDays = 90;"), "{text}");
    assert!(text.contains("notificationsAtOnceMax = 100;"), "{text}");
    assert!(text.contains("signUpLinkHours = 24;"), "{text}");
    assert!(text.contains("signUpResendMinutes = 10;"), "{text}");
    assert!(text.contains("testEmailEverySeconds = 10;"), "{text}");
    assert!(text.contains("handleChangeEveryDays = 7;"), "{text}");
    assert!(text.contains("handleMax = 64;"), "{text}");
}

#[test]
fn a_kind_without_both_settings_or_a_missing_source_is_an_error() {
    let one_way = NOTIFICATION.replace("            (Self::Mentioned, Via::Mail) => &B,\n", "");
    let mut broken = sources();
    broken[0].1 =
        format!("pub const KEEP_DAYS: i64 = 90;\npub const AT_ONCE_MAX: usize = 100;\n{one_way}");
    assert!(notifications(&broken, KINDS).is_err());
    assert!(notifications(&sources(), r#"{"components":{"schemas":{}}}"#).is_err());
    assert!(notifications(&[], KINDS).is_err());
}

#[test]
fn a_word_is_spelled_as_the_variant_of_the_rust_enum() {
    assert_eq!(pascal_case("followed_activity"), "FollowedActivity");
    assert_eq!(pascal_case("run_ended"), "RunEnded");
    assert_eq!(pascal_case("mentioned"), "Mentioned");
}
