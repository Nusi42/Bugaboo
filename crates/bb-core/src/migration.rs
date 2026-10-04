//! The record of a schema change that has been applied to the database.

use jiff::Timestamp;

use crate::personal::Entity;
use crate::security_log::Digest;
use crate::technical::TechnicalName;

/// One applied migration.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct MigrationRecord {
    /// The number of the migration, counting up from 1.
    #[not_personal]
    pub version: i64,
    /// Its name.
    #[not_personal]
    pub name: TechnicalName,
    /// The checksum of the steps the migration had when it was applied. A migration that is
    /// changed afterwards no longer matches, and the program refuses to start.
    #[not_personal]
    pub checksum: Digest,
    /// When it was applied.
    #[not_personal]
    pub applied_at: Timestamp,
}
