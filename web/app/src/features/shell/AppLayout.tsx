import { t } from "@bugaboo/messages";
import { Alert, Button, Link } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { Outlet, useLocation, useNavigate } from "@tanstack/react-router";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { leaveSession, meOptions } from "../../shared/session.ts";
import { Frame } from "./Frame.tsx";
import styles from "./Shell.module.css";

/** The pages of the menu, by address and by the message that names them. */
const menu = [
  { href: paths.home, name: "nav.home", adminOnly: false },
  { href: paths.security, name: "nav.security", adminOnly: false },
  { href: paths.sessions, name: "nav.sessions", adminOnly: false },
  { href: paths.invitations, name: "nav.invitations", adminOnly: true },
  { href: paths.status, name: "nav.status", adminOnly: true },
] as const;

/** The frame around the pages of somebody who is signed in: the menu and *Sign out*. */
export function AppLayout() {
  const me = useQuery(meOptions()).data;
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const signOut = api.query.useMutation("post", "/api/v1/auth/sign-out", {
    onSuccess: () =>
      leaveSession(() => navigate({ to: paths.signIn, search: { reason: "signed-out" } })),
  });
  const header = (
    <>
      <span className={styles.brand}>{t("app.name")}</span>
      <nav aria-label={t("shell.menu")}>
        <ul className={styles.nav}>
          {menu
            .filter((page) => !page.adminOnly || me?.administrator === true)
            .map((page) => (
              <li key={page.href}>
                <Link href={page.href} current={pathname === page.href}>
                  {t(page.name)}
                </Link>
              </li>
            ))}
        </ul>
      </nav>
      <div className={styles.who}>
        {me === undefined ? null : (
          <span>{t("shell.signed-in-as", { name: me.display_name })}</span>
        )}
        <Button
          quiet
          isPending={signOut.isPending}
          onPress={() => {
            signOut.mutate({});
          }}
        >
          {t("nav.sign-out")}
        </Button>
      </div>
    </>
  );
  return (
    <Frame header={header}>
      {signOut.isError ? <Alert danger>{messageOf(signOut.error)}</Alert> : null}
      <Outlet />
    </Frame>
  );
}
