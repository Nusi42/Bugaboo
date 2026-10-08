import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button } from "@bugaboo/ui";
import { useState } from "react";

import { type SignedIn, useAfterSignIn } from "../../shared/after-sign-in.ts";
import { messageOf } from "../../shared/errors.ts";
import { PasskeyCancelled } from "../../shared/passkey.ts";

/** What went wrong with the passkey: the person did not use it, or the server refused it. */
type Trouble =
  | { readonly cancelled: true }
  | { readonly cancelled: false; readonly error: unknown };

export interface PasskeyButtonProps {
  readonly label: string;
  /** Asks the server for a challenge, lets the device answer it, and hands the answer back. */
  readonly signIn: () => Promise<SignedIn>;
}

/** A button that signs in with a passkey: the device asks for a fingerprint, a face or a screen lock. */
export function PasskeyButton({ label, signIn }: PasskeyButtonProps) {
  const afterSignIn = useAfterSignIn();
  const [busy, setBusy] = useState(false);
  const [trouble, setTrouble] = useState<Trouble | null>(null);

  async function press(): Promise<void> {
    setTrouble(null);
    setBusy(true);
    try {
      afterSignIn(await signIn());
    } catch (error) {
      setTrouble(
        error instanceof PasskeyCancelled ? { cancelled: true } : { cancelled: false, error },
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Button
        wide
        isPending={busy}
        onPress={() => {
          void press();
        }}
      >
        {label}
      </Button>
      {trouble?.cancelled === true ? <Alert>{t("passkeys.cancelled")}</Alert> : null}
      {trouble?.cancelled === false ? (
        <Alert tone={alertTone.danger}>{messageOf(trouble.error)}</Alert>
      ) : null}
    </>
  );
}
