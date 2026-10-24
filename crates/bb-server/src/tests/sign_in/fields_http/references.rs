//! Fields that point to people, groups and issues are shown for the viewer (CF-01).

use super::*;

/// Fields that point to a person, people, a group, an issue, and a long text: their IDs.
async fn reference_fields(ada: &mut Browser) -> Result<Vec<String>, Failure> {
    let mut keys = Vec::new();
    for (name, kind) in [
        ("Tester", "person"),
        ("Reviewers", "people"),
        ("Team", "group"),
        ("Duplicate of", "issue"),
        ("Notes", "long-text"),
    ] {
        let made = create_field(
            ada,
            json!({"name": name, "kind": kind, "applies_to": "issues"}),
        )
        .await?;
        keys.push(id_in(&made)?);
    }
    Ok(keys)
}

/// The ways a person in a field is refused: the ID of another kind of thing, a person who is not
/// there, and a person who cannot see the issue.
async fn person_refusals(
    ada: &mut Browser,
    rig: &Rig,
    site: &Site<'_>,
    path: &str,
    tester: &str,
    version: &Value,
    (other, me): (&Value, &str),
) -> TestResult {
    // The ID of another kind of thing is no person, and a person who is not there is no choice.
    let wrong_kind = ada
        .send(
            "PATCH",
            path,
            Some(json!({"version": version, "fields": { tester: other }})),
        )
        .await?;
    assert_eq!(
        wrong_kind.code(),
        "field.wrong-value",
        "{}",
        wrong_kind.body
    );
    let gone = format!(
        "{}{}",
        &me[..me.len() - 1],
        if me.ends_with('a') { 'b' } else { 'a' }
    );
    let nobody = ada
        .send(
            "PATCH",
            path,
            Some(json!({"version": version, "fields": { tester: gone }})),
        )
        .await?;
    assert_eq!(nobody.code(), "field.unknown-reference", "{}", nobody.body);
    // A person who has no role anywhere cannot see the issue, so they cannot be named in it.
    let outsider = rig
        .person(&site.org.admin.session, "otto@example.test")
        .await?
        .person
        .id
        .to_string();
    let unseen = ada
        .send(
            "PATCH",
            path,
            Some(json!({"version": version, "fields": { tester: outsider }})),
        )
        .await?;
    assert_eq!(unseen.code(), "field.unknown-reference", "{}", unseen.body);
    Ok(())
}

#[tokio::test]
async fn cf_01_people_groups_and_issues_are_shown_for_the_viewer_and_a_long_text_waits_for_its_lock()
-> TestResult {
    each_database("cf_01_references", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let keys = reference_fields(&mut ada).await?;
        let [tester, reviewers, team, duplicate, notes] = keys.as_slice() else {
            return Err("five fields".into());
        };
        let me = id_in(ok(&ada.send("GET", "/api/v1/auth/me", None).await?)?)?;
        let group = id_in(ok(&ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Core"})))
            .await?)?)?;
        let first = ok(&site.create(&mut ada, "The first").await?)?.clone();
        let (guest, _) = site.person(StartingRole::Guest, "gus").await?;
        let mut guest = guest;
        let second = ok(&site.create(&mut guest, "The second").await?)?.clone();
        let path = format!(
            "{ISSUES}/{}",
            second["readable_id"].as_str().ok_or("no key")?
        );
        let values = json!({
            tester: me, reviewers: [me], team: group, duplicate: first["id"],
        });
        let set = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": second["version"], "fields": values})),
            )
            .await?;
        let own = &ok(&set)?["own_fields"];
        // A person is a PersonRef, a group is its ID (and its name for those who see names), an
        // issue is its readable ID.
        assert_eq!(own[tester]["shown"], "name", "{own}");
        assert_eq!(own[reviewers][0]["shown"], "name", "{own}");
        assert_eq!(own[team]["id"], json!(group), "{own}");
        assert_eq!(own[duplicate]["readable_id"], first["readable_id"], "{own}");
        // The Guest sees only the issue they reported: the issue named in the field is not theirs
        // to see, so the field shows nothing.
        let seen = guest.send("GET", &path, None).await?;
        let theirs = &ok(&seen)?["issue"]["own_fields"];
        assert!(theirs.get(duplicate).is_none(), "{theirs}");
        let version = ok(&set)?["version"].clone();
        person_refusals(
            &mut ada,
            &rig,
            &site,
            &path,
            tester,
            &version,
            (&first["id"], &me),
        )
        .await?;
        // A long text is saved only under its lock: the change of the issue refuses it.
        let long = ada
            .send(
                "PATCH",
                &path,
                Some(json!({
                    "version": ok(&set)?["version"],
                    "fields": { notes: "Steps" },
                })),
            )
            .await?;
        assert_eq!(long.code(), "request.invalid", "{}", long.body);
        let on_project = ada
            .send(
                "PATCH",
                &format!("{PROJECTS}/{}", site.acme),
                Some(json!({"version": 1, "fields": { notes: "Steps" }})),
            )
            .await?;
        assert_eq!(
            on_project.status,
            StatusCode::BAD_REQUEST,
            "{}",
            on_project.body
        );
        Ok(())
    })
    .await
}
