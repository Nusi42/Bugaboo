import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import type { PlainChoice } from "./choice.ts";

/** One permission of the catalogue, as the API names it. */
export type PermissionKey = Schema<"Permission">;

/** What the person has filled in so far, and how to change each part. */
export interface Draft {
  readonly name: string;
  readonly choice: PlainChoice;
  /** Single permissions instead of one of the three choices. */
  readonly single: boolean;
  readonly picked: readonly PermissionKey[];
  /** The IDs of the projects the key is limited to; none means every project. */
  readonly projects: readonly string[];
  /** The day the key runs out, when the person picked one; else the organization's default. */
  readonly day: string | null;
  readonly password: string;
  readonly setName: (name: string) => void;
  readonly setChoice: (choice: PlainChoice) => void;
  readonly setSingle: (single: boolean) => void;
  readonly pick: (permission: PermissionKey, on: boolean) => void;
  readonly limitTo: (project: string, on: boolean) => void;
  readonly setDay: (day: string) => void;
  readonly setPassword: (password: string) => void;
}

/** The permissions with one taken in or out. */
function withPermission(
  list: readonly PermissionKey[],
  item: PermissionKey,
  on: boolean,
): readonly PermissionKey[] {
  const without = list.filter((one) => one !== item);
  return on ? [...without, item] : without;
}

/** The projects with one taken in or out. */
function withProject(list: readonly string[], item: string, on: boolean): readonly string[] {
  const without = list.filter((one) => one !== item);
  return on ? [...without, item] : without;
}

/** The form of *Make an access key*, as a person fills it in. */
export function useDraft(): Draft {
  const [name, setName] = useState("");
  const [choice, setChoice] = useState<PlainChoice>("see-only");
  const [single, setSingle] = useState(false);
  const [picked, setPicked] = useState<readonly PermissionKey[]>([]);
  const [projects, setProjects] = useState<readonly string[]>([]);
  const [day, setDay] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  return {
    name,
    choice,
    single,
    picked,
    projects,
    day,
    password,
    setName,
    setChoice,
    setSingle,
    pick: (permission, on) => {
      setPicked(withPermission(picked, permission, on));
    },
    limitTo: (project, on) => {
      setProjects(withProject(projects, project, on));
    },
    setDay,
    setPassword,
  };
}

/** What the server is asked to make: the draft in the words of the API. */
export function bodyOf(draft: Draft) {
  return {
    password: draft.password,
    name: draft.name.trim(),
    choice: draft.single ? ("permissions" as const) : draft.choice,
    permissions: draft.single ? [...draft.picked] : [],
    projects: [...draft.projects],
    // The key runs out at the end of the day that was picked.
    expires_at: draft.day === null ? null : `${draft.day}T23:59:59Z`,
  };
}
