# AI apps: connecting Claude Code

An **AI app** is a tool outside Bugaboo, such as Claude Code or Claude Desktop, that works with your account through MCP. It always works as you and never does more than you can. [The AI guide](literacy.md) says what that means; [Connected apps](../access-keys/connected-apps.md) says how you allow an app and how you take it back.

AI apps work only while an administrator has switched AI on for the organization. They are switched off with it.

## Connecting Claude Code

1. Find the address of Bugaboo: the one you open it at, followed by `/mcp`.
2. In a terminal, add it as an MCP server of Claude Code:

   ```text
   claude mcp add --transport http bugaboo https://your-bugaboo.example/mcp
   ```

3. Start Claude Code and open its list of servers. Bugaboo asks you to sign in: your browser opens the page "Claude Code wants to work with your account". Sign in if you are not, look at what it asks, and press **Allow**. Press **Cancel** and nothing is allowed.
4. Back in Claude Code, ask something about your issues, for example "What is on my list in the project WEB?".

Claude Code registers itself and comes back to your own computer. Another app that runs on the web signs in only from a web address an administrator allowed.

## What happens next

- Each session of the app is one **run**. You can see its changes and undo them in one step with **Undo the run**.
- What the app writes is marked **Written by an AI app**, with its name.
- It can look at what you may look at. If you allowed *See and change*, it can also change what you may change, with the limits in [the AI guide](literacy.md).
- It is slowed down if it makes too many calls in a minute.
- You see the apps you allowed in *My settings*, under *Connected apps*, and press **Take back** to end one at once.

## Agents are different

An agent is not an app you sign in with. It has its own key and works on issues you hand over to it. See [Agents](../agents/README.md).
