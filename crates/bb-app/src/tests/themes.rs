//! Own themes (THM-03, THM-04, ADR-0026 update M8): who may make, use and change them, what a save
//! checks, their style sheets and their drafts, the limits, and the choice of a theme that is gone.

use bb_core::ErrorCode;
use bb_core::edit::DraftTarget;
use bb_core::settings::SettingValue;
use bb_core::theme::{PERSONAL_THEMES_MOST, THEME_UPLOAD_MOST, ThemeKind};
use serde_json::json;

use super::scenarios::token;
use super::scenarios_searches::with_role;
use super::scenarios_themes::{a_theme, an_accent, lock_the_theme_for, save_values, white_text};
use super::world::{Outcome, World};
use crate::access::StartingRole;
use crate::auth::AppError;
use crate::editing::{DraftText, SaveDraftQuery, save_draft};
use crate::settings::{ChangeSetting, SettingsQuery, SettingsScope, change_setting, settings};
use crate::themes::{
    CopyTheme, DeleteTheme, ThemeQuery, ThemeSheetQuery, ThemesQuery, UploadTheme, copy_theme,
    delete_theme, download_theme, editing_what, theme, theme_draft_sheet, theme_sheet, themes,
    upload_theme,
};

fn code(error: &AppError) -> Option<ErrorCode> {
    error.code()
}

#[tokio::test]
async fn thm_03_a_theme_for_everyone_with_a_pair_hard_to_read_is_refused() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::ForEveryone, "daylight").await?;
    let refused = save_values(&world, &admin, world.admin.person.id, &view, white_text())
        .await
        .err()
        .ok_or("a theme for everyone with white text on white was saved")?;
    let AppError::HardToRead(pairs) = &refused else {
        return Err(format!("not refused for the pairs: {refused}").into());
    };
    assert!(
        pairs.iter().any(|pair| pair.first == "fg"),
        "the pair of the text is named: {pairs:?}"
    );
    let stored = world.store.lock().state.own_themes[0].clone();
    assert!(stored.saved_at.is_none(), "nothing was saved");
    assert!(stored.values.is_empty());
    Ok(())
}

#[tokio::test]
async fn thm_03_a_personal_theme_with_a_pair_hard_to_read_is_saved_with_the_pairs_named() -> Outcome
{
    let world = World::new().await?;
    let person = world.person("lea@example.com").await?;
    let view = a_theme(
        &world,
        &person.session_token,
        ThemeKind::Personal,
        "daylight",
    )
    .await?;
    let saved = save_values(
        &world,
        &person.session_token,
        person.person.id,
        &view,
        white_text(),
    )
    .await?;
    assert!(saved.view.theme.saved_at.is_some());
    assert!(
        saved.hard_to_read.iter().any(|pair| pair.first == "fg"),
        "the warning names the pair of the text: {:?}",
        saved.hard_to_read
    );
    Ok(())
}

#[tokio::test]
async fn thm_04_an_own_theme_stores_only_the_values_it_changes_and_follows_its_built_in_theme()
-> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::ForEveryone, "dusk").await?;
    let saved = save_values(&world, &admin, world.admin.person.id, &view, an_accent()).await?;
    let stored = world.store.lock().state.own_themes[0].values.clone();
    assert_eq!(stored.colors.len(), 1, "one color, and nothing of the base");
    assert!(stored.shadow.is_none() && stored.radii.is_empty() && stored.fonts.is_empty());
    let sheet = theme_sheet(
        &world.ctx(),
        ThemeSheetQuery {
            session_token: admin.clone(),
            theme: view.theme.id,
        },
    )
    .await?;
    assert_eq!(sheet.version, saved.view.theme.version);
    assert!(sheet.css.contains("--accent: #b4a8ff;"));
    // On the page the sheet sets only what the theme changes; the rest is its built-in theme's.
    // (A sample of it in a picker is a block of its own, which writes its corners and faces too.)
    let page = sheet.css.split("\n\n").next().unwrap_or_default();
    assert_eq!(page.matches("  --").count(), 1, "{}", sheet.css);
    Ok(())
}

