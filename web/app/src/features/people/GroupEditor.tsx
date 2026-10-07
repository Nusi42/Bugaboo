import { t } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { DeleteGroup } from "./DeleteGroup.tsx";
import { GroupMembers } from "./GroupMembers.tsx";
import { useGroup } from "./groups.ts";
import styles from "./People.module.css";
import { RenameGroup } from "./RenameGroup.tsx";
import { groupUsedText } from "./used-by-text.ts";

/** A new form for each name the group has, so the field shows the name that was saved. */
function renameKey(group: { readonly id: string; readonly name: string }): string {
  return `${group.id}:${group.name}`;
}

export interface GroupEditorProps {
  readonly id: string;
  /** Called when the group is gone, so the side panel can close. */
  readonly onDeleted: () => void;
}

/**
 * The editor of one group in the side panel (SEC-02): its name, what it is used for, the people in
 * it and a way to put more in, and *Delete the group*.
 */
export function GroupEditor({ id, onDeleted }: GroupEditorProps) {
  const found = useGroup(id);
  const group = found.data?.group;
  if (found.isError) {
    return codeOf(found.error) === "resource.not_found" ? (
      <p className={styles.muted}>{t("groups.gone")}</p>
    ) : (
      <Alert danger>{messageOf(found.error)}</Alert>
    );
  }
  if (found.data === undefined || group === undefined) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  return (
    <div className={styles.stack}>
      <RenameGroup key={renameKey(group)} group={group} />
      <p className={styles.muted}>{groupUsedText(found.data.used_by)}</p>
      <GroupMembers group={group} />
      <DeleteGroup group={group} members={found.data.used_by.members} onDeleted={onDeleted} />
    </div>
  );
}
