//! The settings routes over HTTP (CFG-04, CFG-06): what the browser app sees. Who may change what
//! is tested through the use cases in `settings.rs`; here the answers, the codes, the cache rules
//! and the one place where a locked setting says why.

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, called};
use super::rig::Rig;
use super::status::app_with_probe;

fn setting<'a>(list: &'a Value, key: &str) -> Result<&'a Value, Failure> {
    list["settings"]
        .as_array()
        .and_then(|settings| settings.iter().find(|setting| setting["key"] == key))
        .ok_or_else(|| format!("no setting {key}").into())
}

#[tokio::test]
async fn cfg_04_my_settings_over_http_say_what_each_setting_is_and_where_its_value_comes_from()
-> TestResult {
    each_database("cfg_04_http_my_settings", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut nobody = Browser::new(&app, &called());
        // Not signed in: nothing, and no secret of the answer is stored.
        let refused = nobody.send("GET", "/api/v1/me/settings", None).await?;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(refused.code(), "auth.not_signed_in");

        let mut ada = Browser::signed_in(&app, &rig).await?;
        // The settings of a topic, with what a form needs to draw them.
        let list = ada
            .send("GET", "/api/v1/me/settings?topic=appearance.", None)
            .await?;
        assert_eq!(list.status, StatusCode::OK);
        assert_eq!(
            list.header("cache-control"),
            "no-store",
            "a person's own data"
        );
        assert_eq!(list.body["settings"].as_array().map(Vec::len), Some(7));
        let mode = setting(&list.body, "appearance.mode")?;
        assert_eq!(mode["kind"], "text");
        assert_eq!(mode["format"], "choice");
        assert_eq!(mode["name_key"], "theme.legend");
        assert_eq!(mode["help_key"], "setting.appearance.mode.help");
        assert_eq!(mode["value"], "device");
        assert_eq!(mode["default"], "device");
        assert_eq!(mode["source"], json!({"kind": "default"}));
        assert_eq!(mode["own"], false);
        assert_eq!(mode["locked"], false);
        assert_eq!(mode["lockable"], true);
        assert_eq!(mode["levels"], json!(["organization", "person"]));
        let choices: Vec<&str> = mode["choices"]
            .as_array()
            .ok_or("no choices")?
            .iter()
            .filter_map(|choice| choice["value"].as_str())
            .collect();
        assert_eq!(choices, ["device", "light", "dark"]);
        let switch = setting(&list.body, "appearance.less-motion")?;
        assert_eq!(
            (&switch["kind"], &switch["value"]),
            (&json!("bool"), &json!(false))
        );
        // A whole number says its bounds, on the organization's level.
        let rules = ada
            .send("GET", "/api/v1/settings?topic=security.", None)
            .await?;
        let idle = setting(&rules.body, "security.session-idle-minutes")?;
        assert_eq!(idle["bounds"], json!({"least": 5, "most": 1440}));
        assert_eq!(idle["levels"], json!(["organization"]));

        // Changing a value answers the setting as it is now; resetting it takes it away again.
        let changed = ada
            .send(
                "PUT",
                "/api/v1/me/settings/appearance.mode",
                Some(json!({"value": "dark"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK);
        assert_eq!(changed.body["value"], "dark");
        assert_eq!(changed.body["source"], json!({"kind": "person"}));
        assert_eq!(changed.body["own"], true);
        let back = ada
            .send("DELETE", "/api/v1/me/settings/appearance.mode", None)
            .await?;
        assert_eq!(
            (&back.body["value"], &back.body["own"]),
            (&json!("device"), &json!(false))
        );

        // What the registry does not accept is refused with the code of an invalid request.
        for (key, value) in [
            ("appearance.mode", json!("sometimes")),
            ("appearance.mode", json!(true)),
            ("appearance.less-motion", json!("yes")),
            ("region.time-zone", json!("Nowhere")),
            ("start.page", json!("https://evil.test/")),
        ] {
            let refused = ada
                .send(
                    "PUT",
                    &format!("/api/v1/me/settings/{key}"),
                    Some(json!({"value": value})),
                )
                .await?;
            assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{key}");
            assert_eq!(refused.code(), "request.invalid", "{key}");
        }
        let unknown = ada
            .send(
                "PUT",
                "/api/v1/me/settings/appearance.colour",
                Some(json!({"value": "x"})),
            )
            .await?;
        assert_eq!(unknown.status, StatusCode::NOT_FOUND);
        assert_eq!(unknown.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_an_administrator_locks_a_setting_and_a_person_is_told_why_over_http() -> TestResult
{
    each_database("cfg_06_http_lock", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let mut bob = ada.invite("bob@example.test", "Bob").await?;

        // A person without the permission neither lists nor changes the organization's settings.
        let listed = bob.send("GET", "/api/v1/settings", None).await?;
        assert_eq!(
            (listed.status, listed.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let tried = bob
            .send("PUT", "/api/v1/settings/appearance.mode/lock", None)
            .await?;
        assert_eq!(tried.status, StatusCode::FORBIDDEN);

        let set = ada
            .send(
                "PUT",
                "/api/v1/settings/appearance.mode",
                Some(json!({"value": "dark"})),
            )
            .await?;
        assert_eq!(set.body["source"], json!({"kind": "organization"}));
        let locked = ada
            .send("PUT", "/api/v1/settings/appearance.mode/lock", None)
            .await?;
        assert_eq!(
            (locked.status, &locked.body["locked"]),
            (StatusCode::OK, &json!(true))
        );
        // Bob sees the organization's value, read only, and cannot change it.
        let mine = bob
            .send("GET", "/api/v1/me/settings?topic=appearance.mode", None)
            .await?;
        let view = setting(&mine.body, "appearance.mode")?;
        assert_eq!(
            (&view["value"], &view["locked"], &view["source"]),
            (
                &json!("dark"),
                &json!(true),
                &json!({"kind": "organization"})
            )
        );
        let refused = bob
            .send(
                "PUT",
                "/api/v1/me/settings/appearance.mode",
                Some(json!({"value": "light"})),
            )
            .await?;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(refused.code(), "setting.locked");
        // A setting that cannot be locked says so as an invalid request.
        let cannot = ada
            .send(
                "PUT",
                "/api/v1/settings/security.session-idle-minutes/lock",
                None,
            )
            .await?;
        assert_eq!(
            (cannot.status, cannot.code()),
            (StatusCode::BAD_REQUEST, "request.invalid")
        );
        // Unlocked, Bob may choose again.
        let open = ada
            .send("DELETE", "/api/v1/settings/appearance.mode/lock", None)
            .await?;
        assert_eq!(open.body["locked"], false);
        let chosen = bob
            .send(
                "PUT",
                "/api/v1/me/settings/appearance.mode",
                Some(json!({"value": "light"})),
            )
            .await?;
        assert_eq!(chosen.status, StatusCode::OK);
        // Resetting the organization's value takes the lock with it.
        let reset = ada
            .send("DELETE", "/api/v1/settings/appearance.mode", None)
            .await?;
        assert_eq!(
            (&reset.body["own"], &reset.body["locked"]),
            (&json!(false), &json!(false))
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_a_project_the_person_cannot_see_is_not_found_over_http() -> TestResult {
    each_database("cfg_06_http_project", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let mut bob = ada.invite("bob@example.test", "Bob").await?;
        let admin = rig
            .sign_in(ADA_EMAIL, super::rig::PASSWORD)
            .await?
            .session_token;
        let acme = rig
            .api()
            .create_project(
                Rig::info(),
                bb_app::projects::CreateProject {
                    session_token: admin,
                    parent: None,
                    short_name: "ACME".to_owned(),
                    name: "Acme".to_owned(),
                },
            )
            .await?
            .into_value()
            .id;
        // The administrator lists the settings of a project: none is set on a project yet.
        let listed = ada
            .send("GET", &format!("/api/v1/projects/{acme}/settings"), None)
            .await?;
        assert_eq!(listed.status, StatusCode::OK);
        assert_eq!(listed.header("cache-control"), "no-store");
        assert!(listed.body["settings"].is_array());
        // Bob cannot see it: the answer is the one for a project that does not exist.
        let hidden = bob
            .send("GET", &format!("/api/v1/projects/{acme}/settings"), None)
            .await?;
        let missing = bob
            .send(
                "GET",
                "/api/v1/projects/0197a5e2-7c1f-7b0a-9a3e-2f6d1c8b4a10/settings",
                None,
            )
            .await?;
        assert_eq!(
            (hidden.status, hidden.code()),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        assert_eq!(
            (missing.status, missing.code()),
            (hidden.status, hidden.code())
        );
        let not_an_id = bob
            .send("GET", "/api/v1/projects/nothing/settings", None)
            .await?;
        assert_eq!(not_an_id.status, StatusCode::NOT_FOUND);
        let working_in = bob
            .send("GET", &format!("/api/v1/me/settings?project={acme}"), None)
            .await?;
        assert_eq!(working_in.status, StatusCode::NOT_FOUND);
        // A project can be named by its short name as well, with the same answers.
        let by_name = ada
            .send("GET", "/api/v1/projects/ACME/settings", None)
            .await?;
        assert_eq!(by_name.status, StatusCode::OK);
        let hidden_by_name = bob
            .send("GET", "/api/v1/projects/ACME/settings", None)
            .await?;
        assert_eq!(
            (hidden_by_name.status, hidden_by_name.code()),
            (hidden.status, hidden.code())
        );
        let working_in_by_name = bob
            .send("GET", "/api/v1/me/settings?project=ACME", None)
            .await?;
        assert_eq!(working_in_by_name.status, StatusCode::NOT_FOUND);
        Ok(())
    })
    .await
}
