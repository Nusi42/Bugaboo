import { useState } from "react";

import { api } from "../../shared/api.ts";
import { createPasskey, PasskeyCancelled } from "../../shared/passkey.ts";
import { refreshTwoStep } from "./two-step.ts";

/** What happened when the person tried to add a passkey. */
export type AddOutcome =
  | { readonly kind: "added"; readonly label: string }
  | { readonly kind: "cancelled" }
  | { readonly kind: "failed"; readonly error: unknown };

export interface AddPasskey {
  /** Makes a passkey on this device and registers it, after the password was typed again. */
  readonly add: (label: string, password: string) => Promise<void>;
  readonly busy: boolean;
  readonly outcome: AddOutcome | null;
}

/** The steps of adding a passkey: a challenge, the device, and the answer back to the server. */
export function useAddPasskey(): AddPasskey {
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<AddOutcome | null>(null);
  const start = api.query.useMutation("post", "/api/v1/auth/passkeys/options");
  const finish = api.query.useMutation("post", "/api/v1/auth/passkeys");

  async function add(label: string, password: string): Promise<void> {
    setBusy(true);
    setOutcome(null);
    try {
      const challenge = await start.mutateAsync({ body: { password } });
      const credential = await createPasskey(challenge.options);
      await finish.mutateAsync({
        body: { challenge_id: challenge.challenge_id, credential, label, password },
      });
      refreshTwoStep();
      setOutcome({ kind: "added", label });
    } catch (error) {
      setOutcome(
        error instanceof PasskeyCancelled ? { kind: "cancelled" } : { kind: "failed", error },
      );
    } finally {
      setBusy(false);
    }
  }

  return { add, busy, outcome };
}
