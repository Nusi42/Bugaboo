//! The rows an answer is read from, and the IDs and moments as they are stored: the same for both
//! databases. Split from `db.rs`, which holds the pools, the transactions and the statements.

use bb_core::id::{
    AccessKeyId, AssignmentId, AttachmentId, ChoiceId, CommentId, EditorId, EntityId, EventId,
    FieldId, GroupId, IdError, InvitationId, IssueId, IssueTypeId, JobId, LabelGroupId, LabelId,
    LayoutId, LockToken, ManyChangeRunId, NotificationId, PasskeyId, ProjectId, RelationId,
    RelationKindId, RoleId, SavedSearchId, SessionId, TagId, UploadId, UserId, WorkflowId,
};
use jiff::Timestamp;
use sea_query::Iden;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::{Postgres, Row as _, Sqlite};

use crate::error::StoreError;

/// One row of an answer, from either database.
pub(crate) enum Row {
    Sqlite(SqliteRow),
    Postgres(PgRow),
}

impl Row {
    /// The value of a column. Whole numbers are always `i64` (`BIGINT`), so both databases give
    /// the same type.
    pub(crate) fn get<'r, T>(&'r self, column: impl Iden + Copy) -> Result<T, StoreError>
    where
        T: sqlx::Decode<'r, Sqlite>
            + sqlx::Type<Sqlite>
            + sqlx::Decode<'r, Postgres>
            + sqlx::Type<Postgres>,
    {
        let name = column.to_string();
        match self {
            Self::Sqlite(row) => row.try_get(name.as_str()),
            Self::Postgres(row) => row.try_get(name.as_str()),
        }
        .map_err(StoreError::from)
    }

    /// An ID stored as 16 bytes.
    pub(crate) fn id<T: StoredId>(&self, column: impl Iden + Copy) -> Result<T, StoreError> {
        let bytes: Vec<u8> = self.get(column)?;
        T::read(bytes)
    }

    /// An ID that may be missing.
    pub(crate) fn optional_id<T: StoredId>(
        &self,
        column: impl Iden + Copy,
    ) -> Result<Option<T>, StoreError> {
        self.get::<Option<Vec<u8>>>(column)?
            .map(T::read)
            .transpose()
    }

    /// A moment stored as microseconds since 1970.
    pub(crate) fn time(&self, column: impl Iden + Copy) -> Result<Timestamp, StoreError> {
        from_micros(self.get(column)?)
    }

    /// A moment that may be missing.
    pub(crate) fn optional_time(
        &self,
        column: impl Iden + Copy,
    ) -> Result<Option<Timestamp>, StoreError> {
        self.get::<Option<i64>>(column)?
            .map(from_micros)
            .transpose()
    }
}

/// An ID as the database holds it: 16 bytes. Writing and reading go through here, so no module
/// has its own copy of the conversion or its own wording for "this row cannot be read".
pub(crate) trait StoredId: Copy {
    /// What the ID is, for the report of a stored value that is not one.
    const WHAT: &'static str;

    /// The ID with these bytes, if they are one.
    fn from_stored(bytes: [u8; 16]) -> Result<Self, IdError>;

    /// The 16 bytes of the ID.
    fn to_stored(self) -> [u8; 16];

    /// The bytes as a value to bind to a query.
    fn stored(self) -> Vec<u8> {
        self.to_stored().to_vec()
    }

    /// The ID stored in `bytes`, or the report that the row is corrupt.
    fn read(bytes: Vec<u8>) -> Result<Self, StoreError> {
        <[u8; 16]>::try_from(bytes)
            .ok()
            .and_then(|bytes| Self::from_stored(bytes).ok())
            .ok_or(StoreError::Corrupt(Self::WHAT))
    }
}

macro_rules! stored_ids {
    ($($ty:ident => $what:literal),+ $(,)?) => {
        $(impl StoredId for $ty {
            const WHAT: &'static str = $what;

            fn from_stored(bytes: [u8; 16]) -> Result<Self, IdError> {
                Self::from_bytes(bytes)
            }

            fn to_stored(self) -> [u8; 16] {
                self.to_bytes()
            }
        })+
    };
}

stored_ids!(
    AccessKeyId => "an access key ID",
    AssignmentId => "an assignment ID",
    AttachmentId => "an attachment ID",
    UploadId => "an upload ID",
    ManyChangeRunId => "a run ID",
    CommentId => "a comment ID",
    EditorId => "an editor ID",
    EntityId => "a record ID",
    EventId => "an event ID",
    FieldId => "a field ID",
    ChoiceId => "a choice ID",
    GroupId => "a group ID",
    InvitationId => "an invitation ID",
    IssueId => "an issue ID",
    IssueTypeId => "an issue type ID",
    JobId => "a job ID",
    LabelGroupId => "a label group ID",
    LabelId => "a label ID",
    LayoutId => "a layout ID",
    LockToken => "an edit lock token",
    NotificationId => "a notification ID",
    PasskeyId => "a passkey ID",
    ProjectId => "a project ID",
    TagId => "a tag ID",
    RelationId => "a relation ID",
    RelationKindId => "a kind of relation ID",
    RoleId => "a role ID",
    SavedSearchId => "a saved search ID",
    SessionId => "a session ID",
    UserId => "a person ID",
    WorkflowId => "a workflow ID",
);

/// A moment as it is stored: microseconds since 1970 (UTC), the same whole number in both databases.
pub(crate) fn to_micros(time: Timestamp) -> i64 {
    time.as_microsecond()
}

/// The moment stored as `micros`.
pub(crate) fn from_micros(micros: i64) -> Result<Timestamp, StoreError> {
    Timestamp::from_microsecond(micros).map_err(|_| StoreError::Corrupt("a time"))
}
