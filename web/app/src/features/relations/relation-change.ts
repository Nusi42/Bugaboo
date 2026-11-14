import { useIsMutating, useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { queryKeys } from "../../shared/query-keys.ts";

/**
 * A change of the relations or the parts of an issue. Its sections hold what they show while it is
 * on its way (`useHeldWhileChanging`), so what it made shows in one frame with the sentence that
 * says it was made (UX §6.4).
 */
export function useRelationChange<Variables, Answer>(
  mutationFn: (variables: Variables) => Promise<Answer>,
  onSuccess?: (answer: Answer) => void,
) {
  return useMutation({
    mutationKey: queryKeys.issueRelations.changing,
    mutationFn,
    ...(onSuccess === undefined ? {} : { onSuccess }),
  });
}

/**
 * What a section of the relations shows: the newest answer, or, while a change of relations is on
 * its way, the answer it showed before. The change waits for the new answers itself, so the
 * section changes once, when it is done.
 */
export function useHeldWhileChanging<Value>(value: Value): Value {
  const changing = useIsMutating({ mutationKey: queryKeys.issueRelations.changing }) > 0;
  const [held, setHeld] = useState(value);
  if (!changing && held !== value) {
    setHeld(value);
  }
  return changing ? held : value;
}
