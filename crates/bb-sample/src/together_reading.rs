//! The people who only read open their notifications and mark some as read.

use bb_app::notifications::{
    NotificationsQuery, ReadNotifications, Which, notifications, read_notifications,
};
use bb_core::id::NotificationId;
use bb_core::notification::NotificationFilter;

use crate::error::{SampleError, Step};
use crate::issues::Maker;
use crate::issues_setup::{id_in, json};
use crate::together::{NOTIFICATIONS_PAGE, Work};

impl Maker<'_, '_> {
    /// The people who only read open their notifications and mark some of them as read.
    pub(crate) async fn people_read(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let readers: Vec<usize> = work
            .viewers
            .iter()
            .chain(&work.guests)
            .chain(&work.contributors)
            .copied()
            .collect();
        for person in readers {
            let token = self.hands.token(person).await?;
            let ctx = self.world.services.ctx(self.world.request());
            let mut to_mark: Vec<NotificationId> = Vec::new();
            let mut cursor = None;
            loop {
                let page = json(
                    notifications(
                        &ctx,
                        NotificationsQuery {
                            session_token: token.clone(),
                            filter: NotificationFilter::All,
                            cursor,
                            limit: NOTIFICATIONS_PAGE,
                        },
                    )
                    .await,
                    Step::Notifications,
                )?;
                for item in page["items"].as_array().map_or(&[][..], Vec::as_slice) {
                    if work.random.chance(work.plan.read_percent) {
                        to_mark.push(id_in(item, "id")?);
                    }
                }
                cursor = page["next_cursor"].as_str().map(str::to_owned);
                if cursor.is_none() {
                    break;
                }
            }
            for chunk in to_mark.chunks(bb_core::notification::AT_ONCE_MAX) {
                self.world.tick();
                read_notifications(
                    &ctx,
                    ReadNotifications {
                        session_token: token.clone(),
                        which: Which::These(chunk.to_vec()),
                    },
                )
                .await
                .map_err(|error| SampleError::step(Step::Notifications, &error))?;
            }
        }
        Ok(())
    }
}
