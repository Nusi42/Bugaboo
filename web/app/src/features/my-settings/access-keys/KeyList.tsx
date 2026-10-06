import { t } from "@bugaboo/messages";
import { Alert, Card, SaveStatus, Skeleton } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../../shared/errors.ts";
import { useAfterDelay } from "../../../shared/use-optimistic.ts";
import styles from "./AccessKeys.module.css";
import { keysOptions, type AccessKey } from "./access-keys.ts";
import { KeyRow } from "./KeyRow.tsx";
import { useDeleteKey } from "./use-delete-key.ts";

/** *Your access keys*: each with what it may do and where, and a way to delete it. */
export function KeyList() {
  const keys = useQuery(keysOptions());
  const shapes = useAfterDelay(keys.isPending);
  const remove = useDeleteKey();
  const items: readonly AccessKey[] = keys.data?.items ?? [];
  return (
    <Card heading={t("access-keys.list")}>
      {keys.isError ? <Alert danger>{messageOf(keys.error)}</Alert> : null}
      <div aria-busy={keys.isPending}>
        {keys.isSuccess && items.length === 0 ? (
          <p className={styles.muted}>{t("access-keys.empty")}</p>
        ) : null}
        <ul className={styles.list} aria-label={t("access-keys.list")}>
          {items.map((item) => (
            <KeyRow
              key={item.id}
              item={item}
              onDelete={() => {
                remove.remove({ id: item.id ?? "" });
              }}
            />
          ))}
        </ul>
        {shapes ? <Skeleton /> : null}
      </div>
      <SaveStatus
        state={remove.saveState}
        labels={{
          saving: t("save.saving"),
          saved: t("save.saved"),
          "not-saved": t("save.not-saved"),
        }}
      />
      {remove.problem === null ? null : <Alert danger>{remove.problem}</Alert>}
      {remove.saveState === "saved" ? <Alert success>{t("access-keys.deleted")}</Alert> : null}
    </Card>
  );
}
