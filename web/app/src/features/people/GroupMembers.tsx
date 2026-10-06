import { t } from "@bugaboo/messages";
import { Alert, Button } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { membersOf, useMembers, type GroupLine } from "./groups.ts";
import { MemberPicker } from "./MemberPicker.tsx";
import styles from "./People.module.css";
import { useMemberChanges, type MemberChanges } from "./use-member-changes.ts";

interface ListProps {
  readonly group: GroupLine;
  readonly changes: MemberChanges;
}

/** The people in the group, a page at a time, each with a way to take them out. */
function MemberList({ group, changes }: ListProps) {
  const members = useMembers(group.id);
  const people = membersOf(members.data?.pages);
  return (
    <div className={styles.stack}>
      {members.isError ? <Alert danger>{messageOf(members.error)}</Alert> : null}
      {members.isSuccess && people.length === 0 ? (
        <p className={styles.muted}>{t("groups.members-none")}</p>
      ) : (
        <ul className={styles.list} aria-label={t("groups.members")}>
          {people.map((person) => (
            <li key={person.id} className={styles.item}>
              <span className={styles.itemTitle}>{person.name}</span>
              <Button
                aria-label={t("groups.remove-name", { name: person.name })}
                onPress={() => {
                  changes.remove(person);
                }}
              >
                {t("groups.remove")}
              </Button>
            </li>
          ))}
        </ul>
      )}
      {members.hasNextPage ? (
        <div className={styles.row}>
          <Button
            isPending={members.isFetchingNextPage}
            onPress={() => {
              void members.fetchNextPage();
            }}
          >
            {t("groups.members-more")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

/** The people in the group, and the picker that puts more of them in; changes offer *Undo*. */
export function GroupMembers({ group }: { readonly group: GroupLine }) {
  const changes = useMemberChanges(group);
  return (
    <>
      <MemberList group={group} changes={changes} />
      {changes.problem === null ? null : <Alert danger>{changes.problem}</Alert>}
      <MemberPicker group={group} changes={changes} />
    </>
  );
}
