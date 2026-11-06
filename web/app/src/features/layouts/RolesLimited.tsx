import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import { paths } from "../../shared/paths.ts";
import type { FormKind } from "./layouts.ts";

export interface RolesLimitedProps {
  readonly kind: FormKind;
  /** The name of the field. */
  readonly field: string;
  /** The names of the roles whose field permissions win over the layout for this field. */
  readonly roles: readonly string[];
}

/**
 * The one line under a field that names every role the field permissions keep from it (CFG-01),
 * with the link to the page where they are set; nothing when no role is kept from the field.
 */
export function RolesLimited({ kind, field, roles }: RolesLimitedProps) {
  return roles.length === 0 ? null : (
    <p {...partOf(part.layoutFieldLimited)}>
      <span {...partOf(part.issueNote)}>
        <span>
          {t(kind === "issue-page" ? "layouts.limited-page" : "layouts.limited-form", {
            roles: listText(roles),
            field,
          })}
        </span>
        <Link href={paths.fieldPermissions}>{t("layouts.limited-link")}</Link>
      </span>
    </p>
  );
}
