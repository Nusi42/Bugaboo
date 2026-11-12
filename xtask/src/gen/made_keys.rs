//! The messages that the words of the enums make (`derived_key!` in `bb-core`): no code writes
//! these keys as a string. TXT-CATALOG counts them as used, and the generator of the message
//! parts puts them on the first page, because the server sends them by key and any page may show
//! them.

use bb_core::ErrorCode;
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::agent_work::{AgentWorkState, BriefPart};
use bb_core::ai::{AuthorKind, Tool};
use bb_core::fields::Field;
use bb_core::holidays::{Holiday, Region as HolidayRegion};
use bb_core::issue::{IssueIcon, Priority, StatusGroup};
use bb_core::many_changes::{ManyChangeStatus, SkipReason};
use bb_core::message::MessageKey;
use bb_core::notification::NotificationKind;
use bb_core::own_fields::{ChoiceColor, FieldFor, FieldType, Look};
use bb_core::people_kinds::PeopleKind;
use bb_core::plan::warning::{AWAY_NAMELESS, DELAY_OF_GOAL, WarningKind, WayOut};
use bb_core::readable_id::IdMode;
use bb_core::time::entry::StarterField;
use bb_core::working_time::Preset;
use bb_policy::{Permission, StartingRole, Topic};

/// The messages of the words of agent work: where the work is, and what a brief lacks.
fn agent_work_keys() -> Vec<Made> {
    let states = AgentWorkState::ALL.iter().map(|state| Made {
        key: state.name(),
        word: format!("the state of agent work `{}`", state.code()),
    });
    let lacks = BriefPart::ALL.iter().map(|lack| Made {
        key: lack.name(),
        word: format!("the part of a brief `{}`", lack.code()),
    });
    states.chain(lacks).collect()
}

/// The messages of the words of public holidays: the regions that can be chosen and the days.
fn holiday_keys() -> Vec<Made> {
    let regions = HolidayRegion::ALL.iter().map(|region| Made {
        key: region.name(),
        word: format!("the region of public holidays `{}`", region.code()),
    });
    let days = Holiday::ALL.iter().map(|holiday| Made {
        key: holiday.name(),
        word: format!("the public holiday `{}`", holiday.code()),
    });
    regions.chain(days).collect()
}

/// The messages of the plan of the timeline: the sentence of each kind of warning, the two other
/// sentences, and the ways out.
fn plan_keys() -> Vec<Made> {
    let kinds = WarningKind::ALL.iter().map(|kind| Made {
        key: kind.sentence(),
        word: format!("the plan warning `{}`", kind.code()),
    });
    let others = [AWAY_NAMELESS, DELAY_OF_GOAL].into_iter().map(|key| Made {
        key,
        word: format!("the sentence of a plan warning `{key}`"),
    });
    let ways = WayOut::ALL.iter().map(|way| Made {
        key: way.label(),
        word: format!("the way out of a plan warning `{}`", way.code()),
    });
    kinds.chain(others).chain(ways).collect()
}

/// A message that the word of an enum makes, and the word.
pub(crate) struct Made {
    pub(crate) key: MessageKey,
    pub(crate) word: String,
}

/// Every message that the words of the enums make (`derived_key!` in `bb-core`): the code
/// does not write these keys, so the check finds them here.
pub(crate) fn made_keys() -> Vec<Made> {
    fn made(kind: &str, word: &str, key: MessageKey) -> Made {
        Made {
            key,
            word: format!("{kind} `{word}`"),
        }
    }
    let mut keys = Vec::new();
    for code in ErrorCode::ALL {
        keys.push(made("the error code", code.code(), code.message()));
    }
    for permission in Permission::ALL {
        keys.push(made(
            "the permission",
            permission.code(),
            permission.sentence(),
        ));
    }
    for topic in Topic::ALL {
        keys.push(made("the topic", topic.code(), topic.name()));
    }
    for role in StartingRole::ALL {
        keys.push(made("the starting role", role.code(), role.name()));
    }
    for kind in PeopleKind::ALL {
        keys.push(made(
            "the kind of people information",
            kind.code(),
            kind.name(),
        ));
    }
    for tool in Tool::ALL {
        keys.push(made("the AI tool", tool.code(), tool.description()));
    }
    for author in AuthorKind::ALL {
        keys.push(made("the kind of author", author.code(), author.label()));
    }
    for field in Field::ALL {
        keys.push(made("the field", field.code(), field.name()));
    }
    keys.extend(agent_work_keys());
    for group in StatusGroup::ALL {
        keys.push(made("the status group", group.code(), group.name()));
    }
    for priority in Priority::ALL {
        keys.push(made("the priority", priority.code(), priority.name()));
    }
    for icon in IssueIcon::ALL {
        keys.push(made("the icon of an issue type", icon.code(), icon.name()));
    }
    for mode in IdMode::ALL {
        keys.push(made("the mode of issue IDs", mode.code(), mode.name()));
    }
    for preset in Preset::ALL {
        keys.push(made(
            "the preset of the working-time rules",
            preset.code(),
            preset.name(),
        ));
    }
    for level in FieldLevel::ALL {
        keys.push(made("the field level", level.code(), level.name()));
    }
    for kind in FieldType::ALL {
        keys.push(made("the example of the kind", kind.code(), kind.example()));
        keys.push(made("the kind of an own field", kind.code(), kind.name()));
    }
    for what in FieldFor::ALL {
        keys.push(made("what an own field is for", what.code(), what.name()));
    }
    for starter in StarterField::ALL {
        keys.push(made("the starter field", starter.code(), starter.name()));
    }
    for color in ChoiceColor::ALL {
        keys.push(made("the color of a choice", color.code(), color.name()));
    }
    for look in Look::ALL {
        keys.push(made("the look of an own field", look.code(), look.name()));
    }
    for level in PeopleLevel::ALL {
        keys.push(made("the people level", level.code(), level.name()));
    }
    for reason in SkipReason::ALL {
        keys.push(made(
            "why an issue is left alone",
            reason.code(),
            reason.name(),
        ));
    }
    for status in ManyChangeStatus::ALL {
        keys.push(made(
            "the status of a run of many changes",
            status.code(),
            status.name(),
        ));
    }
    for kind in NotificationKind::ALL {
        keys.push(made("the kind of notification", kind.code(), kind.label()));
    }
    keys.extend(holiday_keys().into_iter().chain(plan_keys()));
    keys
}
