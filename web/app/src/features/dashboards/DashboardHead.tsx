import { t } from "@bugaboo/messages";
import { Card, cardLook, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { personName } from "../../shared/person.ts";
import { PinButton, pinKind } from "../pins/index.ts";
import { DashboardShare } from "./DashboardShare.tsx";
import type { Dashboard } from "./dashboards.ts";
import { MakerMenu, SaveCopy } from "./HeadMenus.tsx";
import { RenameDialog } from "./RenameDialog.tsx";

/** What can be done with the dashboard: *Pin to sidebar*, and for its maker *Share* and *More*, for others *Save a copy*. */
function HeadActions({ dashboard }: { readonly dashboard: Dashboard }) {
  const [renaming, setRenaming] = useState(false);
  return (
    <div {...partOf(part.dashboardActions)}>
      <PinButton kind={pinKind.dashboard} target={dashboard.id} />
      {dashboard.mine ? (
        <>
          <DashboardShare dashboard={dashboard} />
          <MakerMenu
            dashboard={dashboard}
            onRename={() => {
              setRenaming(true);
            }}
          />
          <RenameDialog dashboard={dashboard} isOpen={renaming} onOpenChange={setRenaming} />
        </>
      ) : (
        <SaveCopy dashboard={dashboard} />
      )}
    </div>
  );
}

/**
 * The head of a dashboard (UX §4.10): its name, who made a shared one (as *Who sees people* lets the person see them),
 * and what can be done with it.
 */
export function DashboardHead({ dashboard }: { readonly dashboard: Dashboard }) {
  return (
    <Card
      heading={dashboard.name}
      level={1}
      look={cardLook.pageHead}
      actions={<HeadActions dashboard={dashboard} />}
      intro={
        dashboard.mine ? undefined : t("search.made-by", { person: personName(dashboard.maker) })
      }
    />
  );
}
