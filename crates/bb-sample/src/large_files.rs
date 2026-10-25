//! The file records of the large dataset: rows without file bytes (a download of one is not found,
//! as when the folder of the files is lost), so the issues have files to list and count.

use bb_core::files::{MediaType, PreviewStatus, StoredFile};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use crate::large_ids::{rng_for, ten};
use crate::large_state::FILE_RECORDS;

/// The media types the file records have, with a size in bytes that is typical for each.
const KINDS: [(&str, i64); 3] = [
    ("image/png", 180_000),
    ("image/jpeg", 420_000),
    ("application/pdf", 650_000),
];

/// The file records, the same for the same seed.
pub(crate) fn records(seed: u64, at: Timestamp) -> Vec<StoredFile> {
    (0..FILE_RECORDS)
        .map(|number| {
            let mut random = rng_for(seed, 0xF11E, number as u64);
            let mut bytes = [0_u8; 32];
            bytes[..10].copy_from_slice(&ten(&mut random));
            bytes[10..20].copy_from_slice(&ten(&mut random));
            bytes[20..30].copy_from_slice(&ten(&mut random));
            let (media_type, size) = KINDS[number % KINDS.len()];
            StoredFile {
                hash: Digest::new(bytes),
                size: size + i64::try_from(random.below(50_000)).unwrap_or(0),
                media_type: MediaType::new(media_type).unwrap_or_else(MediaType::unknown),
                preview: PreviewStatus::Without,
                created_at: at,
            }
        })
        .collect()
}
