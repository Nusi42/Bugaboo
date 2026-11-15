import { t } from "@bugaboo/messages";
import { HoldWords } from "@bugaboo/ui";
import { Outlet, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { RouterProvider as LinkRouter } from "react-aria-components";

import { noteResolved } from "../shared/came-from.ts";

/** The root: links of React Aria navigate inside the app instead of loading a page. */
export function Root() {
  const router = useRouter();
  // Which page the person leaves, so *Back* from an issue returns to where they came from.
  useEffect(() => router.subscribe("onResolved", noteResolved), [router]);
  return (
    <LinkRouter
      navigate={(to) => {
        // A link to the page that is open takes no second step in the history: one Back leaves it.
        if (to === router.history.location.href) {
          router.history.replace(to);
        } else {
          router.history.push(to);
        }
      }}
      useHref={(to) => router.history.createHref(to)}
    >
      {/* What a screen reader hears while a screen waits for its first reads (UX §6.4). */}
      <HoldWords value={t("common.loading")}>
        <Outlet />
      </HoldWords>
    </LinkRouter>
  );
}
