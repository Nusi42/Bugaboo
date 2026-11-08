import { t } from "@bugaboo/messages";
import { Button } from "@bugaboo/ui";
import { useState } from "react";

import type { SavedSearchActionProps } from "../search/index.ts";
import { StandingRuleDialog } from "./StandingRuleDialog.tsx";

/** *Make a standing rule* beside a saved search: the window starts with that search chosen. */
export function RuleButton({ search }: SavedSearchActionProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("agents.rule-new")}
      </Button>
      {open ? (
        <StandingRuleDialog isOpen onOpenChange={setOpen} start={{ search: search.id }} />
      ) : null}
    </>
  );
}
