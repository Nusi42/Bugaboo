import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { paths } from "../../shared/paths.ts";
import { statusOptions } from "../../shared/session.ts";

/** The way to make an account, for the days when an administrator allows it. */
export function SignUpLink() {
  const open = useQuery(statusOptions()).data?.sign_up_open === true;
  return open ? (
    <p {...partOf(part.muted)}>
      <Link href={paths.signUp}>{t("sign-in.create-account")}</Link>
    </p>
  ) : null;
}
