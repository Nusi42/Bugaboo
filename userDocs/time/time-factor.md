# The time factor

The time factor is your own estimate for one issue: how much longer or shorter than *Estimated time* it will take you. *×1.25* means a quarter longer than the estimate; *×1* means as estimated. The timeline plans the issue with it when it plans by effort.

The time factor belongs to the person on the bar, the first person in *Assigned to*. It goes back to ×1 by itself when another person becomes the first assigned person, and the history says so.

## Where you see it

On the issue page the row **Time factor** stands under *Estimated time*. It says whose estimate it is:

- "×1.25 · Lea's own estimate" when the person on the bar set it;
- "×1.5 · set by Ada Brandt" when somebody else set it;
- "×1 (as estimated)" when nobody set one.

If you are the person on the bar, you always see the row, also where your role does not see the time factor of other people's issues. Everybody else sees it as the field permissions of *Time factor* allow (at the start Administrators, Project leads and Team members may change it, Contributors and Viewers see it, and Guests do not). Where your role is shown somebody only as their role, the row says *set by Someone*.

## Changing it

Press **Change** in the row. The window asks **How long will it take you?** when you are the person on the bar, and **How long will it take Lea?** when you change it for somebody else.

- **Takes me …**: type how long the issue takes you, like `10h`. Bugaboo works out the time factor against *Estimated time*. This choice is there only when you can read the estimate.
- **A factor**: type the time factor itself, like `1.25`; `1` is as estimated.

Under the box the window shows what you typed beside the estimate, for example "Estimate 8h · ×1.25 = 10h". **Save the time factor** saves it; the history of the issue shows the change.

The person on the bar may always change their own time factor. Anybody else needs to be allowed to change the issue and the field *Time factor*. When somebody else changes it, the person on the bar is told (*Your time factor was changed*, see [Notifications](../notifications/notifications.md)), and the window says so before you save.

## What it is never used for

The time factor is an estimate for one issue, not a judgement of a person. Bugaboo keeps none per person or per kind of work, and nothing adds them up. No search filters, sorts or groups by it, no list offers it as a column, no total or report uses it, and AI never sets or proposes one.
