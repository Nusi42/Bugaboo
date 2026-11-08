import { addOverlayHost } from "../shell/index.ts";
import { AssistantHost } from "./AssistantHost.tsx";

let registered = false;

/**
 * Puts the assistant where the shell has room for it: its host is a layer of the shell, which adds
 * the button and the entry of quick search while AI counts at the place. Called once when the app
 * starts.
 */
export function registerAssistant(): void {
  if (registered) {
    return;
  }
  registered = true;
  addOverlayHost({ id: "assistant", render: () => <AssistantHost /> });
}
