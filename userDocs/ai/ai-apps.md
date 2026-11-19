# AI apps: connecting Claude Code

An **AI app** is a tool outside Bugaboo, such as Claude Code or Claude Desktop, that works with your account through **MCP**, the open way in which AI tools talk to a program such as Bugaboo. It always works as you and never does more than you can. [The AI guide](literacy.md) says what that means; [Connected apps](../access-keys/connected-apps.md) says how you allow an app and how you take it back.

AI apps are off at the start (an upgrade from an older version keeps them as they were). An administrator switches them on or off for the whole organization (*Organization settings › Connections › Set up the assistant*, the card *AI apps*) or for one project (the switch *AI apps in this project* in its settings). The assistant and agents have switches of their own: switching the assistant off does not switch AI apps off.

## Connecting Claude Code

1. Find the address of Bugaboo: the one you open it at, followed by `/mcp`.
2. In a terminal, add it as an MCP server of Claude Code:

   ```text
   claude mcp add --transport http bugaboo https://your-bugaboo.example/mcp
   ```

3. Start Claude Code and type `/mcp`. Bugaboo asks you to sign in: your browser opens the page "Claude Code wants to work with your account". Sign in if you are not, look at what it asks, and press **Allow**. Press **Cancel** and nothing is allowed.
4. Back in Claude Code, ask something about your issues, for example "What is on my list in the project WEB?".

Claude Code runs on your own computer, so nobody has to set it up for it: it tells Bugaboo who it is and the page sends the answer back to your computer. An app that runs on the web signs in only from a web address an administrator allowed.

## What happens next

- Each session of the app is one **run**. You can see its changes and undo them in one step with **Undo the run**.
- What the app writes is marked **Written by an AI app**, with its name.
- It can look at what you may look at. If you allowed *See and change*, it can also change what you may change, with the limits in [the AI guide](literacy.md).
- Every tool that changes something takes an extra argument `preview`. With `preview` set to true the app only asks what the call would change: Bugaboo answers with what the issue is now and what it would be, saves nothing and writes no entry in the history or in the run. The same call without `preview` changes. A preview is checked against your rights like the change itself, so a *See only* app is refused, and it counts toward the same limit of calls and is not counted as a change.
- You see the apps you allowed in *My settings*, under *Connected apps*, and press **Take back** to end one at once.
- Your data goes where the app sends it. Bugaboo's agreement with the provider of the assistant does not cover it: an app such as Claude Code sends to its own vendor, under your organization's own agreement with that vendor.

## Limits for programs that call `/mcp` themselves

Claude Code and the other MCP apps wait by themselves. A program that sends its own HTTP requests to `/mcp` needs to know how Bugaboo says "wait":

- One app or one key may call only so many times a minute, and one person only so many times over all their apps and keys. Every message to `/mcp` counts.
- A **tool call** over the limit is answered with HTTP `200` and a result with `isError: true`, whose text says how many seconds to wait, and with the header `Retry-After`. Nothing of the call is done. The answer is a result so that the AI reads it and waits; a program reads `Retry-After` or `isError`, not only the status.
- **Every other message** over the limit (`initialize`, `tools/list`, a read of a resource) is answered with HTTP `429` and `Retry-After`.
- An **agent** is also held to a number of changes a minute, which its person may set. An agent that goes over it is **paused**, not told to wait: its person sees which limit it reached and presses *Resume*.
- A message has a largest size; a bigger one is answered `413` before it is read. A message is sent as `application/json`; another type is answered `415`.

How many calls and changes a minute, and how big a message may be, is on [the page of numbers](numbers.md).

## Agents are different

An agent is not an app you sign in with. It has its own key and works on issues you hand over to it. See [Agents](../agents/README.md).
