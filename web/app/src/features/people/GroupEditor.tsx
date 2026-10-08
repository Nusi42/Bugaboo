import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { QueryGate } from "../shell/index.ts";
import { DeleteGroup } from "./DeleteGroup.tsx";
import { GroupMembers } from "./GroupMembers.tsx";
import { useGroup } from "./groups.ts";
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
  const shown =
    found.data !== undefined && group !== undefined
      ? { group, used: found.data.used_by }
      : undefined;
  return (
    <QueryGate
      error={found.error}
      data={shown}
      notFound={<p {...partOf(part.muted)}>{t("groups.gone")}</p>}
    >
      {(editor) => (
        <div {...partOf(part.stack)}>
          <RenameGroup key={renameKey(editor.group)} group={editor.group} />
          <p {...partOf(part.muted)}>{groupUsedText(editor.used)}</p>
          <GroupMembers group={editor.group} />
          <DeleteGroup group={editor.group} members={editor.used.members} onDeleted={onDeleted} />
        </div>
      )}
    </QueryGate>
  );
}
