//! The personal data of one person (ADR-0044): the rows the walk of a person's data finds, as the
//! port [`PersonData`](crate::ports::PersonData) hands them over, and what *Download my data* and
//! *Remove a person* make of them.

pub mod export;
pub mod replay;
pub mod walk;
