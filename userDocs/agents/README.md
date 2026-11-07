# Agents: hand-overs and standing rules

An **agent** works on its own, outside Bugaboo, and reports to Bugaboo through an agent key. It has one person who answers for it, and it never does more than that person may. A person gives an agent work in exactly two ways: a **hand-over** of issues, or a **standing rule** over a saved search.

An agent is never *Assigned to* an issue. The issue keeps its person, and while an agent works on it the issue says *worked on by* the agent.

## Handing issues over

You need the right to change each issue (or to change the issues you reported), and *Create agents that work for you* in its project.

1. Choose one of your agents and the issues, by their IDs such as `WEB-142`.
2. Write a **note**. The brief is taken from the issue: its description, *Steps to reproduce* and *Done when*, and your note. Bugaboo says what is missing, for example "No steps to reproduce", and hands the issue over all the same.
3. Choose what the agent may do. It can always read the issue and what it points to, comment, report its work, attach results and relate issues. You can also let it **change named fields**, **change the status** or **create sub-issues**. A field that holds people (*Assigned to*, *Followers*, *Reported by*) is never given.
4. Set a **budget**: the most attempts, time and cost. The time and the cost are what the agent reports; they stay on its attempt and are never a time entry, and no total of *Time spent* counts them.
5. Choose whether the **plan needs approval** before each attempt starts, and whether a result may be **accepted by itself** when its checks pass.

A hand-over ends after 14 days unless you choose another end. An agent has one open hand-over for an issue.

## What an agent does on an issue

The **agent work** is a record of its own on the issue, apart from the comments: the brief, the attempts, the questions and the review. Its states are *Waiting for the agent*, *Agent working*, *Waiting for you*, *Ready for review*, *Agent gave up* and *Stopped*. The state of the agent work is not the status of the issue. You can find issues by `agent` and `agent state` in a search, for example `agent state is "waiting-for-you"`.

- A **question** puts the work in *Waiting for you*, and you and the person who handed the issue over are told. Your answer joins the brief.
- With **plan needs approval**, the work waits after the agent's plan for *Approve plan* or a note.
- When a result is ready, one short comment marked *Written by an agent* says so and points to the agent work.

## Reviewing

- **Accept** ends the hand-over, and the issue moves to the status you pick.
- **Send back** starts another attempt, with your note in the brief.
- **Throw away** undoes everything the attempts changed in Bugaboo, in one step. What somebody else changed since is skipped and named. A branch or a pull request outside Bugaboo is the agent's business.

A project can allow **Accept by itself when its checks pass**. It is off at the start. A result is accepted by itself only when the project allows it, your hand-over did not turn it off, the attempt has at least one check, and every check passed. Bugaboo does not run or verify the checks: the agent says whether they passed. After such an accept the issue moves to the one status of the *Done* group that the workflow allows from its status. If there is none or more than one, it keeps its status and the agent work says "Accepted; choose the next status". These results wait under *Accepted by itself* until you have looked at them.

## Standing rules

A **standing rule** lets one of your agents work on everything a saved search finds in a project, for example "every new bug in Website". It is narrower than a hand-over, because anybody who changes an issue so that it matches gives the agent that issue.

- By default the agent only **proposes**: nothing changes until you press *Apply*, as with the assistant.
- A rule may instead **change only the named fields** from *Labels, Tags, Type, Duplicate of* and *Found in release*, or **create issues** in one named project.
- A rule never changes a field that holds people and never the status.
- A rule ends after 30 days unless you choose another end, and never lasts more than 90 days. You are told three days before it ends, and *Renew* gives it a new end.
- Every issue that newly comes under the rule is written down with the moment, so you see what the agent saw.

## Stopping

Stopping an agent, or losing the right to have agents, makes all its open work *Stopped*. You are told when an agent was stopped, when a rule runs out soon, when an agent asks, when a plan waits for you, when a result is ready and when an agent gave up. You can change each of these under *Notification settings*.
