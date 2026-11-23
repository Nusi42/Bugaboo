//! The index of the walk of a person's data (ADR-0044, decision 1, amendment *the index of the
//! walk*): for each table whose changes or JSON can name a person, the rows whose text holds the ID
//! of a person, by person. Each row of an index goes with the row it points to (`ON DELETE
//! CASCADE`).

use super::{Iden, StoredTable};

/// The rows of the events that name a person in their changes. The other indexes of one key take
/// its columns.
#[derive(Clone, Copy, Iden)]
pub(crate) enum EventNamesPerson {
    #[iden = "event_names_person"]
    Table,
    Row,
    Person,
}

/// The rows of the notifications that name a person in their changes.
#[derive(Clone, Copy, Iden)]
pub(crate) enum NotificationNamesPerson {
    #[iden = "notification_names_person"]
    Table,
}

/// The jobs that name a person in their payload.
#[derive(Clone, Copy, Iden)]
pub(crate) enum JobNamesPerson {
    #[iden = "job_names_person"]
    Table,
}

/// The runs of changes to many issues that name a person in their action.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ChangeRunNamesPerson {
    #[iden = "change_run_names_person"]
    Table,
}

/// The proposals of agents that name a person in their values.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProposalNamesPerson {
    #[iden = "agent_proposal_names_person"]
    Table,
}

/// The rows of the history of issues that name a person in their changes.
#[derive(Clone, Copy, Iden)]
pub(crate) enum HistoryNamesPerson {
    #[iden = "history_entry_names_person"]
    Table,
    Event,
    Issue,
    Person,
}

stored_tables! {
    EventNamesPerson => bb_core::walk_index::RowNamingPerson {
        Row => row,
        Person => person,
    },
    NotificationNamesPerson (EventNamesPerson) => bb_core::walk_index::RowNamingPerson {
        Row => row,
        Person => person,
    },
    JobNamesPerson (EventNamesPerson) => bb_core::walk_index::RowNamingPerson {
        Row => row,
        Person => person,
    },
    ChangeRunNamesPerson (EventNamesPerson) => bb_core::walk_index::RowNamingPerson {
        Row => row,
        Person => person,
    },
    ProposalNamesPerson (EventNamesPerson) => bb_core::walk_index::RowNamingPerson {
        Row => row,
        Person => person,
    },
    HistoryNamesPerson => bb_core::walk_index::HistoryRowNamingPerson {
        Event => event,
        Issue => issue,
        Person => person,
    },
}
