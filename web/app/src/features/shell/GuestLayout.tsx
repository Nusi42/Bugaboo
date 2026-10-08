import { t } from "@bugaboo/messages";
import { Outlet } from "@tanstack/react-router";
import { partOf, part } from "@bugaboo/ui";

import { Frame } from "./Frame.tsx";

/** The frame around the pages that nobody needs to be signed in for. */
export function GuestLayout() {
  return (
    <Frame header={<span {...partOf(part.shellBrand)}>{t("app.name")}</span>}>
      <div {...partOf(part.shellNarrow)}>
        <Outlet />
      </div>
    </Frame>
  );
}
