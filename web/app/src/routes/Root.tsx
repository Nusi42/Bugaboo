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
        router.history.push(to);
      }}
      useHref={(to) => router.history.createHref(to)}
    >
      <Outlet />
    </LinkRouter>
  );
}
