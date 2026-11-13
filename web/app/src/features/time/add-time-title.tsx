import { t } from "@bugaboo/messages";

import type { EntryIssue } from "./entry-issue.ts";
import { IdsIn } from "./IdsIn.tsx";

/** The title of *Add time*: the sentence with the issue's ID drawn as an ID. */
export function AddTimeTitle({ issue }: { readonly issue: EntryIssue }) {
  return <IdsIn sentence={t("time.add.title", { issue: issue.key })} ids={[issue.key]} />;
}
