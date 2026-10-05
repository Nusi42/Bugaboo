import { Outlet, useRouter } from "@tanstack/react-router";
import { RouterProvider as LinkRouter } from "react-aria-components";

/** The root: links of React Aria navigate inside the app instead of loading a page. */
export function Root() {
  const router = useRouter();
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
