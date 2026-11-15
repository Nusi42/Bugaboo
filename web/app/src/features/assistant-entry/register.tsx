import { addOverlayHost, addTopBarAction } from "../shell/index.ts";
import { AssistantTopBarButton } from "./AssistantButton.tsx";
import { AssistantHost } from "./AssistantHost.tsx";

let registered = false;

/**
 * Puts the assistant where the shell has room for it: its host is a layer of the shell, which adds the
 * button and the entry of quick search while AI counts at the place, and loads the panel when it is
 * first opened. Called once when the app starts.
 */
export function registerAssistant(): void {
  if (registered) {
    return;
  }
  registered = true;
  addOverlayHost({ id: "assistant", render: () => <AssistantHost /> });
  // The place of the button is there from the start: the top bar waits for the answer whether AI counts here.
  addTopBarAction({ id: "assistant", render: () => <AssistantTopBarButton /> });
}
