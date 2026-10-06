import { useNavigate } from "@tanstack/react-router";

import { refreshAccess } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";
import { paths } from "../../shared/paths.ts";

/** Deleting a role: when it is gone, the roles are read again and the list of roles opens. */
export function useDeleteRole() {
  const navigate = useNavigate();
  return api.query.useMutation("delete", "/api/v1/roles/{role_id}", {
    onSuccess: () => {
      refreshAccess();
      void navigate({ to: paths.roles });
    },
  });
}
