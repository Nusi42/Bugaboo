import { t } from "@bugaboo/messages";
import { Link } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { paths } from "../../shared/paths.ts";
import { statusOptions } from "../../shared/session.ts";
import styles from "./SignIn.module.css";

/** The way to make an account, for the days when an administrator allows it. */
export function SignUpLink() {
  const open = useQuery(statusOptions()).data?.sign_up_open === true;
  return open ? (
    <p className={styles.hint}>
      <Link href={paths.signUp}>{t("sign-in.make-account")}</Link>
    </p>
  ) : null;
}
