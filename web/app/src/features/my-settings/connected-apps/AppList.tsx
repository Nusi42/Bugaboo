import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Card,
  DataTable,
  DelayedSkeleton,
  SaveStatus,
  partOf,
  part,
  keepsRoom,
  useHoldWhileReading,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../../shared/errors.ts";
import { saveLabels } from "../../../shared/save-labels.ts";
import { AppRow } from "./AppRow.tsx";
import { appsOptions, type ConnectedApp } from "./connected-apps.ts";
import { useTakeBack } from "./use-take-back.ts";

/** The names of the columns: on a phone they are said in the cells and the head is not shown. */
function columns() {
  return [
    t("connected-apps.col.app"),
    t("connected-apps.col.may"),
    t("connected-apps.col.projects"),
    t("connected-apps.col.allowed"),
    t("connected-apps.col.last-used"),
    { hidden: t("connected-apps.col.actions") },
  ];
}

/** *Your connected apps*: a table with each app, what it may do and where, and a way to take it back. */
export function AppList() {
  const apps = useQuery(appsOptions());
  // The topic waits for the first read of the list with the hold it stands in (UX §6.4).
  useHoldWhileReading(apps.isLoading);
  const takeBack = useTakeBack();
  const items: readonly ConnectedApp[] = apps.data?.items ?? [];
  return (
    <Card
      actions={<SaveStatus state={takeBack.saveState} labels={saveLabels()} variant={keepsRoom} />}
      heading={t("connected-apps.list")}
    >
      {apps.isError ? <Alert tone={alertTone.danger}>{messageOf(apps.error)}</Alert> : null}
      <div aria-busy={apps.isPending}>
        {apps.isSuccess && items.length === 0 ? (
          <p {...partOf(part.muted)}>{t("connected-apps.empty")}</p>
        ) : null}
        {items.length === 0 ? null : (
          <DataTable label={t("connected-apps.list")} columns={columns()}>
            {items.map((item) => (
              <AppRow
                key={item.id}
                item={item}
                onTakeBack={() => {
                  takeBack.remove({ id: item.id ?? "" });
                }}
              />
            ))}
          </DataTable>
        )}
        {apps.isPending ? <DelayedSkeleton /> : null}
      </div>
      {takeBack.problem === null ? null : <Alert tone={alertTone.danger}>{takeBack.problem}</Alert>}
      {takeBack.saveState === "saved" ? (
        <Alert tone={alertTone.success}>{t("connected-apps.taken-back")}</Alert>
      ) : null}
    </Card>
  );
}
