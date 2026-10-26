//! What an own field shows each role and each issue type, in every answer and in the history (CF-06).

use super::*;

#[tokio::test]
async fn cf_06_an_own_field_follows_its_level_per_role_and_per_issue_type() -> TestResult {
    each_database("cf_06_levels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_in(&level)?, id_in(&level["choices"][1])?);
        let (in_bug, in_task) = an_issue_per_type(&site, &mut ada, &key, &major).await?;
        // A role that sees the field in general (the starting level for it) but not for Bug.
        let see = [
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::ChangeIssues,
        ];
        let mut tim = reader_of(
            &site,
            &rig,
            "tim@example.test",
            &see,
            &[
                (&key, None, FieldLevel::CanChange),
                (&key, Some(&site.kind), FieldLevel::Hidden),
            ],
        )
        .await?;
        let bug = tim.send("GET", &format!("{ISSUES}/{in_bug}"), None).await?;
        assert!(
            ok(&bug)?["issue"]["own_fields"].get(&key).is_none(),
            "hidden for Bug: {}",
            bug.body
        );
        assert!(
            ok(&bug)?["changeable"]
                .as_array()
                .is_some_and(|keys| !keys.iter().any(|each| each == &json!(key)))
        );
        let question = tim
            .send("GET", &format!("{ISSUES}/{in_task}"), None)
            .await?;
        assert_eq!(ok(&question)?["issue"]["own_fields"][&key], json!(major));
        assert!(
            ok(&question)?["changeable"]
                .as_array()
                .is_some_and(|keys| keys.iter().any(|each| each == &json!(key)))
        );
        // A change of a field that is hidden for the type is refused like one that is not there.
        let version = ok(&bug)?["issue"]["version"].clone();
        let refused = tim
            .send(
                "PATCH",
                &format!("{ISSUES}/{in_bug}"),
                Some(json!({"version": version, "fields": { &key: &major }})),
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden"),
            "{}",
            refused.body
        );
        // A role that can only see the field cannot change it.
        let mut viewer = reader_of(
            &site,
            &rig,
            "vera@example.test",
            &see,
            &[(&key, None, FieldLevel::CanSee)],
        )
        .await?;
        let seen = viewer
            .send("GET", &format!("{ISSUES}/{in_bug}"), None)
            .await?;
        assert_eq!(ok(&seen)?["issue"]["own_fields"][&key], json!(major));
        let version = ok(&seen)?["issue"]["version"].clone();
        let refused = viewer
            .send(
                "PATCH",
                &format!("{ISSUES}/{in_bug}"),
                Some(json!({"version": version, "fields": { &key: &major }})),
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden", "{}", refused.body);
        // The definition of the field is not shown to a role that sees it nowhere.
        let mut nobody = reader_of(&site, &rig, "nora@example.test", &see, &[]).await?;
        let list = nobody.send("GET", FIELDS, None).await?;
        assert_eq!(ok(&list)?["items"].as_array().map(Vec::len), Some(0));
        let one = nobody.send("GET", &format!("{FIELDS}/{key}"), None).await?;
        assert_eq!(one.code(), "resource.not-found");
        Ok(())
    })
    .await
}

