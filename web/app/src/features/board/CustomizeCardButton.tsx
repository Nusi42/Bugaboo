import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { layoutsAddress, paths } from "../../shared/paths.ts";
import { mayOpen } from "../../shared/places.ts";
import { usePermissions } from "../../shared/use-me.ts";

/** *Customize › Card*: the way to the layout *Card*, for people who may open the layouts (UX §4.6). */
export function CustomizeCardButton() {
  const permissions = usePermissions();
  const navigate = useNavigate();
  if (!mayOpen(paths.layouts, permissions)) {
    return null;
  }
  return (
    <Button
      look={buttonLook.quiet}
      onPress={() => {
        void navigate({ href: layoutsAddress(undefined, { form: "card" }) });
      }}
    >
      {t("board.customize-card")}
    </Button>
  );
}
