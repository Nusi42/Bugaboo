import { t } from "@bugaboo/messages";
import { OrDivider } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { signWithPasskey } from "../../shared/passkey.ts";
import { statusOptions } from "../../shared/session.ts";
import { PasskeyButton } from "./PasskeyButton.tsx";

/** Sign in with a passkey, without a password, when the server can do passkeys at all. */
export function PasskeySignIn() {
  const start = api.query.useMutation("post", "/api/v1/auth/passkey/sign-in/options");
  const finish = api.query.useMutation("post", "/api/v1/auth/passkey/sign-in");
  const available = useQuery(statusOptions()).data?.passkeys_available === true;
  if (!available) {
    return null;
  }
  return (
    <>
      <PasskeyButton
        label={t("sign-in.passkey")}
        signIn={async () => {
          const challenge = await start.mutateAsync({});
          const credential = await signWithPasskey(challenge.options);
          return finish.mutateAsync({ body: { challenge_id: challenge.challenge_id, credential } });
        }}
      />
      <OrDivider label={t("sign-in.or")} />
    </>
  );
}
