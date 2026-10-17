//! The settings of Bugaboo. Add a setting here and nowhere else (ADR-0018).

use super::themes::{
    DARK_THEME_CHOICES, DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, LIGHT_THEME_CHOICES,
};
use super::{
    Bool, ChangedBy, Choice, Format, Integer, Level, Setting, SettingDefinition, Text, settings,
};
use crate::mail::MailSecurity;
use crate::message;
use crate::readable_id::IdMode;

/// The choices of *Theme*: follow the device, or always light or always dark.
const MODE_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new("light", message::APPEARANCE_MODE_LIGHT),
    Choice::new("dark", message::APPEARANCE_MODE_DARK),
];

const SPACING_CHOICES: &[Choice] = &[
    Choice::new("comfortable", message::APPEARANCE_SPACING_COMFORTABLE),
    Choice::new("compact", message::APPEARANCE_SPACING_COMPACT),
];

const ISSUE_VIEW_CHOICES: &[Choice] = &[
    Choice::new("focus", message::APPEARANCE_ISSUE_VIEW_FOCUS),
    Choice::new("panel", message::APPEARANCE_ISSUE_VIEW_PANEL),
];

const DATE_FORMAT_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new(
        "day-month-year",
        message::SETTING_REGION_DATE_FORMAT_DAY_MONTH_YEAR,
    ),
    Choice::new(
        "month-day-year",
        message::SETTING_REGION_DATE_FORMAT_MONTH_DAY_YEAR,
    ),
    Choice::new(
        "year-month-day",
        message::SETTING_REGION_DATE_FORMAT_YEAR_MONTH_DAY,
    ),
];

const NUMBER_FORMAT_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new("point", message::SETTING_REGION_NUMBER_FORMAT_POINT),
    Choice::new("comma", message::SETTING_REGION_NUMBER_FORMAT_COMMA),
    Choice::new("space", message::SETTING_REGION_NUMBER_FORMAT_SPACE),
];

const FIRST_DAY_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new("monday", message::SETTING_REGION_FIRST_DAY_MONDAY),
    Choice::new("sunday", message::SETTING_REGION_FIRST_DAY_SUNDAY),
    Choice::new("saturday", message::SETTING_REGION_FIRST_DAY_SATURDAY),
];

/// The choices of *Issue IDs*: the two modes of ADR-0021. The registry is read as text by the tools
/// that make the constants of the browser, so a choice is written out; the assertions below stop
/// the build when it differs from the word of `IdMode`.
const ID_MODE_CHOICES: &[Choice] = &[
    Choice::new("random", IdMode::Random.name()),
    Choice::new("counting", IdMode::Counting.name()),
];

const _: () = {
    assert!(crate::words::same_text(IdMode::Random.code(), "random"));
    assert!(crate::words::same_text(IdMode::Counting.code(), "counting"));
};

/// The choices of *Protect the connection*. Written out like the modes of the issue IDs, and held
/// to the words of `MailSecurity` by the assertions below.
const MAIL_SECURITY_CHOICES: &[Choice] = &[
    Choice::new("starttls", message::SETTING_MAIL_SECURITY_STARTTLS),
    Choice::new("tls", message::SETTING_MAIL_SECURITY_TLS),
    Choice::new("off", message::SETTING_MAIL_SECURITY_OFF),
];

const _: () = {
    assert!(crate::words::same_text(
        MailSecurity::StartTls.code(),
        "starttls"
    ));
    assert!(crate::words::same_text(MailSecurity::Tls.code(), "tls"));
    assert!(crate::words::same_text(MailSecurity::Off.code(), "off"));
};

