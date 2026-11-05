import type { FieldValues } from "./fields.ts";
import { api } from "../../shared/api.ts";
import { useQueries } from "@tanstack/react-query";

/**
 * The value each field gets from the projects above, the nearest wins: read from the details of
 * the projects of the trail, given by their short names from the top down. A project the person
 * cannot see is not in the trail and gives nothing to show (the booking still uses it).
 */
export function useValuesFromAbove(shortNames: readonly string[]): FieldValues {
  const above = useQueries({
    queries: shortNames.map((project) =>
      api.query.queryOptions("get", "/api/v1/projects/{project}", {
        params: { path: { project } },
      }),
    ),
  });
  // The trail runs from the top down, so a nearer project overrides a farther one.
  const values: Record<string, unknown> = {};
  for (const one of above) {
    Object.assign(values, one.data?.own_fields);
  }
  return values;
}
