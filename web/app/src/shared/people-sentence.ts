import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { roleName, someoneName } from "./person.ts";

/** A level of *Who sees people*, as the API words it. */
type PeopleLevelCode = Schema<"PeopleLevel">;

/** The example of a person shown by their role only, in the words of the glossary: *someone (Team member)*. */
export function exampleOfRoleOnly(): string {
  return roleName("inside", t("starting-role.team-member"));
}

/** What a role sees of people in the kinds at a level, in a sentence of the preview (the words of the glossary). */
export function peopleSentence(level: PeopleLevelCode, kinds: string): string {
  if (level === "names") {
    return t("people-visibility.preview-names", { kinds });
  }
  if (level === "role-only") {
    return t("people-visibility.preview-role-only", { kinds, example: exampleOfRoleOnly() });
  }
  return t("people-visibility.preview-hidden", { kinds, someone: someoneName("inside") });
}
