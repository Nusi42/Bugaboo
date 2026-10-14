import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

import type { FormView } from "./layouts.ts";

/** The sentence that says a form shows only the basics, because its layout could not be read. */
export function FormNotice({ form }: { readonly form: FormView }) {
  return form.failed ? <Alert tone={alertTone.warning}>{t("layouts.unreadable")}</Alert> : null;
}
