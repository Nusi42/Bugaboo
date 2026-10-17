import { api } from "../../../shared/api.ts";
import { refreshMailServer } from "../../../shared/query-keys.ts";

/** What the mail server has that is no setting: whether a password is stored, and if it can send. */
export function useMailServerStatus() {
  return api.query.useQuery("get", "/api/v1/mail");
}

/** Enters, changes or removes the password of the mail server. */
export function useSetMailPassword() {
  return api.query.useMutation("put", "/api/v1/mail/password", {
    onSuccess: refreshMailServer,
  });
}

/** *Send me a test e-mail*. */
export function useSendTestEmail() {
  return api.query.useMutation("post", "/api/v1/mail/test");
}
