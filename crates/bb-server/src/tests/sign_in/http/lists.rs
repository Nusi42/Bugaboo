//! The lists of invitations over HTTP: their fields, cursor and limit, and what a hard cap says.

use super::*;

#[tokio::test]
async fn perf_02_the_invitation_list_takes_fields_cursor_and_limit_over_http() -> TestResult {
    each_database("perf_02_http_invitation_list", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let mut ada = Browser::signed_in(&app, &rig).await?;
        for number in 0..3 {
            let reply = ada
                .send(
                    "POST",
                    "/api/v1/auth/invitations",
                    Some(json!({"email": format!("person{number}@example.test")})),
                )
                .await?;
            assert!(ok(&reply));
        }
        // Two fields, two to a page: only those keys, and a cursor for the rest.
        let first = ada
            .send(
                "GET",
                "/api/v1/auth/invitations?fields=id,email&limit=2",
                None,
            )
            .await?;
        assert_eq!(first.status, StatusCode::OK);
        let items = first.body["items"].as_array().ok_or("no items")?;
        assert_eq!(items.len(), 2);
        for item in items {
            let mut keys: Vec<&String> = item.as_object().ok_or("no object")?.keys().collect();
            keys.sort();
            assert_eq!(keys, ["email", "id"]);
        }
        let cursor = text(&first, "/next_cursor")?;
        let last = ada
            .send(
                "GET",
                &format!("/api/v1/auth/invitations?limit=2&cursor={cursor}"),
                None,
            )
            .await?;
        assert_eq!(last.body["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(last.body["next_cursor"], Value::Null);
        // The last page has `next_cursor` as `null`, and the default is the small set of the
        // route: the ID and the address; the dates only when asked for.
        assert!(
            last.body
                .as_object()
                .is_some_and(|page| page.contains_key("next_cursor"))
        );
        let mut default_keys: Vec<&String> = last.body["items"][0]
            .as_object()
            .ok_or("no object")?
            .keys()
            .collect();
        default_keys.sort();
        assert_eq!(default_keys, ["email", "id"]);
        // A field that does not exist, a cursor that is not one or was changed, and a page size
        // that is no number are refused, each with its own code.
        let mut changed = cursor.clone();
        changed.pop();
        changed.push(if cursor.ends_with('A') { 'B' } else { 'A' });
        for (bad, code) in [
            ("fields=password".to_owned(), "request.unknown-field"),
            ("cursor=nonsense".to_owned(), "request.bad-cursor"),
            (format!("cursor={changed}"), "request.bad-cursor"),
            ("limit=many".to_owned(), "request.invalid"),
        ] {
            let reply = ada
                .send("GET", &format!("/api/v1/auth/invitations?{bad}"), None)
                .await?;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
            assert_eq!(reply.code(), code, "{bad}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_password_alone_shows_nothing_about_the_person_until_the_second_step_is_done()
-> TestResult {
    each_database("sec_12_http_hidden_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let used = called();
        let mut ada = Browser::new(&app, &used);
        ada.set_up(&rig).await?;
        ada.send(
            "POST",
            "/api/v1/auth/two-step/code-app",
            Some(json!({"password": PASSWORD})),
        )
        .await?;
        let confirmed = ada
            .send(
                "POST",
                "/api/v1/auth/two-step/code-app/confirm",
                Some(json!({"code": totp(&rig, ADA_EMAIL).await?, "password": PASSWORD})),
            )
            .await?;
        assert!(ok(&confirmed));
        rig.clock.advance(30);
        let mut laptop = Browser::new(&app, &used);
        let password_only = laptop.sign_in(ADA_EMAIL, PASSWORD).await?;
        assert_eq!(password_only.status, StatusCode::OK);
        assert_eq!(password_only.body["person"], Value::Null);
        assert!(password_only.body["second_step"].is_object());
        let shown = password_only.body.to_string();
        for private in ["Ada Admin", ADA_EMAIL, "administrator"] {
            assert!(!shown.contains(private), "{private}");
        }
        let finished = laptop
            .send(
                "POST",
                "/api/v1/auth/second-step",
                Some(json!({"code": totp(&rig, ADA_EMAIL).await?})),
            )
            .await?;
        assert_eq!(finished.body["person"]["email"], ADA_EMAIL);
        assert_eq!(finished.body["second_step"], Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_the_lists_with_a_hard_cap_say_whether_they_were_cut() -> TestResult {
    each_database("perf_02_http_capped_lists", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let sessions = ada.send("GET", "/api/v1/auth/sessions", None).await?;
        assert_eq!(sessions.body["truncated"], false);
        let two_step = ada.send("GET", "/api/v1/auth/two-step", None).await?;
        assert_eq!(two_step.body["truncated"], false);
        assert!(two_step.body["passkeys"].is_array());
        Ok(())
    })
    .await
}
