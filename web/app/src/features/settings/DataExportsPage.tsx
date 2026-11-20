import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  Card,
  cardLook,
  DataTable,
  ShowMoreOf,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { useMe } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { useDataExports } from "./data-exports.ts";
import { ExportRow } from "./ExportRow.tsx";
import { SomeonesDataWindow } from "./SomeonesDataWindow.tsx";

/** The names of the columns: on a phone they are said in the cells and the head is not shown. */
function columns() {
  return [
    t("data-exports.col.person"),
    t("data-exports.col.asked-by"),
    t("data-exports.col.reason"),
    t("data-exports.col.asked-at"),
    t("data-exports.col.file"),
    t("data-exports.col.deleted-at"),
    { hidden: t("data-exports.col.actions") },
  ];
}

/** The files, newest first, a page at a time. */
function ExportList() {
  const found = useDataExports();
  const me = useMe()?.id;
  const items = itemsOf(found.data?.pages);
  return (
    <Card heading={t("data-exports.list")}>
      {found.error === null ? null : (
        <Alert tone={alertTone.danger}>{messageOf(found.error)}</Alert>
      )}
      {found.isSuccess && items.length === 0 ? (
        <p {...partOf(part.muted)}>{t("data-exports.none")}</p>
      ) : null}
      {items.length === 0 ? null : (
        <DataTable label={t("data-exports.list")} columns={columns()}>
          {items.map((item) => (
            <ExportRow
              key={item.id}
              item={item}
              mine={item.asked_by.shown === "name" && item.asked_by.id === me}
            />
          ))}
        </DataTable>
      )}
      <ShowMoreOf label={t("data-exports.more")} list={found} />
    </Card>
  );
}

/**
 * *Organization settings › Privacy › Data exports* (PRIV-03, ADR-0044, decision 4): who made which file of whose data,
 * when and why, newest first, for a holder of *Download someone's data*, with *Download someone's data* itself.
 */
export function DataExportsPage() {
  usePageTitle(t("nav.data-exports"));
  const [asking, setAsking] = useState(false);
  return (
    <>
      <Card
        heading={t("nav.data-exports")}
        level={1}
        look={cardLook.pageHead}
        intro={t("data-exports.intro")}
        actions={
          <Button
            onPress={() => {
              setAsking(true);
            }}
          >
            {t("data-exports.someone")}
          </Button>
        }
      />
      <ExportList />
      {asking ? (
        <SomeonesDataWindow
          onClose={() => {
            setAsking(false);
          }}
        />
      ) : null}
    </>
  );
}
