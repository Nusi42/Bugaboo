//! The scenarios of invitations, signing up and the first start (M1-F18).

use super::*;

/// An invitation of the administrator: its ID and the secret in its link.
async fn invitation(
    world: &World,
    email: &str,
) -> Result<(bb_core::id::InvitationId, String), String> {
    let created = create_invitation(
        &world.ctx(),
        CreateInvitation {
            session_token: world.admin.session_token.clone(),
            email: email.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let secret = created
        .link_path
        .split_once('#')
        .map(|(_, secret)| secret.to_owned())
        .ok_or("no secret in the link")?;
    Ok((created.invitation.id, secret))
}

pub(crate) async fn create_invitation_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_invitation(
            &world.ctx(),
            CreateInvitation {
                session_token: world.admin.session_token.clone(),
                email: "bob@example.test".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn revoke_invitation_scenario(world: &World) -> Added {
    let (id, _) = invitation(world, "bob@example.test").await?;
    measured(
        &world.store,
        revoke_invitation(
            &world.ctx(),
            RevokeInvitation {
                session_token: world.admin.session_token.clone(),
                invitation_id: id,
            },
        ),
    )
    .await
}

pub(crate) async fn accept_invitation_scenario(world: &World) -> Added {
    let (_, secret) = invitation(world, "bob@example.test").await?;
    measured(
        &world.store,
        accept_invitation(
            &world.ctx(),
            crate::auth::AcceptInvitation {
                secret,
                display_name: "Bob Builder".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn change_sign_up_policy_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_sign_up_policy(
            &world.ctx(),
            ChangeSignUpPolicy {
                session_token: world.admin.session_token.clone(),
                policy: SignUpPolicy {
                    enabled: true,
                    domains: vec!["example.test".to_owned()],
                    version: 0,
                },
            },
        ),
    )
    .await
}

pub(crate) async fn sign_up_scenario(world: &World) -> Added {
    change_sign_up_policy(
        &world.ctx(),
        ChangeSignUpPolicy {
            session_token: world.admin.session_token.clone(),
            policy: SignUpPolicy {
                enabled: true,
                domains: vec!["example.test".to_owned()],
                version: 0,
            },
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        sign_up(
            &world.ctx(),
            SignUp {
                email: "eve@example.test".to_owned(),
                display_name: "Eve".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn issue_setup_code_scenario(_world: &World) -> Added {
    // A server that nobody set up: the starting roles are there, nobody can manage roles, and the
    // start makes the setup code.
    let bare = World::bare()?;
    create_starting_roles(&bare.ctx(), CreateStartingRoles)
        .await
        .map_err(text)?;
    measured(&bare.store, issue_setup_code(&bare.ctx(), IssueSetupCode)).await
}

pub(crate) async fn complete_setup_scenario(_world: &World) -> Added {
    let bare = World::bare()?;
    // The start of the program makes the starting roles before the person comes to set up.
    create_starting_roles(&bare.ctx(), CreateStartingRoles)
        .await
        .map_err(text)?;
    let code = issue_setup_code(&bare.ctx(), IssueSetupCode)
        .await
        .map_err(text)?
        .ok_or("no setup code")?;
    measured(
        &bare.store,
        complete_setup(
            &bare.ctx(),
            CompleteSetup {
                code: code.as_str().to_owned(),
                display_name: "Ada Admin".to_owned(),
                email: "ada@example.test".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn create_starting_roles_scenario(_world: &World) -> Added {
    let bare = World::bare()?;
    measured(
        &bare.store,
        create_starting_roles(&bare.ctx(), CreateStartingRoles),
    )
    .await
}
