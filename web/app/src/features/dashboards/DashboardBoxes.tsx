import { boxesMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, part, partOf, useLayout, Window } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { windowLabels } from "../../shared/window-labels.ts";
import { AddBoxWindow } from "./AddBoxWindow.tsx";
import { BoxForm } from "./BoxForm.tsx";
import { BoxGrid } from "./BoxGrid.tsx";
import { bodyOf, type AskedBox, type Box, type Dashboard } from "./dashboards.ts";
import { useBoxChanges } from "./use-box-changes.ts";

/** *Change box*: the form of one box, saved over it. */
function ChangeBoxWindow({
  box,
  pending,
  onClose,
  onSave,
}: {
  readonly box: Box | null;
  readonly pending: boolean;
  readonly onClose: () => void;
  readonly onSave: (box: AskedBox) => void;
}) {
  return (
    <Window
      isOpen={box !== null}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("dashboards.change-box-title", { title: box?.title ?? "" })}
      labels={windowLabels()}
    >
      {box === null ? null : (
        <BoxForm
          key={box.id}
          start={bodyOf(box)}
          chooseKind={false}
          submit={t("common.save")}
          pending={pending}
          onSubmit={onSave}
        />
      )}
    </Window>
  );
}

/** *Add box* and *Change box*, each saved over the boxes the person sees. */
function BoxWindows({
  dashboard,
  changes,
  adding,
  changingBox,
  onClose,
}: {
  readonly dashboard: Dashboard;
  readonly changes: Pick<ReturnType<typeof useBoxChanges>, "pending" | "save">;
  readonly adding: boolean;
  readonly changingBox: Box | null;
  readonly onClose: () => void;
}) {
  return (
    <>
      <AddBoxWindow
        isOpen={adding}
        onClose={onClose}
        places={dashboard.boxes}
        pending={changes.pending}
        onAdd={(box) => {
          onClose();
          const boxes = [...dashboard.boxes.map((each) => bodyOf(each)), box];
          changes.save({ boxes, said: t("dashboards.added", { title: box.title }) });
        }}
      />
      <ChangeBoxWindow
        box={changingBox}
        pending={changes.pending}
        onClose={onClose}
        onSave={(box) => {
          onClose();
          const boxes = dashboard.boxes.map((each) => (each.id === box.id ? box : bodyOf(each)));
          changes.save({ boxes, said: t("dashboards.box-changed", { title: box.title }) });
        }}
      />
    </>
  );
}

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
    </>
  );
}
