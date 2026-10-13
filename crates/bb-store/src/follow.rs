//! The adapter of the follow port (`bb_app::follow::FollowStore`): who follows an issue (ISS-07).

use bb_app::follow::FollowStore;
use bb_core::follow::{FollowReason, Follower};
use bb_core::id::{IssueId, UserId};
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::FollowerRow;

fn read_follower(row: &Row) -> Result<Follower, StoreError> {
    Ok(Follower {
        issue: row.id::<IssueId>(FollowerRow::Issue)?,
        person: row.id::<UserId>(FollowerRow::Person)?,
        reason: FollowReason::from_code(&row.get::<String>(FollowerRow::Reason)?)
            .ok_or(StoreError::Corrupt("a reason to follow"))?,
        since: row.time(FollowerRow::Since)?,
    })
}

impl FollowStore for AuthStorage {
    async fn followers_of_in(
        transaction: &mut Transaction,
        issue: IssueId,
    ) -> Result<Vec<Follower>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                FollowerRow::Issue,
                FollowerRow::Person,
                FollowerRow::Reason,
                FollowerRow::Since,
            ])
            .from(FollowerRow::Table)
            .and_where(Expr::col(FollowerRow::Issue).eq(issue.stored()))
            .order_by(FollowerRow::Since, Order::Asc)
            .order_by(FollowerRow::Person, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_follower)
            .collect()
    }

    async fn add_followers(
        transaction: &mut Transaction,
        followers: &[Follower],
    ) -> Result<(), StoreError> {
        for follower in followers {
            let mut insert = Query::insert();
            insert
                .into_table(FollowerRow::Table)
                .columns([
                    FollowerRow::Issue,
                    FollowerRow::Person,
                    FollowerRow::Reason,
                    FollowerRow::Since,
                ])
                .values([
                    Expr::val(follower.issue.stored()),
                    Expr::val(follower.person.stored()),
                    Expr::val(follower.reason.code()),
                    Expr::val(to_micros(follower.since)),
                ])
                .map_err(|_| StoreError::Corrupt("a follower"))?
                .on_conflict(
                    OnConflict::columns([FollowerRow::Issue, FollowerRow::Person])
                        .do_nothing()
                        .to_owned(),
                );
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn remove_follower(
        transaction: &mut Transaction,
        issue: IssueId,
        person: UserId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(FollowerRow::Table)
            .and_where(Expr::col(FollowerRow::Issue).eq(issue.stored()))
            .and_where(Expr::col(FollowerRow::Person).eq(person.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }
}