/// What a role with no level for the fields reads of an issue that holds a hidden pick-one field
/// (`key`, with the value `major`) and a hidden long text (`notes`): none of it, in the issue, in
/// the list and in the definitions. The answer about the issue is returned.
async fn reads_nothing_of(
    ada: &mut Browser,
    writer: &mut Browser,
    path: &str,
    (key, major, notes): (&str, &str, &str),
) -> Result<Reply, Failure> {
    let detail = writer.send("GET", path, None).await?;
    assert!(!detail.body.to_string().contains(key), "{}", detail.body);
    assert!(!detail.body.to_string().contains(major), "{}", detail.body);
    // Neither the text nor the HTML of the long text, nor its list of rendered fields.
    for hidden in [notes, "Hidden", "<strong>words</strong>"] {
        assert!(!detail.body.to_string().contains(hidden), "{}", detail.body);
    }
    assert!(
        ok(&detail)?["own_fields_html"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{}",
        detail.body
    );
    let list = writer
        .send("GET", &format!("{}?fields=own_fields", acme_issues()), None)
        .await?;
    assert!(!list.body.to_string().contains(major), "{}", list.body);
    // The definition of the field is not there for them, and the list does not hold it.
    let one = writer.send("GET", &format!("{FIELDS}/{key}"), None).await?;
    assert_eq!(one.code(), "resource.not-found", "{}", one.body);
    let all = writer.send("GET", FIELDS, None).await?;
    assert!(!all.body.to_string().contains(key), "{}", all.body);
    let for_ada = ada.send("GET", &format!("{FIELDS}/{key}"), None).await?;
    assert_eq!(for_ada.status, StatusCode::OK, "{}", for_ada.body);
    Ok(detail)
}

/// The search is no way around a hidden field (M5-F50, SEC-MATRIX *search*): a word held only by
/// the hidden long text finds nothing, the field is not among those offered, and asking for it
/// (`notes contains …`, also to sort or group by it) is told like a word that is not there.
async fn searches_nothing_of(ada: &mut Browser, writer: &mut Browser) -> Result<(), Failure> {
    let search = |text: &str| format!("/api/v1/search/issues?q={}", text.replace(' ', "%20"));
    for text in ["notes contains words", "notes is words", "sort by notes"] {
        let hidden = writer.send("GET", &search(text), None).await?;
        let missing = writer
            .send("GET", &search(&text.replace("notes", "nonesuch")), None)
            .await?;
        assert_eq!(hidden.code(), "search.invalid", "{text}: {}", hidden.body);
        assert_eq!(
            hidden.body["mistakes"][0]["key"], missing.body["mistakes"][0]["key"],
            "{text}"
        );
    }
    let groups = writer
        .send("GET", "/api/v1/search/issues/groups?by=notes", None)
        .await?;
    assert_eq!(groups.status, StatusCode::BAD_REQUEST, "{}", groups.body);
    // A `contains` on its own does not look in own fields, so it finds nothing of the text.
    let bare = writer.send("GET", &search("contains words"), None).await?;
    assert_eq!(bare.status, StatusCode::OK, "{}", bare.body);
    assert!(!bare.body.to_string().contains("Secret"), "{}", bare.body);
    let offered = writer.send("GET", "/api/v1/search/fields", None).await?;
    assert!(
        !offered.body.to_string().contains("notes"),
        "{}",
        offered.body
    );
    // Ada, who reads the field, is offered it and may search by it.
    let hers = ada.send("GET", "/api/v1/search/fields", None).await?;
    assert!(hers.body.to_string().contains("notes"), "{}", hers.body);
    let asked = ada
        .send("GET", &search("notes contains words"), None)
        .await?;
    assert_eq!(asked.status, StatusCode::OK, "{}", asked.body);
    Ok(())
}

#[tokio::test]
async fn cf_06_a_hidden_own_field_never_leaves_in_any_answer_or_history() -> TestResult {
    each_database("cf_06_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_in(&level)?, id_in(&level["choices"][1])?);
        // A long text is sent as HTML next to the values: hidden, it is in no answer either.
        let notes = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let made = ok(&ada
            .send(
                "POST",
                &acme_issues(),
                Some(json!({
                    "issue_type": site.kind,
                    "title": "Secret",
                    "fields": { &key: &major, &notes: "Hidden **words**" },
                })),
            )
            .await?)?
        .clone();
        assert!(made["own_fields"][&notes].is_string(), "{made}");
        let issue = made["readable_id"].as_str().ok_or("no key")?.to_owned();
        let path = format!("{ISSUES}/{issue}");
        // A role that changes the title, with the own field hidden (it has no level for it).
        let mut writer = reader_of(
            &site,
            &rig,
            "wes@example.test",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
            ],
            &[("title", None, FieldLevel::CanChange)],
        )
        .await?;
        let detail = reads_nothing_of(&mut ada, &mut writer, &path, (&key, &major, &notes)).await?;
        searches_nothing_of(&mut ada, &mut writer).await?;
        // A write is refused alike for a hidden field and for one that is not there.
        let version = ok(&detail)?["issue"]["version"].clone();
        let hidden = writer
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: &major }})),
            )
            .await?;
        let missing = writer
            .send(
                "PATCH",
                &path,
                Some(json!({
                    "version": version,
                    "fields": { "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c": &major },
                })),
            )
            .await?;
        assert_eq!(
            (hidden.status, hidden.code()),
            (missing.status, missing.code())
        );
        assert_eq!(hidden.code(), "access.forbidden");
        // Ada changes the own field; the writer, with the old version, is told who changed what:
        // the field that is hidden from them is not among the fields, nor in the values now.
        let minor = id_in(&level["choices"][0])?;
        ok(&ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: &minor }})),
            )
            .await?)?;
        let stale = writer
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "title": "Renamed"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert!(!stale.body.to_string().contains(&key), "{}", stale.body);
        assert!(!stale.body.to_string().contains(&minor), "{}", stale.body);
        // The record of the change that the conflict reads from (the history of the issue) names
        // the hidden fields of the change in neither the key nor the text.
        for hidden in [&notes, "Hidden", "words"] {
            assert!(!stale.body.to_string().contains(hidden), "{}", stale.body);
        }
        // Ada reads the text and the HTML of it.
        let seen = ada.send("GET", &path, None).await?;
        assert_eq!(
            ok(&seen)?["issue"]["own_fields"][&notes],
            "Hidden **words**"
        );
        assert!(
            ok(&seen)?["own_fields_html"][0]["html"]
                .as_str()
                .is_some_and(|html| html.contains("<strong>words</strong>"))
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_06_a_lead_who_may_not_see_a_field_reads_neither_its_name_nor_its_rows() -> TestResult {
    each_database("cf_06_names", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let salary = create_field(
            &mut ada,
            json!({"name": "Salary band", "kind": "text", "applies_to": "issues"}),
        )
        .await?;
        let elsewhere = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project")?
            .to_owned();
        let only_beta = create_field(
            &mut ada,
            json!({
                "name": "Beta only",
                "kind": "text",
                "applies_to": "issues",
                "scope": {"projects": [elsewhere]},
            }),
        )
        .await?;
        let (salary_id, beta_id) = (id_in(&salary)?, id_in(&only_beta)?);
        // A lead who decides what roles see and checks access, with the salary band hidden.
        let mut lead = reader_of(
            &site,
            &rig,
            "lena@example.test",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::DecideWhatRolesSee,
                Permission::CheckAccess,
            ],
            &[
                (&salary_id, None, FieldLevel::Hidden),
                (&beta_id, None, FieldLevel::CanSee),
                ("title", None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let me = lead.send("GET", "/api/v1/auth/me", None).await?;
        let lena = id_in(ok(&me)?)?;
        // The same answers for Ada, who manages fields, do name them.
        let table = ada.send("GET", "/api/v1/field-permissions", None).await?;
        assert!(ok(&table)?.to_string().contains("Salary band"));
        for (screen, path) in [
            ("field permissions", "/api/v1/field-permissions".to_owned()),
            (
                "field permissions of a project",
                format!("/api/v1/field-permissions?project={}", site.acme),
            ),
            (
                "check access",
                format!("/api/v1/check-access/people/{lena}?project={}", site.acme),
            ),
            ("the list of fields", FIELDS.to_owned()),
        ] {
            let seen = lead.send("GET", &path, None).await?;
            assert_eq!(seen.status, StatusCode::OK, "{screen}: {}", seen.body);
            let text = seen.body.to_string();
            assert!(!text.contains("Salary band"), "{screen}: {text}");
            assert!(!text.contains(&salary_id), "{screen}: {text}");
            // A field that belongs to another project is not told either, in a project.
            if screen != "field permissions" && screen != "the list of fields" {
                assert!(!text.contains("Beta only"), "{screen}: {text}");
            }
        }
        // At the organization the lead reads the field that they may see and that sits in a
        // project they see.
        let list = lead.send("GET", FIELDS, None).await?;
        assert_eq!(ok(&list)?["items"].as_array().map(Vec::len), Some(1));
        // A field they may not see answers like one that is not there; a field they may see does not.
        let hidden = lead
            .send("GET", &format!("{FIELDS}/{salary_id}"), None)
            .await?;
        assert_eq!(hidden.code(), "resource.not-found", "{}", hidden.body);
        let hidden_used = lead
            .send("GET", &format!("{FIELDS}/{salary_id}/used-by"), None)
            .await?;
        assert!(matches!(
            hidden_used.code(),
            "resource.not-found" | "access.forbidden"
        ));
        let shown = lead
            .send("GET", &format!("{FIELDS}/{beta_id}"), None)
            .await?;
        assert_eq!(shown.status, StatusCode::OK, "{}", shown.body);
        move_preview_hides(&site, &rig, &mut ada, (&salary_id, &beta_id)).await?;
        Ok(())
    })
    .await
}

/// The move preview of a project with a sub-project: Pia's role (on the new parent) reads the
/// fields `salary` and `beta` and a field only for the sub-project; a lead who may not see the
/// salary band reads neither its name nor its row, and reads the field of the sub-project by name.
async fn move_preview_hides(
    site: &Site<'_>,
    rig: &Rig,
    ada: &mut Browser,
    (salary_id, beta_id): (&str, &str),
) -> TestResult {
    // The move preview: a project with a sub-project (and a field that is only for the
    // sub-project) moves below ACME, where Pia has a role that reads all three fields.
    let create = |short: &'static str| json!({"short_name": short, "name": short});
    let moved = id_in(
        &ada.send("POST", PROJECTS, Some(create("MOVED")))
            .await?
            .body,
    )?;
    let sub = id_in(&ada.send("POST", PROJECTS, Some(create("MSUB"))).await?.body)?;
    let into_moved = ada
        .send(
            "POST",
            &format!("{PROJECTS}/{sub}/move-preview"),
            Some(json!({"to": moved})),
        )
        .await?;
    let version = into_moved.body["preview"].clone();
    ok(&ada
        .send(
            "POST",
            &format!("{PROJECTS}/{sub}/move"),
            Some(json!({"to": moved, "preview": version})),
        )
        .await?)?;
    let sub_only = create_field(
        ada,
        json!({
            "name": "Sub only",
            "kind": "text",
            "applies_to": "issues",
            "scope": {"projects": [sub]},
        }),
    )
    .await?;
    let sub_id = id_in(&sub_only)?;
    let pia = site.org.person().await?;
    let reads = site
        .org
        .role(
            "Pia reads",
            &[Permission::SeeProjects, Permission::SeeIssues],
        )
        .await?;
    let all_three = [
        (salary_id, None, FieldLevel::CanSee),
        (beta_id, None, FieldLevel::CanSee),
        (sub_id.as_str(), None, FieldLevel::CanSee),
    ];
    set_levels(site, rig, reads.id, &all_three).await?;
    let acme_id: ProjectId = site.acme.parse().map_err(|error| format!("{error}"))?;
    site.org.give(&pia, reads.id, acme_id).await?;
    // The lead may also move projects, and may see "Sub only" but not "Salary band".
    let mut mona = reader_of(
        site,
        rig,
        "mona@example.test",
        &[
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::MoveProjects,
        ],
        &[
            (salary_id, None, FieldLevel::Hidden),
            (&sub_id, None, FieldLevel::CanSee),
        ],
    )
    .await?;
    let ask = json!({"to": site.acme});
    let path = format!("{PROJECTS}/{moved}/move-preview");
    let whole = ada.send("POST", &path, Some(ask.clone())).await?;
    let everything = whole.body.to_string();
    assert!(everything.contains(salary_id), "{everything}");
    assert!(everything.contains("Sub only"), "{everything}");
    let seen = mona.send("POST", &path, Some(ask)).await?;
    assert_eq!(seen.status, StatusCode::OK, "{}", seen.body);
    let text = seen.body.to_string();
    // Hidden: neither the name nor the ID of the field, nor a row with its levels.
    assert!(!text.contains("Salary band"), "move preview: {text}");
    assert!(!text.contains(salary_id), "move preview: {text}");
    // Allowed: the field of the sub-project, by its name, in the sub-project's row.
    assert!(text.contains("Sub only"), "move preview: {text}");
    Ok(())
}

#[tokio::test]
async fn cf_06_an_own_field_never_starts_with_a_default_that_cannot_be_kept() -> TestResult {
    each_database("cf_06_default", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let refuse = |reply: &Reply| {
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::BAD_REQUEST, "request.invalid"),
                "{}",
                reply.body
            );
        };
        // A kind that points to people, or a long text, takes no starting value.
        for kind in ["person", "long-text"] {
            let made = ada
                .send(
                    "POST",
                    FIELDS,
                    Some(json!({
                        "name": format!("Starts {kind}"),
                        "kind": kind,
                        "applies_to": "issues",
                        "default_value": "x",
                    })),
                )
                .await?;
            refuse(&made);
        }
        // A long text cannot be made for projects at all.
        let long = ada
            .send(
                "POST",
                FIELDS,
                Some(json!({"name": "Notes", "kind": "long-text", "applies_to": "projects"})),
            )
            .await?;
        refuse(&long);
        // A starting value is used, and dropped when the rules no longer allow it.
        let field = create_field(
            &mut ada,
            json!({"name": "Region", "kind": "text", "applies_to": "issues", "default_value": "North"}),
        )
        .await?;
        let key = id_in(&field)?;
        let made = ok(&site.create(&mut ada, "Uses the default").await?)?.clone();
        assert_eq!(made["own_fields"][&key], "North");
        let tightened = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{key}"),
                Some(json!({"version": field["version"], "rules": {"max_length": 3}})),
            )
            .await?;
        assert!(ok(&tightened)?["default_value"].is_null(), "{}", tightened.body);
        let later = ok(&site.create(&mut ada, "Starts empty").await?)?.clone();
        assert!(later["own_fields"].get(&key).is_none());
        Ok(())
    })
    .await
}
