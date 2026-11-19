# My agents

**My agents** is your page for the agents that work for you: *My agents* in the menu (on a phone, under *More*). You see it when you have an agent, or when you may create agents. [Agents: hand-overs and standing rules](README.md) says how you give an agent work. [The AI guide](../ai/literacy.md) says what an agent can and cannot do.

## What the page shows

1. **Waiting for you**, first. A question to answer, a plan to approve, a result to review, a standing rule that runs out soon. Each line has a button that opens the issue. *Nothing waits for you.* is what you read when it is empty.
2. **Accepted by itself.** Results that were accepted without you because the project allows it and every check passed, also those of hand-overs that a standing rule gave to the agent. Look at them and press **I looked at it**.
3. **Each agent** with what it works on, and the buttons **Pause**, **Resume** and **Stop**.
4. **Standing rules**, with **Renew** (a rule that never runs out has none) and a button to create one. A rule says whether it proposes changes or hands over what it finds. See [Agents](README.md).

**Stop all my agents** stops every agent of yours at once, takes their keys back and ends the work they were given. A stopped agent never works again.

**Pause** keeps the agent and its work; **Resume** lets it go on. Bugaboo also pauses an agent that reached one of its limits, and says which limit it was. You can raise that limit up to what your organization allows, and press **Resume**.

## Creating an agent

Press **Create an agent**, give it a name and press **Create the agent**. You need the right *Create agents that work for you* in a project. The page of the new agent opens. The most agents a person can have is on [the page of numbers](../ai/numbers.md).

An agent is never *Assigned to* an issue: the issue keeps its person and says *worked on by* the agent.

**One agent for each role, each with its own key.** If several programs work for you, for example one that fixes bugs and one that writes tests, create an agent for each and give each its own agent key. Everything an agent does is shown under its name, its limits count for it alone, and you can pause, stop or hand over one without the others. Programs that share one key all appear as the same agent, and only their own words could tell them apart.

## Creating several agents at once

A team of agents works best with one agent for each part of the work, for example one that plans, one that builds and one that checks: each has its own key, its own history, its own limits and can be paused on its own. To create them in one step, press **Create several agents** under the list of your agents. Write one name a line, choose when their keys run out, enter your password **once** and press **Create … agents**.

- Every agent gets one key, called like the agent. You see all the keys **once** at the end, each with its own button to copy it and **Copy all keys** for the whole list (one line each: the name of the agent, then its key). Copy them then: Bugaboo keeps only a fingerprint of each.
- All or nothing: if one agent cannot be created, for example because you have an agent of that name already or a name is in the list twice, none is created and Bugaboo says which line to fix.
- The agents start with the same limits as an agent you create alone. The most agents of one list is on [the page of numbers](../ai/numbers.md).

## Agents asked for by a program

An AI app you allowed, or a script with one of your access keys that may change things, can ask you for the agents it needs, for example one that plans and one that builds. It asks with a reason, and nothing is created until you say yes.

- You are told by a notification, and the request shows at the top of *My agents* under **Asked for by a program**: who asked, why, and each agent with what it is for. A request waits a day for your answer.
- Tick the requests you want and press **Approve**, with your password once. Every agent of them is created in one step; if one cannot be created, none is.
- **Decline** ends a request. Nothing is created.
- After you approved, the program that asked collects the keys of the new agents **once**, within a day. You do not see these keys, and nobody else can collect them: not another key of yours, not another app. If the program does not collect them in time, the agents stay without keys, and you can create keys for them on their pages.
- A program can never create a key by itself: every key still needs your password. An agent cannot ask for agents, and a key that can only look cannot ask. A key or an app has at most three requests waiting at once. Requests are deleted a week after they were asked.

## The page of an agent

- **Agent keys.** An agent signs in with a key. Press **Create an agent key**, give it a name that says what it is for (for example "Fixer on the build server") and enter your password. You see the key **once**: copy it then. Every key runs out. **Renew the key** gives a new one and takes the old one back; **Take back** stops a key at once, and an agent that uses it can no longer sign in.
- **Limits.** *Changes a minute* and *Issues at once*, and how many times it may try one issue. A limit is never more than your organization allows. When an agent reaches a limit it is paused with a sentence that says which.
- Its **work** and its **standing rules**.
- **Hand to another person.** The other person must accept the agent. Its keys are taken back and they create their own.

## For administrators

*Organization settings › Connections › Agents* lists every agent of the organization (working first, then paused, then stopped) and sets the most an agent may do. You can **Stop** any agent. A project can switch agents off with the switch **Agents in this project** in its settings, and you can switch agents off for the whole organization. See [the numbers](../ai/numbers.md).
