//! The stale saves of label groups, labels and tags: the rows of the table of `conflict_http` for these routes.

use bb_testing::Failure;
use serde_json::json;

use super::{Browser, Save, made, read};
use crate::sign_in_tests::issues_site::{ISSUES, Site, ok, text_in};

pub(super) async fn prepare(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "changeLabelGroup" => {
            let group = made(
                ada,
                "/api/v1/label-groups",
                json!({"name": "Area", "pick_one": false}),
            )
            .await?;
            Save::new(
                "PATCH",
                format!("/api/v1/label-groups/{}", text_in(&group, "id")?),
                json!({"name": "Part"}),
                &group["version"],
            )
        }
        "changeLabelGroupOffers" => {
            let group = made(
                ada,
                "/api/v1/label-groups",
                json!({"name": "Platform", "pick_one": false}),
            )
            .await?;
            Save::new(
                "PUT",
                format!("/api/v1/label-groups/{}/offers", text_in(&group, "id")?),
                json!({"changes": [{"offered": true}]}),
                &group["version"],
            )
        }
        "changeLabel" | "combineLabels" => {
            let label = made(
                ada,
                "/api/v1/labels",
                json!({"name": format!("ui for {id}"), "label_group": null}),
            )
            .await?;
            let path = format!("/api/v1/labels/{}", text_in(&label, "id")?);
            if id == "changeLabel" {
                return Save::new("PATCH", path, json!({"name": "screen"}), &label["version"]);
            }
            let other = made(
                ada,
                "/api/v1/labels",
                json!({"name": format!("web for {id}"), "label_group": null}),
            )
            .await?;
            Save::new(
                "POST",
                format!("{path}/combine"),
                json!({"into": text_in(&other, "id")?, "into_version": other["version"]}),
                &label["version"],
            )
        }
        "changeTag" | "combineTags" => {
            let issue = ok(&site.create(ada, "Tagged").await?)?.clone();
            for name in ["slow", "fast"] {
                made(
                    ada,
                    &format!("{ISSUES}/{}/tags", text_in(&issue, "readable_id")?),
                    json!({"name": name}),
                )
                .await?;
            }
            let list = read(ada, "/api/v1/tags").await?;
            let find = |name: &str| {
                list["items"]
                    .as_array()
                    .and_then(|items| items.iter().find(|tag| tag["name"] == name))
                    .cloned()
                    .ok_or_else(|| Failure::from(format!("no tag {name} in {list}")))
            };
            let (slow, fast) = (find("slow")?, find("fast")?);
            let path = format!("/api/v1/tags/{}", text_in(&slow, "id")?);
            if id == "changeTag" {
                Save::new("PATCH", path, json!({"name": "sluggish"}), &slow["version"])
            } else {
                Save::new(
                    "POST",
                    format!("{path}/combine"),
                    json!({"into": text_in(&fast, "id")?, "into_version": fast["version"]}),
                    &slow["version"],
                )
            }
        }
        other => Err(format!("no row for {other}").into()),
    }
}
