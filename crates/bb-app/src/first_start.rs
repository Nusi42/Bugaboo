//! The first start of a server, in the one order it has to run (ADR-0025, ADR-0021, ISS-02): the
//! starting roles first, so whoever sets the organization up gets the Administrator role; then
//! the starting issue types and kinds of relation and the starter workflows of a new organization;
//! then the setup code, while nobody can manage
//! roles yet. Every part does nothing when it was done before, so the whole is run at every start.
//!
//! The server, the sample organization and the tests start this way and no one of them lists the
//! steps itself.

use crate::access::{CreateStartingRoles, create_starting_roles};
use bb_auth::HumanCode;

use crate::auth::{
    AppError, AuthServices, AuthStore, IssueSetupCode, RequestInfo, issue_setup_code,
};
use crate::issues::{CreateStartingIssueTypes, create_starting_issue_types};
use crate::relations::{CreateStartingRelationKinds, create_starting_relation_kinds};
use crate::workflows::{CreateStartingWorkflows, create_starting_workflows};

/// Makes what a new organization starts with, and the setup code while the organization has no
/// administrator. Returns the new code when one was made.
///
/// # Errors
/// Fails when the store fails.
pub async fn first_start<S: AuthStore>(
    services: &AuthServices<S>,
) -> Result<Option<HumanCode>, AppError> {
    // Nobody asks: the server does this by itself, so there is no request to name.
    let ctx = services.ctx(RequestInfo::default());
    create_starting_roles(&ctx, CreateStartingRoles).await?;
    create_starting_issue_types(&ctx, CreateStartingIssueTypes).await?;
    create_starting_relation_kinds(&ctx, CreateStartingRelationKinds).await?;
    create_starting_workflows(&ctx, CreateStartingWorkflows).await?;
    issue_setup_code(&ctx, IssueSetupCode).await
}
