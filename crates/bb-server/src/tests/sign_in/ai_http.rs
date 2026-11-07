//! The setup of AI over HTTP on both databases (AI-01, AI-02, PRIV-08, COMP-08, SEC-16): off at the
//! start, a key that is stored encrypted and never sent back, the agreement kept with who and when,
//! the password asked again, and a provider that fails without telling the person anything.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::ai::AiStore as _;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::Site;
use super::rig::{OTHER_PASSWORD, PASSWORD};

const AI: &str = "/api/v1/ai";

#[tokio::test]
async fn ai_01_the_key_is_stored_as_a_secret_and_never_read_back() -> TestResult {
    each_database("ai_01_key_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let key = "a-made-up-ai-key-42";
        let body = json!({"key": key, "password": PASSWORD});
        let denied = bob
            .send("PUT", "/api/v1/ai/key", Some(body.clone()))
            .await?;
        assert_eq!(denied.code(), "access.forbidden");
        let unknown = bob.send("GET", AI, None).await?;
        assert_eq!(unknown.code(), "access.forbidden");
        let wrong = ada
            .send(
                "PUT",
                "/api/v1/ai/key",
                Some(json!({"key": key, "password": OTHER_PASSWORD})),
            )
            .await?;
        assert_eq!(wrong.code(), "auth.wrong-credentials");
        assert!(rig.storage.ai_key().await?.is_none());

        let set = ada.send("PUT", "/api/v1/ai/key", Some(body)).await?;
        assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
        let shown = ada.send("GET", AI, None).await?;
        assert_eq!(shown.body["key_set"], json!(true));
        for path in [
            AI,
            "/api/v1/ai/what-goes-where",
            "/api/v1/settings",
            "/api/v1/admin/status",
        ] {
            let page = ada.send("GET", path, None).await?;
            assert!(!page.body.to_string().contains(key), "{path}");
        }
        // The key comes back from the store as it was put in, and it is stored as a secret under
        // the master key: the names of the secrets hold it, the rows do not hold it in the clear.
        let stored = rig.storage.ai_key().await?.ok_or("no key")?;
        assert_eq!(stored.as_slice(), key.as_bytes());
        assert!(
            rig.database
                .secret_names()
                .await?
                .iter()
                .any(|name| name.as_str() == bb_core::ai::KEY_SECRET)
        );

        let removed = ada
            .send("PUT", "/api/v1/ai/key", Some(json!({"password": PASSWORD})))
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT);
        assert!(rig.storage.ai_key().await?.is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_08_the_agreement_is_kept_with_who_and_when_and_a_new_provider_asks_again()
-> TestResult {
    each_database("priv_08_agreement_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let start = ada.send("GET", AI, None).await?;
        assert_eq!(start.body["enabled"], json!(false));
        assert_eq!(start.body["provider"], json!("anthropic"));
        assert_eq!(start.body["agreement_status"], json!("needed"));

        let model = ada
            .send(
                "PUT",
                AI,
                Some(json!({"model": "claude-test", "enabled": true})),
            )
            .await?;
        assert_eq!(model.code(), "ai.agreement-needed", "{}", model.body);

        let confirmed = ada
            .send(
                "POST",
                "/api/v1/ai/agreement",
                Some(json!({"destination": "anthropic"})),
            )
            .await?;
        assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.body);
        let agreement = &confirmed.body["agreement"];
        assert_eq!(agreement["destination"], json!("anthropic"));
        assert!(agreement["confirmed_at"].is_string());
        assert!(agreement["confirmed_by"].is_object(), "{agreement}");
        assert_eq!(confirmed.body["agreement_status"], json!("confirmed"));

        let on = ada
            .send(
                "PUT",
                AI,
                Some(json!({"model": "claude-test", "enabled": true})),
            )
            .await?;
        assert_eq!(on.status, StatusCode::OK, "{}", on.body);

        // A new address asks for the password, and the agreement does not carry over.
        let body = |password: Option<&str>| {
            json!({
                "enabled": false,
                "provider": "openai-compatible",
                "address": "https://models.example.test/v1",
                "password": password,
            })
        };
        let without = ada.send("PUT", AI, Some(body(None))).await?;
        assert_eq!(without.code(), "auth.wrong-credentials");
        let moved = ada.send("PUT", AI, Some(body(Some(PASSWORD)))).await?;
        assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
        assert_eq!(moved.body["destination"], json!("other-provider"));
        assert!(moved.body.get("agreement").is_none());
        assert_eq!(moved.body["agreement_status"], json!("needed"));
        let plain = ada
            .send(
                "PUT",
                AI,
                Some(json!({"address": "http://models.example.test/v1", "password": PASSWORD})),
            )
            .await?;
        assert_eq!(plain.code(), "ai.address-not-encrypted");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_02_a_local_model_answers_only_where_ai_is_on_and_a_failure_tells_nothing() -> TestResult
{
    each_database("ai_02_local_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        assert!(rig.ai.calls().is_empty());
        let set = ada
            .send(
                "PUT",
                AI,
                Some(json!({
                    "enabled": true,
                    "provider": "openai-compatible",
                    "address": "http://models.lan/v1",
                    "model": "local-test",
                    "password": PASSWORD,
                })),
            )
            .await?;
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        assert_eq!(set.body["destination"], json!("local-model"));
        assert_eq!(set.body["agreement_status"], json!("not-needed"));

        let tried = ada.send("POST", "/api/v1/ai/try", None).await?;
        assert_eq!(tried.status, StatusCode::NO_CONTENT, "{}", tried.body);
        let calls = rig.ai.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].address.as_deref(), Some("http://models.lan/v1"));

        rig.clock.advance(60);
        rig.ai.will_fail();
        let failed = ada.send("POST", "/api/v1/ai/try", None).await?;
        assert_eq!(failed.code(), "ai.unavailable");
        assert_eq!(failed.status, StatusCode::BAD_GATEWAY);
        let said = failed.body.to_string();
        assert!(said.contains("The AI provider did not answer"), "{said}");
        assert!(!said.contains("models.lan"), "{said}");

        let where_it_goes = ada.send("GET", "/api/v1/ai/what-goes-where", None).await?;
        assert_eq!(where_it_goes.body["destination"], json!("local-model"));
        assert_eq!(where_it_goes.body["names_sent"], json!(true));
        Ok(())
    })
    .await
}
