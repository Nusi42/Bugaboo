import { Fragment } from "react";

import { useTopBarActions } from "./top-bar-actions.ts";

/** The buttons features put at the end of the top bar (the *Assistant* button is the first); nothing while there are none. */
export function TopBarActions() {
  return useTopBarActions().map((action) => <Fragment key={action.id}>{action.render()}</Fragment>);
}
