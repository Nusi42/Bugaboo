import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { signWithPasskey } from "../../shared/passkey.ts";
import { PasskeyButton } from "./PasskeyButton.tsx";

/** *Use a passkey* as the second step, after the password. */
export function PasskeySecondStep() {
  const start = api.query.useMutation("post", "/api/v1/auth/second-step/passkey/options");
  const finish = api.query.useMutation("post", "/api/v1/auth/second-step/passkey");
  return (
    <PasskeyButton
      label={t("second-step.passkey")}
      signIn={async () => {
        const challenge = await start.mutateAsync({});
        const credential = await signWithPasskey(challenge.options);
        return finish.mutateAsync({ body: { challenge_id: challenge.challenge_id, credential } });
      }}
    />
  );
}
