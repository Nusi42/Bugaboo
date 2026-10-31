import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Card,
  LinkRow,
  partOf,
  part,
  SearchField,
  ShowMoreOf,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { foundByName } from "../../shared/found-by-name.ts";
import { groupAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { groupLinesOf, useGroups } from "./groups.ts";
import { CreateGroupForm } from "./CreateGroupForm.tsx";

/**
 * The groups, found by name, each a row that opens the page of the group, as the rows of the roles open theirs. An
 * organization can have many groups, so the list has its search box.
 */
function GroupList() {
  const [words, setWords] = useState("");
  const list = useGroups();
  const lines = groupLinesOf(list.data?.pages);
  const shown = foundByName(lines, words);
  if (list.isSuccess && lines.length === 0) {
    return <p {...partOf(part.muted)}>{t("groups.none-yet")}</p>;
  }
  return (
    <div {...partOf(part.stack)}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      <SearchField
        label={t("groups.search")}
        clearLabel={t("search.clear")}
        value={words}
        onChange={setWords}
      />
      {shown.length === 0 && lines.length > 0 ? (
        <p {...partOf(part.muted)}>{t("groups.nothing")}</p>
      ) : (
        <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={t("groups.list")}>
          {shown.map((group) => (
            <LinkRow key={group.id} href={groupAddress(group.id)}>
              {group.name}
            </LinkRow>
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
