import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { GroupEditor } from "./GroupEditor.tsx";
import { groupLinesOf, useGroups } from "./groups.ts";
import { MakeGroupForm } from "./MakeGroupForm.tsx";
import styles from "./People.module.css";

export interface GroupsPageProps {
  /** The ID of the group that is open in the side panel, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (id: string) => void;
  readonly onClose: () => void;
}

/**
 * *Organization settings › Groups* (SEC-02): the groups as a searchable list, where one opens next
 * to it to change its name and who is in it; and the form that makes a group.
 */
export function GroupsPage({ open, onOpen, onClose }: GroupsPageProps) {
  usePageTitle(t("nav.groups"));
  const list = useGroups();
  const rows: readonly ListRow[] = groupLinesOf(list.data?.pages).map((group) => ({
    key: group.id,
    name: group.name,
    editor: <GroupEditor id={group.id} onDeleted={onClose} />,
  }));
  return (
    <div className={styles.stack}>
      <Card heading={t("nav.groups")} level={1}>
        <p className={styles.text}>{t("groups.intro")}</p>
        {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
        {list.isSuccess && rows.length === 0 ? (
          <p className={styles.muted}>{t("groups.none-yet")}</p>
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
        {list.hasNextPage ? (
          <Button
            isPending={list.isFetchingNextPage}
            onPress={() => {
              void list.fetchNextPage();
            }}
          >
            {t("groups.more")}
          </Button>
        ) : null}
      </Card>
      <Card heading={t("groups.make-title")}>
        <p className={styles.text}>{t("groups.make-intro")}</p>
        <MakeGroupForm />
      </Card>
    </div>
  );
}