#[tokio::test]
async fn thm_04_a_value_outside_its_kind_or_an_unknown_token_is_refused_also_in_an_upload()
-> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::Personal, "dusk").await?;
    let bad = json!({
        "colors": {"accent": {"value": "red"}, "no-such-color": {"value": "#000000"}},
        "radii": {"r-card": 999},
        "spacing": {"s-1": 4},
    });
    let refused = save_values(&world, &admin, world.admin.person.id, &view, bad.clone())
        .await
        .err()
        .ok_or("a theme with a bad value was saved")?;
    let AppError::ThemeValuesRefused(found) = &refused else {
        return Err(format!("not refused for its values: {refused}").into());
    };
    let named: Vec<&str> = found.iter().map(|problem| problem.token.as_str()).collect();
    for name in ["accent", "no-such-color", "r-card", "spacing"] {
        assert!(named.contains(&name), "{name} is named: {named:?}");
    }
    // The same in a file, where a part that is not one of the file is named too.
    let file = json!({"name": "Bad", "extends": "dusk", "values": bad, "css": "body{}"});
    let upload = upload_theme(
        &world.ctx(),
        UploadTheme {
            session_token: admin.clone(),
            kind: ThemeKind::Personal,
            file: file.to_string().into_bytes(),
        },
    )
    .await
    .err()
    .ok_or("a file with a bad value was taken")?;
    let AppError::ThemeValuesRefused(found) = &upload else {
        return Err(format!("not refused for its values: {upload}").into());
    };
    assert!(found.iter().any(|problem| problem.token == "css"));
    assert!(found.iter().any(|problem| problem.token == "accent"));
    Ok(())
}

