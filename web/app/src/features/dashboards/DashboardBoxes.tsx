import { boxesMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, part, partOf, useLayout } from "@bugaboo/ui";
import { lazyRouteComponent } from "@tanstack/react-router";
import { Suspense, useState, type ReactNode } from "react";

import { BoxGrid } from "./BoxGrid.tsx";
import type { Box, Dashboard } from "./dashboards.ts";
import { useBoxChanges } from "./use-box-changes.ts";

/**
 * *Add box* and *Change box* with their forms and previews, loaded once *Change* is on: a page that is only looked at,
 * such as *Home* at its start, loads neither.
 */
const BoxWindows = lazyRouteComponent(() => import("./BoxWindows.tsx"), "BoxWindows");

/**
 * The line of *Change*: *Change* (on *Home* *Customize*) with what the page adds to it, or while it is on, *Add box* and
 * *Done*.
 */
function ChangeLine({
  dashboard,
  changing,
  onChanging,
  onAdd,
  also,
}: {
  readonly dashboard: Dashboard;
  readonly changing: boolean;
  readonly onChanging: (on: boolean) => void;
  readonly onAdd: () => void;
  readonly also: ReactNode;
}) {
  const full = dashboard.boxes.length >= boxesMost;
  return (
    <div {...partOf(part.dashboardActions)}>
      {changing ? (
        <>
          <span {...partOf(part.muted)}>
            {dashboard.home ? t("dashboards.changing-home") : t("dashboards.changing")}
          </span>
          <Button isDisabled={full} onPress={onAdd}>
            {t("dashboards.add-box")}
          </Button>
          <Button
            look={buttonLook.primary}
            onPress={() => {
              onChanging(false);
            }}
          >
            {t("dashboards.done")}
          </Button>
          {full ? (
            <span {...partOf(part.muted)}>{t("dashboards.full", { most: boxesMost })}</span>
          ) : null}
        </>
      ) : (
        <>
          <Button onPress={onChanging.bind(null, true)}>
            {dashboard.home ? t("dashboards.customize") : t("dashboards.change")}
          </Button>
          {also}
        </>
      )}
    </div>
  );
}

export interface DashboardBoxesProps {
  readonly dashboard: Dashboard;
  /** More in the line of *Change* while it is off (on *Home* *Create a dashboard*). */
  readonly also?: ReactNode;
}

/**
 * The boxes of a dashboard with *Change* for its maker (UX §4.10; on *Home* *Customize*, §4.1): the grid, *Add box*
 * with its previews, and *Change box*. A dashboard of another person only shows its boxes.
 */
export function DashboardBoxes({ dashboard, also = null }: DashboardBoxesProps) {
  const phone = useLayout() === "phone";
  const [changing, setChanging] = useState(dashboard.mine && dashboard.boxes.length === 0);
  const [adding, setAdding] = useState(false);
  const [changingBox, setChangingBox] = useState<Box | null>(null);
  const changes = useBoxChanges(dashboard);
  return (
    <>
      {dashboard.mine ? (
        <ChangeLine
          dashboard={dashboard}
          changing={changing}
          onChanging={setChanging}
          also={also}
          onAdd={() => {
            setAdding(true);
          }}
        />
      ) : null}
      {changes.refused === null ? null : <Alert tone={alertTone.danger}>{changes.refused}</Alert>}
      {dashboard.boxes.length === 0 ? (
        <p {...partOf(part.dashboardEmpty)}>
          {dashboard.mine ? t("dashboards.empty-mine") : t("dashboards.empty")}
        </p>
      ) : (
        <BoxGrid
          dashboard={dashboard}
          changing={changing && dashboard.mine}
          phone={phone}
          onChangeBox={setChangingBox}
        />
      )}
      {changing ? (
        <Suspense fallback={null}>
          <BoxWindows
            dashboard={dashboard}
            changes={changes}
            adding={adding}
            changingBox={changingBox}
            onClose={() => {
              setAdding(false);
              setChangingBox(null);
            }}
          />
        </Suspense>
      ) : null}
    </>
  );
}
