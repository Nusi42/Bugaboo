//! The names of agents as a reader may be told them (ADR-0035): an agent is named only to a reader
//! who is told who the person responsible for it is (*Who sees people*), because otherwise the
//! text would say which of someone's agents wrote or changed it. Each agent is read once for a
//! request (PERF-N1). The one place that says it: the history and the marks of texts share it.

use std::collections::HashMap;

use bb_core::id::{AgentId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, PersonRef};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The agents a request has read: their names and the persons responsible, none for a gone agent.
#[derive(Default)]
pub(crate) struct AgentNames {
    read: HashMap<AgentId, Option<(String, UserId)>>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The name of the agent for a reader who may be told it, otherwise none; each agent is read
    /// once into `names`.
    pub(crate) async fn agent_named_to(
        &self,
        names: &mut AgentNames,
        held: &Evaluation,
        agent: AgentId,
    ) -> Result<Option<String>, AppError> {
        if let std::collections::hash_map::Entry::Vacant(place) = names.read.entry(agent) {
            let found = self
                .store()
                .agent(agent)
                .await
                .map_err(|error| internal::<S>(&error))?
                .map(|found| (found.name, found.person));
            place.insert(found);
        }
        let Some(Some((name, person))) = names.read.get(&agent) else {
            return Ok(None);
        };
        let told = self
            .person_ref(held, PeopleKind::Authors, PeopleContext::Ordinary, *person)
            .await?;
        Ok(matches!(told, PersonRef::Name { .. }).then(|| name.clone()))
    }
}
