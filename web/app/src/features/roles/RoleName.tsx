import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { NameForm } from "../../shared/NameForm.tsx";
import { refreshAccess } from "../../shared/query-keys.ts";

export interface RoleNameProps {
  readonly id: string;
  readonly name: string;
  /** The version of the role that was looked at. */
  readonly version: number;
}

/** *Rename*: the name of the role, saved with the button. A starting role can be renamed too. */
export function RoleName({ id, name, version }: RoleNameProps) {
  const rename = api.query.useMutation("patch", "/api/v1/roles/{role}", {
    onSuccess: refreshAccess,
  });
  return (
    <NameForm
      label={t("roles.field-name")}
      name={name}
      saving={rename}
      onSave={(typed) => {
        rename.mutate({ params: { path: { role: id } }, body: { name: typed, version } });
      }}
    />
  );
}
