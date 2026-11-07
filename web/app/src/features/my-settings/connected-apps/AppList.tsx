import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, SaveStatus, Skeleton, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../../shared/errors.ts";
import { saveLabels } from "../../../shared/save-labels.ts";
import { useAfterDelay } from "../../../shared/use-optimistic.ts";
import { AppRow } from "./AppRow.tsx";
import { appsOptions, type ConnectedApp } from "./connected-apps.ts";
import { useTakeBack } from "./use-take-back.ts";

/** Words of the HTML standard, not text for people. */
const column = "col";

/** The names of the columns: on a phone they are said in the cells and the head is not shown. */
function Head() {
  return (
    <thead {...partOf(part.appsHead)}>
      <tr {...partOf(part.appsRow)}>
        {(
          [
            "connected-apps.col.app",
            "connected-apps.col.may",
            "connected-apps.col.projects",
            "connected-apps.col.allowed",
            "connected-apps.col.last-used",
          ] as const
        ).map((key) => (
          <th key={key} scope={column} {...partOf(part.appsCell, { variants: ["head"] })}>
            {t(key)}
          </th>
        ))}
        <th scope={column} {...partOf(part.appsCell, { variants: ["head"] })}>
          <span {...partOf(part.visuallyHidden)}>{t("connected-apps.col.actions")}</span>
        </th>
      </tr>
    </thead>
  );
}

/** *Your connected apps*: a table with each app, what it may do and where, and a way to take it back. */
export function AppList() {
  const apps = useQuery(appsOptions());
  const shapes = useAfterDelay(apps.isPending);
  const takeBack = useTakeBack();
  const items: readonly ConnectedApp[] = apps.data?.items ?? [];
  return (
    <Card heading={t("connected-apps.list")}>
      {apps.isError ? <Alert tone={alertTone.danger}>{messageOf(apps.error)}</Alert> : null}
      <div aria-busy={apps.isPending}>
        {apps.isSuccess && items.length === 0 ? (
          <p {...partOf(part.muted)}>{t("connected-apps.empty")}</p>
        ) : null}
        {items.length === 0 ? null : (
          <table {...partOf(part.appsTable)} aria-label={t("connected-apps.list")}>
            <Head />
            <tbody>
              {items.map((item) => (
                <AppRow
                  key={item.id}
                  item={item}
                  onTakeBack={() => {
                    takeBack.remove({ id: item.id ?? "" });
                  }}
                />
              ))}
            </tbody>
          </table>
        )}
        {shapes ? <Skeleton /> : null}
      </div>
      <SaveStatus state={takeBack.saveState} labels={saveLabels()} />
      {takeBack.problem === null ? null : <Alert tone={alertTone.danger}>{takeBack.problem}</Alert>}
      {takeBack.saveState === "saved" ? (
        <Alert tone={alertTone.success}>{t("connected-apps.taken-back")}</Alert>
      ) : null}
    </Card>
  );
}