#[tokio::test]
async fn thm_04_a_personal_theme_is_served_to_its_maker_only() -> Outcome {
    let world = World::new().await?;
    let lea = world.person("lea@example.com").await?;
    let ben = world.person("ben@example.com").await?;
    let view = a_theme(&world, &lea.session_token, ThemeKind::Personal, "dusk").await?;
    save_values(
        &world,
        &lea.session_token,
        lea.person.id,
        &view,
        an_accent(),
    )
    .await?;
    let sheet_for = |session: String| ThemeSheetQuery {
        session_token: session,
        theme: view.theme.id,
    };
    assert!(
        theme_sheet(&world.ctx(), sheet_for(lea.session_token.clone()))
            .await
            .is_ok()
    );
    for session in [ben.session_token.clone(), token(&world)] {
        let refused = theme_sheet(&world.ctx(), sheet_for(session.clone()))
            .await
            .err()
            .ok_or("somebody else got a personal sheet")?;
        assert_eq!(code(&refused), Some(ErrorCode::NotFound));
        let read = theme(
            &world.ctx(),
            ThemeQuery {
                session_token: session.clone(),
                theme: view.theme.id,
            },
        )
        .await
        .err()
        .ok_or("somebody else read a personal theme")?;
        assert_eq!(code(&read), Some(ErrorCode::NotFound));
        let listed = themes(
            &world.ctx(),
            ThemesQuery {
                session_token: session,
            },
        )
        .await?
        .into_value();
        assert!(listed.themes.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn thm_04_a_draft_is_served_to_its_editor_only_and_listed_nowhere() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::ForEveryone, "dusk").await?;
    let what = editing_what(view.theme.id)?;
    // A new theme is offered as a choice to nobody and has no sheet of its own; its maker finds it
    // in the list, marked as never saved, to go on with it.
    let listed = themes(
        &world.ctx(),
        ThemesQuery {
            session_token: admin.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(listed.themes.len(), 1);
    assert!(listed.themes[0].theme.saved_at.is_none());
    let query = |session: &str| ThemeSheetQuery {
        session_token: session.to_owned(),
        theme: view.theme.id,
    };
    assert_eq!(
        code(
            &theme_sheet(&world.ctx(), query(&admin))
                .await
                .err()
                .ok_or("a theme never saved has a sheet")?
        ),
        Some(ErrorCode::NotFound)
    );
    // The editor keeps the unsaved values as the draft of the theme; the preview is made from it.
    lock_the_theme_for(&world, view.theme.id, world.admin.person.id);
    save_draft(
        &world.ctx(),
        SaveDraftQuery {
            session_token: admin.clone(),
            target: DraftTarget::Editing(what),
            draft: DraftText {
                text: an_accent().to_string(),
                base_version: Some(view.theme.version),
            },
        },
    )
    .await?;
    let draft = theme_draft_sheet(&world.ctx(), query(&admin)).await?;
    assert!(draft.css.contains("--accent: #b4a8ff;"));
    // Another administrator who may change the theme has no draft of it, and gets none.
    let other = world.person("ana@example.com").await?;
    with_role(&world, &other, StartingRole::Administrator, None).await?;
    for session in [other.session_token.clone()] {
        assert_eq!(
            code(
                &theme_draft_sheet(&world.ctx(), query(&session))
                    .await
                    .err()
                    .ok_or("somebody else got the draft")?
            ),
            Some(ErrorCode::NotFound)
        );
    }
    // They see the theme, which they may change, but its values as the draft holds them nowhere.
    let listed = themes(
        &world.ctx(),
        ThemesQuery {
            session_token: other.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(listed.themes.len(), 1, "a theme for everyone never saved");
    assert!(
        listed.themes[0].theme.values.is_empty(),
        "a draft is listed nowhere"
    );
    // While its maker edits it, nobody else takes it away.
    let refused = delete_theme(
        &world.ctx(),
        DeleteTheme {
            session_token: other.session_token,
            theme: view.theme.id,
            version: view.theme.version,
        },
    )
    .await
    .err()
    .ok_or("a theme that is being edited was taken away")?;
    assert_eq!(code(&refused), Some(ErrorCode::EditInProgress));
    Ok(())
}

#[tokio::test]
async fn thm_04_a_theme_never_saved_is_listed_to_its_maker_only_or_to_who_looks_after_themes()
-> Outcome {
    let world = World::new().await?;
    let lea = world.person("lea@example.com").await?;
    let ben = world.person("ben@example.com").await?;
    let admin = token(&world);
    let personal = a_theme(&world, &lea.session_token, ThemeKind::Personal, "dusk").await?;
    let everyone = a_theme(&world, &admin, ThemeKind::ForEveryone, "dusk").await?;
    let ids = |session: String| async {
        themes(
            &world.ctx(),
            ThemesQuery {
                session_token: session,
            },
        )
        .await
        .map(|list| {
            list.into_value()
                .themes
                .iter()
                .map(|view| view.theme.id)
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(ids(lea.session_token.clone()).await?, [personal.theme.id]);
    assert!(ids(ben.session_token.clone()).await?.is_empty());
    assert_eq!(ids(admin.clone()).await?, [everyone.theme.id]);
    // Another holder of *Change organization settings* may take it away.
    with_role(&world, &ben, StartingRole::Administrator, None).await?;
    assert_eq!(ids(ben.session_token.clone()).await?, [everyone.theme.id]);
    delete_theme(
        &world.ctx(),
        DeleteTheme {
            session_token: ben.session_token,
            theme: everyone.theme.id,
            version: everyone.theme.version,
        },
    )
    .await?;
    assert!(ids(admin).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn thm_04_a_theme_for_everyone_needs_change_organization_settings() -> Outcome {
    let world = World::new().await?;
    let lea = world.person("lea@example.com").await?;
    let made = a_theme(&world, &lea.session_token, ThemeKind::ForEveryone, "dusk")
        .await
        .err()
        .ok_or("a person without the permission made a theme for everyone")?;
    assert!(made.contains(ErrorCode::Forbidden.code()), "{made}");
    // One the administrator saved is used by everybody, but changed by nobody without the right.
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::ForEveryone, "dusk").await?;
    let saved = save_values(&world, &admin, world.admin.person.id, &view, an_accent()).await?;
    let listed = themes(
        &world.ctx(),
        ThemesQuery {
            session_token: lea.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert_eq!(listed.themes.len(), 1);
    assert!(!listed.themes[0].may_change);
    let changed = save_values(
        &world,
        &lea.session_token,
        lea.person.id,
        &saved.view,
        an_accent(),
    )
    .await
    .err()
    .ok_or("a person without the permission changed a theme for everyone")?;
    assert_eq!(code(&changed), Some(ErrorCode::Forbidden));
    let deleted = delete_theme(
        &world.ctx(),
        DeleteTheme {
            session_token: lea.session_token.clone(),
            theme: view.theme.id,
            version: saved.view.theme.version,
        },
    )
    .await
    .err()
    .ok_or("a person without the permission took a theme for everyone away")?;
    assert_eq!(code(&deleted), Some(ErrorCode::Forbidden));
    // A personal copy of it is theirs to change.
    let copy = copy_theme(
        &world.ctx(),
        CopyTheme {
            session_token: lea.session_token.clone(),
            theme: view.theme.id,
            name: "Mine".to_owned(),
            kind: ThemeKind::Personal,
        },
    )
    .await?
    .into_value();
    assert!(copy.view.may_change);
    assert_eq!(copy.view.theme.values, saved.view.theme.values);
    Ok(())
}

#[tokio::test]
async fn thm_04_a_choice_of_a_deleted_theme_counts_as_its_built_in_theme() -> Outcome {
    let world = World::new().await?;
    let lea = world.person("lea@example.com").await?;
    let view = a_theme(&world, &lea.session_token, ThemeKind::Personal, "midnight").await?;
    let saved = save_values(
        &world,
        &lea.session_token,
        lea.person.id,
        &view,
        an_accent(),
    )
    .await?;
    let dark = "appearance.dark-theme";
    let chosen = change_setting(
        &world.ctx(),
        ChangeSetting {
            session_token: lea.session_token.clone(),
            scope: SettingsScope::Person,
            key: dark.to_owned(),
            value: SettingValue::Text(saved.view.choice.clone()),
            version: None,
        },
    )
    .await?
    .into_value();
    assert_eq!(chosen.value, SettingValue::Text(saved.view.choice.clone()));
    // Nobody else may choose it, and nobody may choose it as a light theme.
    let ben = world.person("ben@example.com").await?;
    for (session, key) in [
        (ben.session_token.clone(), dark),
        (lea.session_token.clone(), "appearance.light-theme"),
    ] {
        let refused = change_setting(
            &world.ctx(),
            ChangeSetting {
                session_token: session,
                scope: SettingsScope::Person,
                key: key.to_owned(),
                value: SettingValue::Text(saved.view.choice.clone()),
                version: None,
            },
        )
        .await
        .err()
        .ok_or("a theme somebody may not use was chosen")?;
        assert_eq!(code(&refused), Some(ErrorCode::InvalidRequest));
    }
    delete_theme(
        &world.ctx(),
        DeleteTheme {
            session_token: lea.session_token.clone(),
            theme: view.theme.id,
            version: saved.view.theme.version,
        },
    )
    .await?;
    let mine = settings(
        &world.ctx(),
        SettingsQuery {
            session_token: lea.session_token,
            scope: SettingsScope::Person,
            topic: Some("appearance.".to_owned()),
            project: None,
        },
    )
    .await?
    .into_value();
    let value = mine
        .settings
        .iter()
        .find(|setting| setting.definition.key() == dark)
        .map(|setting| setting.value.clone())
        .ok_or("no dark theme")?;
    assert_eq!(value, SettingValue::Text("midnight".to_owned()));
    Ok(())
}

#[tokio::test]
async fn thm_04_the_limits_of_the_number_and_the_size_of_themes_hold() -> Outcome {
    let world = World::new().await?;
    let lea = world.person("lea@example.com").await?;
    for _ in 0..PERSONAL_THEMES_MOST {
        a_theme(&world, &lea.session_token, ThemeKind::Personal, "dusk").await?;
    }
    let refused = a_theme(&world, &lea.session_token, ThemeKind::Personal, "dusk")
        .await
        .err()
        .ok_or("a person made more personal themes than allowed")?;
    assert!(
        refused.contains(ErrorCode::PersonalThemesMost.code()),
        "{refused}"
    );
    let big = upload_theme(
        &world.ctx(),
        UploadTheme {
            session_token: token(&world),
            kind: ThemeKind::Personal,
            file: vec![b' '; THEME_UPLOAD_MOST + 1],
        },
    )
    .await
    .err()
    .ok_or("a file over the limit was taken")?;
    assert_eq!(code(&big), Some(ErrorCode::TooLarge));
    Ok(())
}

#[tokio::test]
async fn thm_04_a_downloaded_theme_uploads_as_the_same_theme() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::Personal, "paper").await?;
    let saved = save_values(&world, &admin, world.admin.person.id, &view, an_accent()).await?;
    let file = download_theme(
        &world.ctx(),
        ThemeQuery {
            session_token: admin.clone(),
            theme: view.theme.id,
        },
    )
    .await?
    .into_value();
    let bytes = serde_json::to_vec(&file).map_err(|error| error.to_string())?;
    let again = upload_theme(
        &world.ctx(),
        UploadTheme {
            session_token: admin,
            kind: ThemeKind::Personal,
            file: bytes,
        },
    )
    .await?
    .into_value();
    assert_eq!(again.view.theme.values, saved.view.theme.values);
    assert_eq!(again.view.theme.extends.as_str(), "paper");
    assert!(again.view.theme.saved_at.is_some());
    Ok(())
}

#[tokio::test]
async fn iss_17_a_stale_save_of_a_theme_for_everyone_never_saved_names_its_version_to_who_looks_after_it()
-> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let view = a_theme(&world, &admin, ThemeKind::ForEveryone, "dusk").await?;
    let ben = world.person("ben@example.com").await?;
    with_role(&world, &ben, StartingRole::Administrator, None).await?;
    let mut stale_view = view.clone();
    stale_view.theme.version += 1;
    let refused = save_values(
        &world,
        &ben.session_token,
        ben.person.id,
        &stale_view,
        an_accent(),
    )
    .await
    .err()
    .ok_or("a save from a version that is not there went through")?;
    let AppError::Stale(stale) = refused else {
        return Err(format!("not a stale save: {refused}").into());
    };
    let AppError::Conflict(conflict) = world.ctx().conflict_of(&stale).await else {
        return Err("no conflict answer".into());
    };
    assert_eq!(conflict.version, Some(view.theme.version));
    Ok(())
}
