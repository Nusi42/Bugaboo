//! AI in the sample (M7 part 6, AI-01 to AI-04, PRIV-08): an administrator sets up the provider,
//! confirms the agreement and switches AI on, marks what is never sent to it, and a Contributor lets
//! an AI app in. All of it goes through the use cases the screens call, one change a second of the
//! sample's clock.

use bb_app::access::StartingRole;
use bb_app::ai::{
    ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement, NeverSentTarget, SetNeverSent,
    change_ai_setup, change_project_ai, confirm_ai_agreement, set_never_sent,
};
use bb_app::auth::{
    AuthorizationRequest, DecideAuthorization, RegisterApp, RegisterOutcome, StartAppSession,
    TokenOutcome, TokenRequest, decide_authorization, exchange_token, register_app,
    start_app_session,
};
use bb_auth::pkce_challenge;
use bb_core::access_key::KeyChoice;
use bb_core::ai::{Destination, ProviderKind};
use bb_core::fields::Field;
use bb_dev_words::sample::{AiPlan, AppPlan};
use serde_json::json;

use crate::error::{SampleError, Step};
use crate::issues::Maker;
use crate::issues_setup::picked;

/// The port the app says it listens on when it is sent back (the address registered has none, as
/// an app on the computer asks the system for one).
const APP_PORT: u16 = 51234;
/// What the app proves it started the sign-in with (the example of the standard).
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
/// What the app asks the person to allow.
const APP_SCOPE: &str = "see-only";

fn failed(error: &dyn std::fmt::Display) -> SampleError {
    SampleError::step(Step::Ai, &error)
}

impl Maker<'_, '_> {
    /// The administrator sets up the provider, confirms the agreement, switches AI on for the
    /// organization and for the projects the plan names, and marks what is never sent to AI.
    pub(crate) async fn set_up_ai(&mut self, plan: &AiPlan) -> Result<(), SampleError> {
        let administrator = self.administrator()?;
        let token = self.hands.token(administrator).await?;
        let password = self.world.people[administrator].password.clone();
        let ctx = self.world.services.ctx(self.world.request());
        let provider = ChangeAiSetup {
            session_token: token.clone(),
            enabled: None,
            provider: Some(ProviderKind::OpenAiCompatible),
            model: Some(plan.model.clone()),
            address: Some(plan.address.clone()),
            send_names: None,
            password: Some(password),
        };
        self.world.tick();
        change_ai_setup(&ctx, provider)
            .await
            .map_err(|error| failed(&error))?;
        self.world.tick();
        confirm_ai_agreement(
            &ctx,
            ConfirmAiAgreement {
                session_token: token.clone(),
                destination: Destination::OtherProvider,
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        self.world.tick();
        let on = ChangeAiSetup {
            session_token: token.clone(),
            enabled: Some(true),
            provider: None,
            model: None,
            address: None,
            send_names: None,
            password: None,
        };
        change_ai_setup(&ctx, on)
            .await
            .map_err(|error| failed(&error))?;
        for (projects, enabled) in [(&plan.projects_on, true), (&plan.projects_off, false)] {
            for project in projects {
                self.world.tick();
                change_project_ai(
                    &ctx,
                    ChangeProjectAi {
                        session_token: token.clone(),
                        project: project.clone(),
                        enabled: Some(enabled),
                    },
                )
                .await
                .map_err(|error| failed(&error))?;
            }
        }
        for project in &plan.never_sent_projects {
            self.never_send(&token, NeverSentTarget::Project(project.clone()))
                .await?;
        }
        for code in &plan.never_sent_fields {
            let field = Field::from_code(code).ok_or_else(|| SampleError::NotMade(code.clone()))?;
            self.never_send(&token, NeverSentTarget::Field(field))
                .await?;
        }
        Ok(())
    }

    /// Marks a field or a project *Never sent to AI*.
    async fn never_send(&self, token: &str, target: NeverSentTarget) -> Result<(), SampleError> {
        self.world.tick();
        set_never_sent(
            &self.world.services.ctx(self.world.request()),
            SetNeverSent {
                session_token: token.to_owned(),
                target,
                marked: true,
                version: 0,
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        Ok(())
    }

    /// The first Contributor lets an AI app in, as a person does at the consent step, and the
    /// app opens a session, so the run page has one to show.
    pub(crate) async fn let_an_app_in(&mut self, plan: &AppPlan) -> Result<(), SampleError> {
        let contributor = self
            .world
            .people
            .iter()
            .position(|made| made.person.role == StartingRole::Contributor && !made.person.code_app)
            .ok_or(SampleError::PersonMissing)?;
        let token = self.hands.token(contributor).await?;
        let ctx = self.world.services.ctx(self.world.request());
        self.world.tick();
        let client = match register_app(
            &ctx,
            RegisterApp {
                name: plan.name.clone(),
                return_addresses: vec![plan.return_address.clone()],
            },
        )
        .await
        .map_err(|error| failed(&error))?
        {
            RegisterOutcome::Registered(app) => app.client_id,
            RegisterOutcome::Refused(problem) => return Err(failed(&problem.code())),
        };
        let return_address = plan
            .return_address
            .replace("127.0.0.1", &format!("127.0.0.1:{APP_PORT}"));
        self.world.tick();
        let decided = picked(
            decide_authorization(
                &ctx,
                DecideAuthorization {
                    session_token: token,
                    request: AuthorizationRequest {
                        client_id: client.to_string(),
                        response_type: "code".to_owned(),
                        return_address: return_address.clone(),
                        challenge: pkce_challenge(VERIFIER),
                        challenge_method: "S256".to_owned(),
                        scope: APP_SCOPE.to_owned(),
                        resource: String::new(),
                        state: "sample".to_owned(),
                    },
                    allow: true,
                    scope: KeyChoice::SeeOnly,
                    projects: Vec::new(),
                },
            )
            .await,
            Step::Ai,
            |answer| json!({ "redirect_to": answer.redirect_to }),
        )?;
        let code = decided["redirect_to"]
            .as_str()
            .and_then(|address| address.split_once('?'))
            .and_then(|(_, query)| bb_core::oauth::form_pairs(query))
            .and_then(|pairs| {
                pairs
                    .into_iter()
                    .find(|(name, _)| name == "code")
                    .map(|(_, code)| code)
            })
            .ok_or_else(|| SampleError::NotMade(plan.name.clone()))?;
        self.world.tick();
        let tokens = match exchange_token(
            &ctx,
            TokenRequest {
                grant_type: "authorization_code".to_owned(),
                client_id: client.to_string(),
                code: code.into(),
                return_address,
                verifier: VERIFIER.to_owned().into(),
                refresh_token: String::new().into(),
                resource: String::new(),
            },
        )
        .await
        .map_err(|error| failed(&error))?
        {
            TokenOutcome::Issued(answer) => answer,
            TokenOutcome::Refused(problem) => return Err(failed(&problem.code())),
        };
        self.world.tick();
        start_app_session(
            &ctx,
            StartAppSession {
                token: tokens.access_token.to_string(),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        Ok(())
    }
}
