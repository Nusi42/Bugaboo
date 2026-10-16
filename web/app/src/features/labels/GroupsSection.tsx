import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { ListEditor, type ListRow } from "../shell/index.ts";
import { EditGroup } from "./EditGroup.tsx";
import type { LabelGroup } from "./labels.ts";
import { CreateLabelGroupForm } from "./CreateLabelGroupForm.tsx";
import type { OpenProps } from "./OpenProps.ts";

/** The row of a group of labels: whether it is a group to pick one from, and whether it is retired. */
function rowOf(group: LabelGroup): ListRow {
  return {
    key: group.id,
    name: group.name,
    details: (
      <span {...partOf(part.muted)}>
        {group.pick_one ? t("labels.group-pick-one") : null}
        {group.retired ? t("labels.retired") : null}
      </span>
    ),
    editor: <EditGroup key={group.id} group={group} />,
  };
}

/** The groups of labels as a list that opens next to it, and the form that makes one. */
export function GroupsSection({
  groups,
  ...opening
}: OpenProps & { readonly groups: readonly LabelGroup[] }) {
  return (
    <Card heading={t("labels.groups-title")}>
      <ListEditor
        rows={groups.map((group) => rowOf(group))}
        searchLabel={t("labels.groups-search")}
        nothing={t("labels.groups-nothing")}
        openKey={opening.open}
        onOpen={opening.onOpen}
        onClose={opening.onClose}
      />
      <h3>{t("labels.create-label-group-title")}</h3>
      <p {...partOf(part.text)}>{t("labels.create-label-group-intro")}</p>
      <CreateLabelGroupForm />
    </Card>
  );
}
