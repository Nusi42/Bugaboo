import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import type { ProjectChoice } from "../fields/index.ts";
import { refreshLabels, type TagLine } from "./labels.ts";

/** What is filled in for a tag so far: its name, color and the projects it is for. */
export function useTagDraft(tag: TagLine) {
  const [name, setName] = useState(tag.name);
  const [color, setColor] = useState(tag.color);
  const [everywhere, setEverywhere] = useState(tag.wholeOrganization);
  const [projects, setProjects] = useState<readonly ProjectChoice[]>(
    tag.projects.map((each) => ({ id: each.id, name: each.name, shortName: each.short_name })),
  );
  const change = api.query.useMutation("patch", "/api/v1/tags/{tag}", {
    onSuccess: refreshLabels,
  });
  return {
    saving: change.isPending,
    saved: change.isSuccess,
    error: change.isError ? messageOf(change.error) : null,
    /** Saves the change, on the version that was looked at. */
    save: (): void => {
      change.mutate({
        params: { path: { tag: tag.id } },
        body: {
          version: tag.version,
          name: name.trim(),
          color,
          // An empty list makes the tag one for every project.
          projects: everywhere ? [] : projects.map((each) => each.id),
        },
      });
    },
    name,
    setName,
    color,
    setColor,
    everywhere,
    setEverywhere,
    projects,
    setProjects,
  };
}
