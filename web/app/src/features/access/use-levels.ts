import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../shared/use-optimistic.ts";
import {
  applied,
  isLevelTable,
  type Change,
  type FieldKeyCode,
  type FieldLevelCode,
  type LevelTable,
  type PeopleKindKey,
  type PeopleLevelCode,
} from "./levels.ts";

/** What a screen of one table needs: the table, why it is not there, and the changes that show at once. */
export interface LevelsOf<Subject extends string, Level extends string> {
  readonly table: LevelTable<Subject, Level> | undefined;
  readonly failed: unknown;
  readonly edits: OptimisticMutation<Change<Subject, Level>>;
}

/** The query of a place: the project (an ID), or the whole organization without one. */
function placeQuery(project: string | undefined): { readonly project?: string } {
  return project === undefined ? {} : { project };
}

/** The field permissions of the whole organization or of a project, and the changes of its cells. */
export function useFieldPermissions(
  project: string | undefined,
  issueType?: string,
): LevelsOf<FieldKeyCode, FieldLevelCode> {
  const place = {
    ...placeQuery(project),
    ...(issueType === undefined ? {} : { issue_type: issueType }),
  };
  const options = api.query.queryOptions("get", "/api/v1/field-permissions", {
    params: { query: place },
  });
  const query = useQuery(options);
  const edits = useOptimisticMutation<
    Change<FieldKeyCode, FieldLevelCode>,
    LevelTable<FieldKeyCode, FieldLevelCode>,
    void
  >({
    queryKey: options.queryKey,
    isCached: isLevelTable,
    apply: applied,
    send: async (change) => {
      await api.client.PUT("/api/v1/field-permissions/{role}/{field}", {
        params: {
          path: { role: change.role, field: change.subject },
          query: place,
        },
        body: { level: change.level },
      });
    },
  });
  return { table: query.data, failed: query.error, edits };
}

/** *Who sees people* of the whole organization or of a project, and the changes of its cells. */
export function usePeopleVisibility(
  project: string | undefined,
): LevelsOf<PeopleKindKey, PeopleLevelCode> {
  const options = api.query.queryOptions("get", "/api/v1/people-visibility", {
    params: { query: placeQuery(project) },
  });
  const query = useQuery(options);
  const edits = useOptimisticMutation<
    Change<PeopleKindKey, PeopleLevelCode>,
    LevelTable<PeopleKindKey, PeopleLevelCode>,
    void
  >({
    queryKey: options.queryKey,
    isCached: isLevelTable,
    apply: applied,
    send: async (change) => {
      await api.client.PUT("/api/v1/people-visibility/{role}/{people_kind}", {
        params: {
          path: { role: change.role, people_kind: change.subject },
          query: placeQuery(project),
        },
        body: { level: change.level },
      });
    },
  });
  return { table: query.data, failed: query.error, edits };
}
