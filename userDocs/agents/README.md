# Agents: hand-overs and standing rules

How to create an agent, its keys and its limits are on [My agents](my-agents.md). What AI can and cannot do is in [the AI guide](../ai/literacy.md).

An **agent** works on its own, outside Bugaboo, and reports to Bugaboo through an agent key. It has one person who answers for it, and it never does more than that person may. A person gives an agent work in exactly two ways: a **hand-over** of issues, or a **standing rule** over a saved search.

An agent is never *Assigned to* an issue. The issue keeps its person, and while an agent works on it the issue says *worked on by* the agent.

## Handing issues over

You need the right to change each issue (or to change the issues you reported), and *Create agents that work for you* in its project.

1. Open the issue and press **Hand over to an agent** (or tick several issues in a list and press it there). A window opens: choose one of your agents. You need an agent first: see [My agents](my-agents.md).
2. Write a **note**. The brief is taken from the issue: its description, *Steps to reproduce* and *Done when*, and your note. Bugaboo says what is missing, for example "No steps to reproduce", and hands the issue over all the same.
3. Choose what the agent may do. It can always read the issue and what it points to, comment, report its work, attach results and relate issues. You can also let it **change named fields**, **change the status** or **create sub-issues**. A field that holds people (*Assigned to*, *Followers*, *Reported by*, and any own field of the kind *Person* or *People*) is never given: the hand-over is refused if it names one, or a name that is no field of an issue. Taking a relation away, changing or deleting a comment, and deleting a file need their own tick; adding a relation or a comment does not.
4. Set a **budget**: the most attempts, time and cost. The time and the cost are what the agent reports; they stay on its attempt and are never a time entry, and no total of *Time spent* counts them.
5. Choose whether the **plan needs approval** before each attempt starts, and whether a result may be **accepted by itself** when its checks pass.

A hand-over ends after 14 days unless you choose another end. An agent has one open hand-over for an issue.

## What an agent does on an issue

The **agent work** is a record of its own on the issue, apart from the comments: the brief, the attempts, the questions and the review. Its states are *Waiting for the agent*, *Agent working*, *Waiting for you*, *Ready for review*, *Agent gave up* and *Stopped*. The state of the agent work is not the status of the issue. You can find issues by `agent` and `agent state` in a search, for example the state *Waiting for you*.

- A **question** puts the work in *Waiting for you*, and you and the person who handed the issue over are told. On the issue, write your answer in the box *Your answer* and press **Answer**. Your answer joins the brief and the agent goes on.
- With **plan needs approval**, the work waits after the agent's plan for you. Press **Approve plan** to let the agent start. If you write a note first, the button becomes **Answer with the note**: the plan is not approved, and the agent makes another plan.
- An agent changes an issue handed over to it only while an attempt of it runs on that issue. Before it takes the issue, and after it has submitted its result, a change of the agent is refused, so that **Throw away** can undo what it did. The one change without an attempt is the move of the status after a result is accepted by itself: Bugaboo makes it, not the agent.
- When a result is ready, one short comment marked *Written by an agent* says so and points to the agent work.

## Reviewing

- **Accept** ends the hand-over, and the issue moves to the status you pick. The window offers the statuses the workflow allows from where the issue is, the ones of the group *Done* first. It proposes the first of those (*the workflow proposes it*): the status the workflow lists first among them. Pick another if you want one.
- **Send back** starts another attempt, with your note in the brief.
- **Throw away** undoes everything the attempts changed in Bugaboo, in one step. What somebody else changed since is skipped and named. Anything done outside Bugaboo, such as work in a code repository, stays as it is and is the agent's business.

A project can allow **Accept by itself when its checks pass**. It is off at the start. A result is accepted by itself only when the project allows it, your hand-over did not turn it off, the attempt has at least one check, and every check passed. Bugaboo does not run or verify the checks: the agent says whether they passed. After such an accept the issue moves to the status the review window would propose, but only when the workflow allows exactly one status of the group *Done* from where the issue is: nobody is asked, so Bugaboo does not choose between two. With none or with several, the issue keeps its status and the agent work says "The issue kept its status, because the workflow does not name one next status. Choose the next one yourself." These results wait under *Accepted by itself* until you have looked at them.

## Standing rules

A **standing rule** lets one of your agents work on everything a saved search finds in a project, for example "every new bug in Website". It is narrower than a hand-over, because anybody who changes an issue so that it matches gives the agent that issue.

- By default the agent only **proposes**. A proposal shows on the issue as a *Proposed change* card with the name of the agent, the value before and after, and *Apply* and *Discard*. Nothing changes until you press *Apply*, as with the assistant. On an issue that only the rule found, the agent has no attempt: it reads the issue and proposes, and changes nothing there.
- In the window, a rule may instead be set to **change only these fields** (*Labels, Tags, Type, Duplicate of* and *Found in release*), or to **create issues in one project**.
- A rule never changes a field that holds people and never the status.
- A rule ends after 30 days unless you choose another end, and never lasts more than 90 days. You are told three days before it ends, and *Renew* gives it a new end.

To create one, open *My agents*, press **Create a standing rule**, and in the window *New standing rule* choose a saved search, the agent, the project it works in and what it may do (*Propose only*, *Change only these fields* or *Create issues in one project*), then press **Create the rule**. The numbers are on [the page of numbers](../ai/numbers.md).

- Every issue that newly comes under the rule is written down with the moment, so you see what the agent saw.

## Stopping

Stopping an agent, or losing the right to have agents, makes all its open work *Stopped*. You are told when an agent was stopped, when a rule runs out soon, when an agent asks, when a plan waits for you, when a result is ready and when an agent gave up. You can change each of these under *Notification settings*.
