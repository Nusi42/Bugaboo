import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link, partOf, part, RowChevron, ShowMoreOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { groupAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { groupLinesOf, useGroups } from "./groups.ts";
import { CreateGroupForm } from "./CreateGroupForm.tsx";

/** The groups, each a row that opens the page of the group, as the rows of the roles open theirs. */
function GroupList() {
  const list = useGroups();
  const lines = groupLinesOf(list.data?.pages);
  return (
    <div {...partOf(part.stack)}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {list.isSuccess && lines.length === 0 ? (
        <p {...partOf(part.muted)}>{t("groups.none-yet")}</p>
      ) : (
        <ul {...partOf(part.list)} aria-label={t("groups.list")}>
          {lines.map((group) => (
            <li key={group.id} {...partOf(part.item)}>
              <span {...partOf(part.rowMain)}>
                <Link row href={groupAddress(group.id)}>
                  {group.name}
                </Link>
              </span>
              <RowChevron />
            </li>
          ))}
        </ul>
      )}
      <ShowMoreOf label={t("groups.more")} list={list} />
    </div>
  );
}

/**
 * *Organization settings › Groups* (SEC-02): the groups, each opening a page of its own like a person and a role, and
 * the form that makes a group.
 */
export function GroupsPage() {
  usePageTitle(t("nav.groups"));
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.groups")} level={1} intro={t("groups.intro")}>
        <GroupList />
      </Card>
      <Card heading={t("groups.create-title")} intro={t("groups.create-intro")}>
        <CreateGroupForm />
      </Card>
    </div>
  );
}
