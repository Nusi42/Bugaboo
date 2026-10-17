import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";

/** The two kinds of thing that are renamed and combined the same way. */
export const labelsAndTagsKind = { tag: "tag", label: "label" } as const;

/** A tag or a label. */
export type LabelsAndTagsKind = (typeof labelsAndTagsKind)[keyof typeof labelsAndTagsKind];

/** What is asked of the preview: a new name, or the one to combine into. */
export type ChangeAsked = Schema<"PreviewChangeBody">;

/** The preview of renaming or combining a tag or label, as it is asked for and as it comes. */
export function useChangePreview(kind: LabelsAndTagsKind) {
  const [preview, setPreview] = useState<Schema<"ChangePreviewBody"> | null>(null);
  const tag = api.query.useMutation("post", "/api/v1/tags/{tag}/preview");
  const label = api.query.useMutation("post", "/api/v1/labels/{label}/preview");
  const asked = kind === labelsAndTagsKind.tag ? tag : label;
  return {
    preview,
    pending: asked.isPending,
    error: asked.isError ? messageOf(asked.error) : null,
    forget: () => {
      setPreview(null);
    },
    ask: (id: string, body: ChangeAsked) => {
      const options = { onSuccess: setPreview };
      if (kind === labelsAndTagsKind.tag) {
        tag.mutate({ params: { path: { tag: id } }, body }, options);
      } else {
        label.mutate({ params: { path: { label: id } }, body }, options);
      }
    },
  };
}
