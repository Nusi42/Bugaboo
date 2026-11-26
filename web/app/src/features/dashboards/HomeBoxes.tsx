import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, HoldWhileLoading, part, partOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";

import { useCreateDashboardWindow } from "./CreateDashboard.tsx";
import { DashboardBoxes } from "./DashboardBoxes.tsx";
import { useHomeDashboard } from "./dashboards.ts";

/** *Create a dashboard* on *Home*, beside *Customize*. */
function CreateButton() {
  const create = useCreateDashboardWindow();
  return (
    <>
      <Button isPending={create.pending} onPress={create.open}>
        {t("dashboards.create")}
      </Button>
      {create.window}
    </>
  );
}

/**
 * The boxes of *Home* (ADR-0050, decision 6; UX §4.1): the person's own dashboard below the fixed blocks, made at the
 * first visit from the blocks *Home* showed before (*My timer* and *Due soon* side by side, *My open issues* under them); *Customize* is its
 * *Change*. *Home* is never shared and has no *Save a copy*. *Create a dashboard* stands beside *Customize*, so a window
 * too low for the whole sidebar can make a first dashboard too.
 */
export function HomeBoxes() {
  const home = useHomeDashboard();
  if (home.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(home.error)}</Alert>;
  }
  if (home.data === undefined) {
    return <HoldWhileLoading />;
  }
  return (
    <div {...partOf(part.dashboard)}>
      <DashboardBoxes key={home.data.id} dashboard={home.data} also={<CreateButton />} />
    </div>
  );
}
