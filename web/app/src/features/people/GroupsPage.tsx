import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part, ShowMoreOf, useLayout } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { GroupEditor } from "./GroupEditor.tsx";
import { useGroupTrail } from "./group-trail.ts";
import { groupLinesOf, useGroup, useGroups } from "./groups.ts";
import { CreateGroupForm } from "./CreateGroupForm.tsx";

export interface GroupsPageProps {
  /** The ID of the group that is open in the side panel, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (id: string) => void;
  readonly onClose: () => void;
}

/** On a phone one group is a screen of its own, like a person: its name in the bar, and Back to the list. */
function GroupScreen({ id, onGone }: { readonly id: string; readonly onGone: () => void }) {
  const name = useGroup(id).data?.group.name;
  useGroupTrail(name);
  return (
    <Card heading={name ?? t("nav.groups")} level={1}>
      <GroupEditor id={id} onDeleted={onGone} />
    </Card>
  );
}

/**
 * *Organization settings › Groups* (SEC-02): the groups as a searchable list, where one opens next
 * to it to change its name and who is in it; and the form that makes a group.
 */
export function GroupsPage({ open, onOpen, onClose }: GroupsPageProps) {
  usePageTitle(t("nav.groups"));
  const list = useGroups();
  const phone = useLayout() === "phone";
  const rows: readonly ListRow[] = groupLinesOf(list.data?.pages).map((group) => ({
    key: group.id,
    name: group.name,
    editor: <GroupEditor id={group.id} onDeleted={onClose} />,
  }));
  if (phone && open !== undefined) {
    return <GroupScreen id={open} onGone={onClose} />;
  }
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.groups")} level={1}>
        <p {...partOf(part.text)}>{t("groups.intro")}</p>
        {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
        {list.isSuccess && rows.length === 0 ? (
          <p {...partOf(part.muted)}>{t("groups.none-yet")}</p>
        ) : (
          <ListEditor
            rows={rows}
            searchLabel={t("groups.search")}
            nothing={t("groups.nothing")}
            openKey={open}
            onOpen={onOpen}
            onClose={onClose}
          />
        )}
        <ShowMoreOf label={t("groups.more")} list={list} />
      </Card>
      <Card heading={t("groups.create-title")}>
        <p {...partOf(part.text)}>{t("groups.create-intro")}</p>
        <CreateGroupForm />
      </Card>
    </div>
  );
}
