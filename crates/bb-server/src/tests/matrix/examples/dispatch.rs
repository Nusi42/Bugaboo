//! The dispatch of the example requests of the permission matrix: each area answers the operations
//! of its routes, in its own file.

use bb_testing::Failure;

use super::super::actors::Actor;
use super::super::examples_releases::release_request;
use super::super::routes::Fixture;
use super::{
    Example, Request, agent_request, open_request, organization_request, own_request,
    project_request,
};
use crate::sign_in_tests::http::Browser;

/// The request of an example: the method, the address with its parts filled in, and a body that
/// is valid for the route. Changes that need a thing to change make it first (a new invitation to
/// take back, the session of the actor to end).
///
/// # Errors
/// A failure of the set-up, or an operation that has a row but no request here.
pub(in super::super) async fn request(
    fixture: &Fixture<'_>,
    example: &Example,
    actor: Actor,
    browser: &mut Browser,
) -> Result<Request, Failure> {
    let operation = example.operation;
    if let Some(found) = open_request(operation)? {
        return Ok(found);
    }
    if let Some(found) = own_request(operation, actor, browser).await? {
        return Ok(found);
    }
    if let Some(found) = agent_request(operation) {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_agent_work::agent_work_request(operation) {
        return Ok(found);
    }
    if let Some(found) = organization_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = project_request(fixture, operation)? {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_people::people_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_editing::editing_request(fixture, operation) {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_ai::ai_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_fields::field_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_issues::issue_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_layouts::layout_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = release_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_themes::theme_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_data_exports::data_export_request(fixture, operation)?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_people_state::people_state_request(fixture, operation).await?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_relations::relation_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_files::file_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_searches::search_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) = super::super::examples_tags::tag_request(fixture, operation).await? {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_workflows::workflow_request(fixture, operation).await?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_time::time_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_working_hours::working_hours_request(fixture, operation)?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_work_times::work_times_request(fixture, operation, browser).await?
    {
        return Ok(found);
    }
    if let Some(found) =
        super::super::examples_timeline::timeline_request(fixture, operation).await?
    {
        return Ok(found);
    }
    Err(format!("{operation} has a row in the table but no request").into())
}
