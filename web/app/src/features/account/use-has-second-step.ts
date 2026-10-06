import { useQuery } from "@tanstack/react-query";

import { twoStepOptions } from "./two-step.ts";

/** Whether the person has a second step they can sign in with: a code app or a passkey. */
export function useHasSecondStep(): boolean {
  const status = useQuery(twoStepOptions()).data;
  return status !== undefined && (status.code_app || status.passkeys.length > 0);
}
