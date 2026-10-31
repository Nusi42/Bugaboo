import { t } from "@bugaboo/messages";
import type { Schema } from "@bugaboo/api-client";
import { Card, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { NameForm } from "../../shared/NameForm.tsx";
import { refreshAccess } from "../../shared/query-keys.ts";
import { paths } from "../../shared/paths.ts";
import type { GroupLine } from "./groups.ts";
import { groupUsedText } from "./used-by-text.ts";

/** *Rename*: the name of the group, saved with the button. */
export function RenameGroup({ group }: { readonly group: GroupLine }) {
  const rename = api.query.useMutation("patch", "/api/v1/groups/{group}", {
    onSuccess: refreshAccess,
  });
  return (
    <NameForm
      label={t("groups.field-name")}
      name={group.name}
      saving={rename}
      onSave={(name) => {
        rename.mutate({
          params: { path: { group: group.id } },
          body: { name, version: group.version },
        });
      }}
    />
  );
}

/** A new form for each name the group has, so the field shows the name that was saved. */
function renameKey(group: GroupLine): string {
  return `${group.id}:${group.name}`;
}

/** The first card of the page of a group: its name as the title, what it is used for, *Rename*, and *All groups*. */
export function GroupNameCard({
  group,
  used,
}: {
  readonly group: GroupLine;
  readonly used: Schema<"GroupUsedByBody">;
}) {
  return (
    <Card heading={group.name} level={1} back={{ href: paths.groups, label: t("groups.back") }}>
      <p {...partOf(part.muted)}>{groupUsedText(used)}</p>
      <RenameGroup key={renameKey(group)} group={group} />
    </Card>
  );
}
