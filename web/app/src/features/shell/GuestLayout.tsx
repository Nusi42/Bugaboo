import { t } from "@bugaboo/messages";
import { Outlet } from "@tanstack/react-router";

import { Frame } from "./Frame.tsx";
import styles from "./Shell.module.css";

/** The frame around the pages that nobody needs to be signed in for. */
export function GuestLayout() {
  return (
    <Frame header={<span className={styles.brand}>{t("app.name")}</span>}>
      <div className={styles.narrow}>
        <Outlet />
      </div>
    </Frame>
  );
}
