//! Every route of signing in once, in the order a person meets them.

use super::*;

fn origin() -> Result<Url, Failure> {
    Ok(Url::parse(PUBLIC_URL)?)
}

/// What the tour of the routes shares.
struct Tour {
    rig: Rig,
    app: Router,
    used: Arc<Mutex<BTreeSet<String>>>,
    ada: Browser,
}

const API: &str = "/api/v1/auth";

async fn first_start(tour: &mut Tour) -> TestResult {
    let rig = &tour.rig;
    let ada = &mut tour.ada;
    let api = API;
    // The first start.
    let reply = ada
        .route("GET", "/api/v1/auth/status", &format!("{api}/status"), None)
        .await?;
    assert_eq!(
        reply.body,
        json!({"setup_needed": true, "sign_up_open": false, "passkeys_available": true})
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/setup",
            &format!("{api}/setup"),
            Some(setup_body(&rig.setup_code)),
        )
        .await?;
    assert!(ok(&reply));
    let reply = ada
        .route("GET", "/api/v1/auth/me", &format!("{api}/me"), None)
        .await?;
    assert_eq!(reply.body["name"], "Ada Admin");

    Ok(())
}

async fn invitations(tour: &mut Tour) -> TestResult {
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // Invitations: make, list, look up, accept, revoke.
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/invitations",
            &format!("{api}/invitations"),
            Some(json!({"email": "bob@example.test"})),
        )
        .await?;
    assert!(ok(&reply));
    let secret = text(&reply, "/link_path")?
        .split_once('#')
        .ok_or("no secret")?
        .1
        .to_owned();
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/invitations",
            &format!("{api}/invitations"),
            None,
        )
        .await?;
    assert_eq!(reply.body["items"].as_array().map(Vec::len), Some(1));
    let mut bob = Browser::new(app, used);
    let reply = bob
        .route(
            "POST",
            "/api/v1/auth/invitations/lookup",
            &format!("{api}/invitations/lookup"),
            Some(json!({"secret": secret})),
        )
        .await?;
    assert_eq!(reply.body["email"], "bob@example.test");
    let reply = bob
        .route(
            "POST",
            "/api/v1/auth/invitations/accept",
            &format!("{api}/invitations/accept"),
            Some(json!({"secret": secret, "name": "Bob", "password": PASSWORD})),
        )
        .await?;
    assert!(ok(&reply));
    let reply = ada
        .send(
            "POST",
            &format!("{api}/invitations"),
            Some(json!({"email": "eve@example.test"})),
        )
        .await?;
    assert!(ok(&reply));
    let listed = ada.send("GET", &format!("{api}/invitations"), None).await?;
    let id = text(&listed, "/items/0/id")?;
    let reply = ada
        .route(
            "DELETE",
            "/api/v1/auth/invitations/{invitation}",
            &format!("{api}/invitations/{id}"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

async fn sign_up(tour: &mut Tour) -> TestResult {
    let ada = &mut tour.ada;
    let api = API;
    // Signing up on one's own: the rule, then somebody who uses it.
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/sign-up-policy",
            &format!("{api}/sign-up-policy"),
            None,
        )
        .await?;
    assert_eq!(
        reply.body,
        json!({"enabled": false, "domains": [], "version": 0})
    );
    let reply = ada
        .route(
            "PUT",
            "/api/v1/auth/sign-up-policy",
            &format!("{api}/sign-up-policy"),
            Some(json!({"enabled": true, "domains": ["example.test"], "version": 0})),
        )
        .await?;
    assert_eq!(
        reply.body,
        json!({"enabled": true, "domains": ["example.test"], "version": 1})
    );
    // Signing up sends a link by e-mail, so it needs e-mail.
    tour.rig.turn_mail_on().await?;
    let app = &tour.app;
    let used = &tour.used;
    let mut eve = Browser::new(app, used);
    let reply = eve
        .route(
            "POST",
            "/api/v1/auth/sign-up",
            &format!("{api}/sign-up"),
            Some(json!({"email": "eve@example.test"})),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::ACCEPTED, "{}", reply.body);
    let reply = eve
        .send(
            "POST",
            &format!("{api}/sign-up"),
            Some(json!({"email": "x@evil.test"})),
        )
        .await?;
    assert_eq!(reply.code(), "auth.sign-up-refused");
    Ok(())
}

async fn code_app(tour: &mut Tour) -> TestResult {
    let rig = &tour.rig;
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // The code app: set up, switch on, sign in with it, new backup codes, switch off.
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/code-app",
            &format!("{api}/two-step/code-app"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert!(
        reply.body["otpauth_url"]
            .as_str()
            .is_some_and(|url| url.starts_with("otpauth://"))
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/code-app/confirm",
            &format!("{api}/two-step/code-app/confirm"),
            Some(json!({"code": totp(rig, ADA_EMAIL).await?, "password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.body["codes"].as_array().map(Vec::len), Some(10));
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/two-step",
            &format!("{api}/two-step"),
            None,
        )
        .await?;
    assert_eq!(reply.body["code_app"], true);
    assert_eq!(reply.body["backup_codes_left"], 10);
    rig.clock.advance(30);
    let mut laptop = Browser::new(app, used);
    let reply = laptop
        .send(
            "POST",
            &format!("{api}/sign-in"),
            Some(sign_in_body(ADA_EMAIL, PASSWORD)),
        )
        .await?;
    assert_eq!(
        reply.body["second_step"],
        json!({"code_app": true, "passkey": false, "backup_code": true})
    );
    let reply = laptop.send("GET", &format!("{api}/me"), None).await?;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    let reply = laptop
        .route(
            "POST",
            "/api/v1/auth/second-step",
            &format!("{api}/second-step"),
            Some(json!({"code": totp(rig, ADA_EMAIL).await?})),
        )
        .await?;
    assert!(ok(&reply));
    assert_eq!(
        laptop.send("GET", &format!("{api}/me"), None).await?.status,
        StatusCode::OK
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/backup-codes",
            &format!("{api}/two-step/backup-codes"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.body["codes"].as_array().map(Vec::len), Some(10));
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/two-step/code-app/remove",
            &format!("{api}/two-step/code-app/remove"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

async fn passkeys(tour: &mut Tour) -> TestResult {
    let ada = &mut tour.ada;
    let api = API;
    // Passkeys: add, use as the second step, sign in with one alone, remove.
    let mut device = SoftPasskey::new(true);
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/passkeys/options",
            &format!("{api}/passkeys/options"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    let challenge = text(&reply, "/challenge_id")?;
    let answer = device.do_registration(
        origin()?,
        serde_json::from_value(reply.body["options"].clone())?,
    )?;
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/passkeys",
            &format!("{api}/passkeys"),
            Some(json!({"challenge_id": challenge, "credential": answer, "label": "Laptop", "password": PASSWORD})),
        )
        .await?;
    let passkey_id = text(&reply, "/id")?;
    use_passkeys(tour, &mut device, &passkey_id).await
}

/// A passkey as the second step, one alone, and removing it.
async fn use_passkeys(tour: &mut Tour, device: &mut SoftPasskey, passkey_id: &str) -> TestResult {
    let rig = &tour.rig;
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    let mut phone = Browser::new(app, used);
    phone
        .send(
            "POST",
            &format!("{api}/sign-in"),
            Some(sign_in_body(ADA_EMAIL, PASSWORD)),
        )
        .await?;
    let reply = phone
        .route(
            "POST",
            "/api/v1/auth/second-step/passkey/options",
            &format!("{api}/second-step/passkey/options"),
            None,
        )
        .await?;
    let challenge = text(&reply, "/challenge_id")?;
    let answer = device.do_authentication(
        origin()?,
        serde_json::from_value(reply.body["options"].clone())?,
    )?;
    let reply = phone
        .route(
            "POST",
            "/api/v1/auth/second-step/passkey",
            &format!("{api}/second-step/passkey"),
            Some(json!({"challenge_id": challenge, "credential": answer})),
        )
        .await?;
    assert!(ok(&reply));
    assert_eq!(
        phone.send("GET", &format!("{api}/me"), None).await?.status,
        StatusCode::OK
    );
    let mut tablet = Browser::new(app, used);
    let reply = tablet
        .route(
            "POST",
            "/api/v1/auth/passkey/sign-in/options",
            &format!("{api}/passkey/sign-in/options"),
            None,
        )
        .await?;
    let challenge = text(&reply, "/challenge_id")?;
    let person = rig.person_id(ADA_EMAIL).await?;
    let stored = rig.storage.passkeys_of(person).await?;
    let mut options = reply.body["options"].clone();
    options["publicKey"]["allowCredentials"] = json!([{
        "type": "public-key",
        "id": Base64UrlUnpadded::encode_string(&stored[0].credential_id),
    }]);
    let mut answer = serde_json::to_value(
        device.do_authentication(origin()?, serde_json::from_value(options)?)?,
    )?;
    answer["response"]["userHandle"] = json!(Base64UrlUnpadded::encode_string(&person.to_bytes()));
    let reply = tablet
        .route(
            "POST",
            "/api/v1/auth/passkey/sign-in",
            &format!("{api}/passkey/sign-in"),
            Some(json!({"challenge_id": challenge, "credential": answer})),
        )
        .await?;
    assert!(ok(&reply));
    assert_eq!(
        tablet.send("GET", &format!("{api}/me"), None).await?.status,
        StatusCode::OK
    );
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/passkeys/{passkey}/remove",
            &format!("{api}/passkeys/{passkey_id}/remove"),
            Some(json!({"password": PASSWORD})),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

async fn sessions_and_password(tour: &mut Tour) -> TestResult {
    let app = &tour.app;
    let used = &tour.used;
    let ada = &mut tour.ada;
    let api = API;
    // Sessions and the password: list, end one, change the password, end all, sign out.
    let reply = ada
        .route(
            "GET",
            "/api/v1/auth/sessions",
            &format!("{api}/sessions"),
            None,
        )
        .await?;
    let sessions = reply.body["sessions"].as_array().ok_or("no sessions")?;
    assert!(sessions.iter().any(|session| session["current"] == true));
    let other = sessions
        .iter()
        .find(|session| session["current"] == false)
        .and_then(|session| session["id"].as_str())
        .map(str::to_owned)
        .ok_or("no other session")?;
    let reply = ada
        .route(
            "DELETE",
            "/api/v1/auth/sessions/{session}",
            &format!("{api}/sessions/{other}"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply = ada
        .route(
            "POST",
            "/api/v1/auth/password",
            &format!("{api}/password"),
            Some(json!({"current_password": PASSWORD, "new_password": OTHER_PASSWORD})),
        )
        .await?;
    assert!(ok(&reply));
    let reply = ada
        .route(
            "DELETE",
            "/api/v1/auth/sessions",
            &format!("{api}/sessions"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(ada.jar.is_empty());
    let mut again = Browser::new(app, used);
    let reply = again
        .route(
            "POST",
            "/api/v1/auth/sign-in",
            &format!("{api}/sign-in"),
            Some(sign_in_body(ADA_EMAIL, OTHER_PASSWORD)),
        )
        .await?;
    assert!(ok(&reply));
    let reply = again
        .route(
            "POST",
            "/api/v1/auth/sign-out",
            &format!("{api}/sign-out"),
            None,
        )
        .await?;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    Ok(())
}

/// The routes of the API description below `prefix`, as `METHOD /path`.
fn documented_routes(prefix: &str) -> BTreeSet<String> {
    operations()
        .iter()
        .filter(|operation| operation.path.starts_with(prefix))
        .map(|operation| format!("{} {}", operation.method, operation.path))
        .collect()
}

/// Every route of the API description under /auth was used.
fn every_route_was_used(tour: &Tour) -> TestResult {
    let documented = documented_routes("/api/v1/auth/");
    let used = tour.used.lock().map_err(|_| "poisoned")?.clone();
    assert_eq!(
        documented.difference(&used).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "documented routes that no test used"
    );
    assert!(documented.len() >= 28, "{}", documented.len());

    Ok(())
}

#[test]
fn the_walk_over_the_routes_of_the_description_sees_every_method_including_patch() {
    let documented = documented_routes("/api/v1/");
    let patches: Vec<String> = operations()
        .iter()
        .filter(|operation| operation.method == Method::PATCH)
        .map(|operation| format!("PATCH {}", operation.path))
        .collect();
    assert!(!patches.is_empty(), "the description has no PATCH route");
    for patch in patches {
        assert!(documented.contains(&patch), "{patch} is missed by the walk");
    }
}

/// Every route of the sign-in family, used once the way the browser app will use it. The routes
/// that were used are compared with the API description at the end, so a new route that no
/// test uses fails here.
#[tokio::test]
async fn auth_01_every_route_of_signing_in_works_over_http() -> TestResult {
    each_database("auth_01_http_every_route", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let used = called();
        let ada = Browser::new(&app, &used);
        let mut tour = Tour {
            rig,
            app,
            used,
            ada,
        };
        first_start(&mut tour).await?;
        invitations(&mut tour).await?;
        sign_up(&mut tour).await?;
        code_app(&mut tour).await?;
        passkeys(&mut tour).await?;
        sessions_and_password(&mut tour).await?;
        every_route_was_used(&tour)
    })
    .await
}
