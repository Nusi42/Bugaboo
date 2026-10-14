import { partOf, part } from "@bugaboo/ui";

import { sortingKind } from "./change-preview.ts";
import { CombineInto } from "./CombineInto.tsx";
import type { TagLine } from "./labels.ts";
import { TagForm } from "./TagForm.tsx";

export interface EditTagProps {
  readonly tag: TagLine;
  /** The other tags, which this one can be combined into. */
  readonly others: readonly TagLine[];
}

/**
 * The editor of a tag (TAG-03): its name, color and the projects it is for; renaming shows what it
 * does to the issues first; and *Combine into another*.
 */
export function EditTag({ tag, others }: EditTagProps) {
  return (
    <div {...partOf(part.stack)}>
      <TagForm tag={tag} />
      <CombineInto
        kind={sortingKind.tag}
        id={tag.id}
        name={tag.name}
        version={tag.version}
        others={others.filter((each) => each.id !== tag.id)}
      />
    </div>
  );
}
