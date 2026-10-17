import type { Schema } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import { personNameInside } from "../../shared/person.ts";

/** The followers by name as the kind *Followers* says for the reader; shown on request. */
export function FollowerNames({ people }: { readonly people: readonly Schema<"PersonRef">[] }) {
  return <p {...partOf(part.text)}>{listText(people.map((person) => personNameInside(person)))}</p>;
}
