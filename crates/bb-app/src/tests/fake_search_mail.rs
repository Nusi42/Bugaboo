//! The e-mailed-search part of the in-memory store (QRY-07).

use bb_core::id::{SavedSearchId, SearchMailId, UserId};
use bb_core::search_mail::{MailPause, SearchMail};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use super::fake_many_changes::job;
use crate::jobs::QueuedJob;
use crate::search_mail::SearchMailStore;

impl SearchMailStore for FakeStore {
    async fn search_mail_in(
        tx: &mut Tx,
        id: SearchMailId,
    ) -> Result<Option<SearchMail>, FakeError> {
        Ok(tx
            .state
            .search_mails
            .iter()
            .find(|mail| mail.id == id)
            .cloned())
    }

    async fn search_mail_of_in(
        tx: &mut Tx,
        holder: UserId,
        search: SavedSearchId,
    ) -> Result<Option<SearchMail>, FakeError> {
        Ok(tx
            .state
            .search_mails
            .iter()
            .find(|mail| mail.holder == holder && mail.search == search)
            .cloned())
    }

    async fn count_search_mails_of_in(tx: &mut Tx, holder: UserId) -> Result<u64, FakeError> {
        let found = tx
            .state
            .search_mails
            .iter()
            .filter(|mail| mail.holder == holder)
            .count();
        Ok(u64::try_from(found).unwrap_or(u64::MAX))
    }

    async fn search_mails_of(&self, holder: UserId) -> Result<Vec<SearchMail>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<SearchMail> = state
                .search_mails
                .iter()
                .filter(|mail| mail.holder == holder)
                .cloned()
                .collect();
            found.sort_by_key(|mail| mail.id);
            found
        }))
    }

    async fn search_mails_due(
        &self,
        until: Timestamp,
        limit: u64,
    ) -> Result<Vec<SearchMail>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<SearchMail> = state
                .search_mails
                .iter()
                .filter(|mail| mail.paused.is_none() && mail.next_at <= until)
                .cloned()
                .collect();
            found.sort_by_key(|mail| (mail.next_at, mail.id));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn queue_search_mail(
        &self,
        id: SearchMailId,
        due: Timestamp,
        next: Timestamp,
        queued: &QueuedJob,
        now: Timestamp,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let state = &mut shared.state;
        let Some(mail) = state
            .search_mails
            .iter_mut()
            .find(|mail| mail.id == id && mail.next_at == due && mail.paused.is_none())
        else {
            return Ok(false);
        };
        mail.next_at = next;
        state.jobs.push(job(
            queued.id,
            &queued.kind,
            &queued.payload,
            queued.run_at,
            now,
        ));
        Ok(true)
    }

    async fn search_mail_sent(&self, id: SearchMailId, at: Timestamp) -> Result<(), FakeError> {
        let mut shared = self.lock();
        if let Some(mail) = shared
            .state
            .search_mails
            .iter_mut()
            .find(|mail| mail.id == id)
        {
            mail.last_sent_at = Some(at);
        }
        Ok(())
    }

    async fn pause_search_mail(&self, id: SearchMailId, pause: MailPause) -> Result<(), FakeError> {
        let mut shared = self.lock();
        if let Some(mail) = shared
            .state
            .search_mails
            .iter_mut()
            .find(|mail| mail.id == id)
        {
            mail.paused = Some(pause);
        }
        Ok(())
    }

    async fn put_search_mail(tx: &mut Tx, mail: &SearchMail) -> Result<(), FakeError> {
        let mails = &mut tx.state.search_mails;
        match mails.iter_mut().find(|found| found.id == mail.id) {
            Some(found) => *found = mail.clone(),
            None => mails.push(mail.clone()),
        }
        Ok(())
    }

    async fn delete_search_mail(tx: &mut Tx, id: SearchMailId) -> Result<bool, FakeError> {
        let mails = &mut tx.state.search_mails;
        let before = mails.len();
        mails.retain(|mail| mail.id != id);
        Ok(mails.len() < before)
    }
}
