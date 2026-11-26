import { t } from "@bugaboo/messages";
import { Window } from "@bugaboo/ui";

import { windowLabels } from "../../shared/window-labels.ts";
import { AddBoxWindow } from "./AddBoxWindow.tsx";
import { BoxForm } from "./BoxForm.tsx";
import { bodyOf, type AskedBox, type Box, type Dashboard } from "./dashboards.ts";
import type { useBoxChanges } from "./use-box-changes.ts";

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
export function BoxWindows({
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