settings! {
    /// The theme choice: `light`, `dark`, or `device` for *Match my device*.
    APPEARANCE_MODE: Text = {
        key: "appearance.mode",
        name: message::THEME_LEGEND,
        default: "device",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_MODE_HELP,
        lockable: true,
        format: Format::Choices(MODE_CHOICES),
    }

    /// The light theme that is used on a light device (*Daylight* or *Paper*), or always when the
    /// mode is *Always light*.
    APPEARANCE_LIGHT_THEME: Text = {
        key: "appearance.light-theme",
        name: message::APPEARANCE_LIGHT,
        default: DEFAULT_LIGHT_THEME,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_LIGHT_THEME_HELP,
        lockable: true,
        format: Format::Choices(LIGHT_THEME_CHOICES),
    }

    /// The dark theme that is used on a dark device (*Dusk* or *Midnight*), or always when the
    /// mode is *Always dark*.
    APPEARANCE_DARK_THEME: Text = {
        key: "appearance.dark-theme",
        name: message::APPEARANCE_DARK,
        default: DEFAULT_DARK_THEME,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_DARK_THEME_HELP,
        lockable: true,
        format: Format::Choices(DARK_THEME_CHOICES),
    }

    /// How close together things are on screen: *Comfortable* or *Compact*.
    APPEARANCE_SPACING: Text = {
        key: "appearance.spacing",
        name: message::APPEARANCE_SPACING_LEGEND,
        default: "comfortable",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_SPACING_HELP,
        lockable: true,
        format: Format::Choices(SPACING_CHOICES),
    }

    /// *Less motion*: fewer moving effects, also when the device does not ask for it.
    APPEARANCE_LESS_MOTION: Bool = {
        key: "appearance.less-motion",
        name: message::APPEARANCE_LESS_MOTION,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_LESS_MOTION_HELP,
        lockable: true,
    }

    /// *Larger text*: all text a step bigger.
    APPEARANCE_LARGER_TEXT: Bool = {
        key: "appearance.larger-text",
        name: message::APPEARANCE_LARGER_TEXT,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_LARGER_TEXT_HELP,
        lockable: true,
    }

    /// Where an issue opens: in the focus view or in a side panel (UX §3.1).
    APPEARANCE_ISSUE_VIEW: Text = {
        key: "appearance.issue-view",
        name: message::SETTING_APPEARANCE_ISSUE_VIEW_NAME,
        default: "focus",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_APPEARANCE_ISSUE_VIEW_HELP,
        lockable: true,
        format: Format::Choices(ISSUE_VIEW_CHOICES),
    }

    /// The time zone of the times a person sees: `device`, or a name like `Europe/Berlin`.
    REGION_TIME_ZONE: Text = {
        key: "region.time-zone",
        name: message::SETTING_REGION_TIME_ZONE_NAME,
        default: "device",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_REGION_TIME_ZONE_HELP,
        lockable: false,
        format: Format::TimeZone,
    }

    /// How dates are written.
    REGION_DATE_FORMAT: Text = {
        key: "region.date-format",
        name: message::SETTING_REGION_DATE_FORMAT_NAME,
        default: "device",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_REGION_DATE_FORMAT_HELP,
        lockable: false,
        format: Format::Choices(DATE_FORMAT_CHOICES),
    }

    /// How big numbers and decimals are written.
    REGION_NUMBER_FORMAT: Text = {
        key: "region.number-format",
        name: message::SETTING_REGION_NUMBER_FORMAT_NAME,
        default: "device",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_REGION_NUMBER_FORMAT_HELP,
        lockable: false,
        format: Format::Choices(NUMBER_FORMAT_CHOICES),
    }

    /// The day a week starts on in calendars.
    REGION_FIRST_DAY: Text = {
        key: "region.first-day",
        name: message::SETTING_REGION_FIRST_DAY_NAME,
        default: "device",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_REGION_FIRST_DAY_HELP,
        lockable: false,
        format: Format::Choices(FIRST_DAY_CHOICES),
    }

    /// The page that opens after signing in, as an address of the app.
    START_PAGE: Text = {
        key: "start.page",
        name: message::SETTING_START_PAGE_NAME,
        default: "/",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_START_PAGE_HELP,
        lockable: false,
        format: Format::PageAddress,
    }

    /// The keys a person chose for shortcuts, as one JSON object from the ID of a shortcut to its
    /// keys. A shortcut that is not in it keeps its usual keys.
    SHORTCUTS_KEYS: Text = {
        key: "shortcuts.keys",
        name: message::SETTING_SHORTCUTS_KEYS_NAME,
        default: "{}",
        levels: [Person],
        changed_by: EachPerson,
        help: message::SETTING_SHORTCUTS_KEYS_HELP,
        lockable: false,
        format: Format::ShortcutKeys,
    }

    /// The size a person gave the window of each form, as one JSON object from the ID of a form to
    /// its width and height in pixels (UX §6.3). A form that is not in it opens at its usual size.
    WINDOW_SIZES: Text = {
        key: "window.sizes",
        name: message::SETTING_WINDOW_SIZES_NAME,
        default: "{}",
        levels: [Person],
        changed_by: EachPerson,
        help: message::SETTING_WINDOW_SIZES_HELP,
        lockable: false,
        format: Format::WindowSizes,
    }

    /// How many minutes without activity pass before people are signed out.
    SECURITY_SESSION_IDLE_MINUTES: Integer = {
        key: "security.session-idle-minutes",
        name: message::SETTING_SECURITY_SESSION_IDLE_MINUTES_NAME,
        default: 30,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_SESSION_IDLE_MINUTES_HELP,
        lockable: false,
        bounds: (5, 1440),
    }

    /// How many hours a sign-in can last at most, however busy the person is.
    SECURITY_SESSION_ABSOLUTE_HOURS: Integer = {
        key: "security.session-absolute-hours",
        name: message::SETTING_SECURITY_SESSION_ABSOLUTE_HOURS_NAME,
        default: 24,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_SESSION_ABSOLUTE_HOURS_HELP,
        lockable: false,
        bounds: (1, 720),
    }

    /// Whether people may make an account without an invitation (AUTH-06).
    AUTH_SIGN_UP_ENABLED: Bool = {
        key: "auth.sign-up-enabled",
        name: message::SETTING_AUTH_SIGN_UP_ENABLED_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_SIGN_UP_ENABLED_HELP,
        lockable: false,
    }

    /// The e-mail domains that may sign up on their own (a list separated by commas).
    AUTH_SIGN_UP_DOMAINS: Text = {
        key: "auth.sign-up-domains",
        name: message::SETTING_AUTH_SIGN_UP_DOMAINS_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_SIGN_UP_DOMAINS_HELP,
        lockable: false,
        list: (100, 253),
    }

    /// The hosts that images in text may come from (SEC-13, a list separated by commas). Images of
    /// this server are always shown; the list is empty at first, so no image comes from outside.
    CONTENT_IMAGE_HOSTS: Text = {
        key: "content.image-hosts",
        name: message::SETTING_CONTENT_IMAGE_HOSTS_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_CONTENT_IMAGE_HOSTS_HELP,
        lockable: false,
        list: (50, 253),
        format: Format::Hosts,
    }

    /// Whether everyone has to use two-step sign-in.
    SECURITY_TWO_STEP_REQUIRED: Bool = {
        key: "security.two-step-required",
        name: message::SETTING_SECURITY_TWO_STEP_REQUIRED_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_SECURITY_TWO_STEP_REQUIRED_HELP,
        lockable: false,
    }

    /// How many days a new access key lives when the person picks no other date.
    AUTH_ACCESS_KEY_DEFAULT_DAYS: Integer = {
        key: "auth.access-key-default-days",
        name: message::SETTING_AUTH_ACCESS_KEY_DEFAULT_DAYS_NAME,
        default: 90,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_ACCESS_KEY_DEFAULT_DAYS_HELP,
        lockable: false,
        bounds: (1, 365),
    }

    /// The most days an access key can live, whatever date the person picks.
    AUTH_ACCESS_KEY_LONGEST_DAYS: Integer = {
        key: "auth.access-key-longest-days",
        name: message::SETTING_AUTH_ACCESS_KEY_LONGEST_DAYS_NAME,
        default: 365,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AUTH_ACCESS_KEY_LONGEST_DAYS_HELP,
        lockable: false,
        bounds: (1, 365),
    }

    /// How a project gives out issue IDs: `random` (`WEB-K7Q2X`) or `counting` (`WEB-142`). The
    /// organization's value is the default of every project; a project can choose (ISS-01a,
    /// ADR-0021). Changing it gives every issue of the project a new ID and keeps the old ones.
    ISSUES_ID_MODE: Text = {
        key: "issues.id-mode",
        name: message::SETTING_ISSUES_ID_MODE_NAME,
        default: "random",
        levels: [Organization, Project],
        changed_by: ProjectLeads,
        help: message::SETTING_ISSUES_ID_MODE_HELP,
        lockable: false,
        format: Format::Choices(ID_MODE_CHOICES),
    }

    /// How many symbols a random issue ID has when it is drawn; the length grows by itself when
    /// the short ones run out.
    ISSUES_ID_LENGTH: Integer = {
        key: "issues.id-length",
        name: message::SETTING_ISSUES_ID_LENGTH_NAME,
        default: 5,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_ISSUES_ID_LENGTH_HELP,
        lockable: false,
        bounds: (4, 8),
    }

    /// Whether the person is told in the app when they are mentioned.
    NOTIFICATIONS_MENTIONED_IN_APP: Bool = {
        key: "notifications.mentioned-in-app",
        name: message::SETTING_NOTIFICATIONS_MENTIONED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_MENTIONED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when they are mentioned.
    NOTIFICATIONS_MENTIONED_BY_MAIL: Bool = {
        key: "notifications.mentioned-by-mail",
        name: message::SETTING_NOTIFICATIONS_MENTIONED_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_MENTIONED_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether the person is told in the app when an issue is assigned to them.
    NOTIFICATIONS_ASSIGNED_IN_APP: Bool = {
        key: "notifications.assigned-in-app",
        name: message::SETTING_NOTIFICATIONS_ASSIGNED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_ASSIGNED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when an issue is assigned to them.
    NOTIFICATIONS_ASSIGNED_BY_MAIL: Bool = {
        key: "notifications.assigned-by-mail",
        name: message::SETTING_NOTIFICATIONS_ASSIGNED_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_ASSIGNED_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether the person is told in the app when a status change, a comment or a file on an issue they follow.
    NOTIFICATIONS_FOLLOWED_ACTIVITY_IN_APP: Bool = {
        key: "notifications.followed-activity-in-app",
        name: message::SETTING_NOTIFICATIONS_FOLLOWED_ACTIVITY_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_FOLLOWED_ACTIVITY_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when a status change, a comment or a file on an issue they follow.
    NOTIFICATIONS_FOLLOWED_ACTIVITY_BY_MAIL: Bool = {
        key: "notifications.followed-activity-by-mail",
        name: message::SETTING_NOTIFICATIONS_FOLLOWED_ACTIVITY_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_FOLLOWED_ACTIVITY_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether the person is told in the app when other changes to fields of an issue they follow.
    NOTIFICATIONS_FOLLOWED_FIELDS_IN_APP: Bool = {
        key: "notifications.followed-fields-in-app",
        name: message::SETTING_NOTIFICATIONS_FOLLOWED_FIELDS_IN_APP_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_FOLLOWED_FIELDS_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when other changes to fields of an issue they follow.
    NOTIFICATIONS_FOLLOWED_FIELDS_BY_MAIL: Bool = {
        key: "notifications.followed-fields-by-mail",
        name: message::SETTING_NOTIFICATIONS_FOLLOWED_FIELDS_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_FOLLOWED_FIELDS_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether the person is told in the app when their run of many changes has ended.
    NOTIFICATIONS_RUN_ENDED_IN_APP: Bool = {
        key: "notifications.run-ended-in-app",
        name: message::SETTING_NOTIFICATIONS_RUN_ENDED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_RUN_ENDED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when their run of many changes has ended.
    NOTIFICATIONS_RUN_ENDED_BY_MAIL: Bool = {
        key: "notifications.run-ended-by-mail",
        name: message::SETTING_NOTIFICATIONS_RUN_ENDED_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_RUN_ENDED_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether the person is told in the app when somebody asks them to finish editing, or takes their editing over.
    NOTIFICATIONS_EDIT_NOTICE_IN_APP: Bool = {
        key: "notifications.edit-notice-in-app",
        name: message::SETTING_NOTIFICATIONS_EDIT_NOTICE_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_EDIT_NOTICE_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when somebody asks them to finish editing, or takes their editing over.
    NOTIFICATIONS_EDIT_NOTICE_BY_MAIL: Bool = {
        key: "notifications.edit-notice-by-mail",
        name: message::SETTING_NOTIFICATIONS_EDIT_NOTICE_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: message::SETTING_NOTIFICATIONS_EDIT_NOTICE_BY_MAIL_HELP,
        lockable: true,
    }

    /// Whether the organization sends e-mail at all (PRIV-08): off until an administrator has
    /// entered the mail server and switched it on.
    MAIL_ENABLED: Bool = {
        key: "mail.enabled",
        name: message::SETTING_MAIL_ENABLED_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_ENABLED_HELP,
        lockable: false,
    }

    /// The host name of the mail server Bugaboo sends e-mail through.
    MAIL_HOST: Text = {
        key: "mail.host",
        name: message::SETTING_MAIL_HOST_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_HOST_HELP,
        lockable: false,
        list: (1, 253),
        format: Format::Hosts,
    }

    /// The port of the mail server.
    MAIL_PORT: Integer = {
        key: "mail.port",
        name: message::SETTING_MAIL_PORT_NAME,
        default: 587,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_PORT_HELP,
        lockable: false,
        bounds: (1, 65535),
    }

    /// How the connection to the mail server is protected.
    MAIL_SECURITY: Text = {
        key: "mail.security",
        name: message::SETTING_MAIL_SECURITY_NAME,
        default: "starttls",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_SECURITY_HELP,
        lockable: false,
        format: Format::Choices(MAIL_SECURITY_CHOICES),
    }

    /// The user name Bugaboo signs in to the mail server with; empty for a server that asks for none.
    MAIL_USERNAME: Text = {
        key: "mail.username",
        name: message::SETTING_MAIL_USERNAME_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_USERNAME_HELP,
        lockable: false,
    }

    /// The address e-mails come from.
    MAIL_FROM: Text = {
        key: "mail.from",
        name: message::SETTING_MAIL_FROM_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_FROM_HELP,
        lockable: false,
    }

    /// *Short e-mails*: e-mails leave out all issue text (title and comments), for places where
    /// mail leaves the company (M4-Q6).
    MAIL_SHORT: Bool = {
        key: "mail.short",
        name: message::SETTING_MAIL_SHORT_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_MAIL_SHORT_HELP,
        lockable: false,
    }
}

/// Settings that exist only for tests, so the project level and the other rules of the layers can
/// be tested while no real setting is set on a project yet. They are not in [`ALL`]; they are in
/// [`registry`](super::registry) only in tests and with the feature `test-settings`, which only
/// `[dev-dependencies]` switch on.
#[cfg(any(test, feature = "test-settings"))]
pub mod for_tests {
    use super::settings;
    use super::{
        Bool, ChangedBy, Choice, Format, Integer, Level, Setting, SettingDefinition, Text,
    };
    use crate::message;

    const KIND_CHOICES: &[Choice] = &[
        Choice::new("none", message::APP_NAME),
        Choice::new("short", message::APP_NAME),
        Choice::new("long", message::APP_NAME),
    ];

    settings! {
        /// A text that project leads set on a project and people set for themselves; lockable. People
        /// change their own value, project leads the project's.
        TEST_PROJECT_NOTE: Text = {
            key: "test.project-note",
            name: message::APP_NAME,
            default: "none",
            levels: [Organization, Project, Person],
            changed_by: EachPerson,
            help: message::APP_NAME,
            lockable: true,
        }

        /// A bounded number on the organization and project levels that cannot be locked.
        TEST_PROJECT_LIMIT: Integer = {
            key: "test.project-limit",
            name: message::APP_NAME,
            default: 3,
            levels: [Organization, Project],
            changed_by: ProjectLeads,
            help: message::APP_NAME,
            lockable: false,
            bounds: (1, 9),
        }

        /// A choice that only administrators change, on the organization and project levels.
        TEST_ADMIN_CHOICE: Text = {
            key: "test.admin-choice",
            name: message::APP_NAME,
            default: "none",
            levels: [Organization, Project],
            changed_by: Administrators,
            help: message::APP_NAME,
            lockable: true,
            format: Format::Choices(KIND_CHOICES),
        }

        /// A setting that has a person level but that only administrators change: nobody sets it
        /// for themselves.
        TEST_ADMIN_PERSON: Text = {
            key: "test.admin-person",
            name: message::APP_NAME,
            default: "none",
            levels: [Organization, Person],
            changed_by: Administrators,
            help: message::APP_NAME,
            lockable: false,
        }

        /// A text that only exists for the organization.
        TEST_ORGANIZATION_ONLY: Text = {
            key: "test.organization-only",
            name: message::APP_NAME,
            default: "none",
            levels: [Organization],
            changed_by: Administrators,
            help: message::APP_NAME,
            lockable: false,
        }

        /// A switch with all three levels, for the tests of a person's own values.
        TEST_SWITCH: Bool = {
            key: "test.switch",
            name: message::APP_NAME,
            default: false,
            levels: [Organization, Project, Person],
            changed_by: EachPerson,
            help: message::APP_NAME,
            lockable: true,
        }
    }
}
