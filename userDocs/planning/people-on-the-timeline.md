# People on the timeline

The timeline shows who works on each issue and on which days they cannot. Bugaboo checks the plan against those days and warns you when it does not fit, with ways out you choose yourself. On a phone the timeline is a list with a short bar for each issue. For the chart itself, see [The timeline](timeline.md).

## The person on a bar

The first person in *Assigned to* is the person on the bar. Their name stands after the end of the bar, next to its Issue ID; when more people are assigned, a number follows, for example "Lea Lang +2". The card of the bar lists everybody.

You see people as *Who sees people* lets you see them:

- with **Names**, by their name;
- with **Role only**, as "Someone (Team member)". You cannot group the timeline by person then, because nobody could be told apart;
- with **Hidden**, nobody is named on the bar.

## Group by person

Set **Group by** to **Assigned to** to get one row for each person, so you see who has overlapping work. An issue with several people is in each of their rows. In the row of the first person it is drawn as usual; in the rows of the others it is an outline. Its card says "Planned with Lea's work times", and so does the name of the bar that is read out: the bar takes the days of the first person. The row of each person also shows their away days and public holidays across the chart, also where they have no bar, as far as you may see their work times.

## Days without work

The days on which the person on a bar does not work are marked inside the bar:

- **Away** days are hatched. Bugaboo never knows or shows why somebody is away: everybody else sees only *Away*.
- **Public holidays** are hatched the other way, with their name, for example "Holiday: Christmas Day".
- Days that are not the person's usual work days are shaded like a weekend.

The Legend under the tools explains the marks. Whether you see these days at all follows *Who sees people* for *Work times*. When the person on the bar is not named for you, the days stay hatched without a word. When you may not see who an issue is assigned to at all, its bar has no hatched days and the plan warnings about its person are not listed for you.

## Plan by effort

With **Plan by effort** on, an issue with an estimate and only one date gets as many of the person's work days as its estimate, times its time factor, needs. Each day counts with the person's work hours and their share in the project, and away days, days off and public holidays are skipped, so the bar grows around them. When a person has half of their hours in the project, the same issue takes twice the days. The days that are worked out are shown, never saved, and a due date somebody set never moves by itself.

## The card of a bar

Select a bar to see its card at the side. Besides the dates, it shows:

- everybody the issue is assigned to;
- the **effort**, for example "Estimate 16h · ×1.25 = 3 work days · Lea's own estimate" (the time factor is always named, ×1 too), with **Change** to give the time factor or how long it takes (the same window as on the issue page). The work days are counted when *Plan by effort* is on;
- the **person's time**, for example "Mon–Fri, 8h a day · 100 % in Website", when you may see their work times;
- that *Plan by effort* worked out a date, when it did.

## Plan warnings

**Warnings** in the tools shows how many warnings the plan has and opens their list at the side (on a phone, from the bottom). A warning sign also stands at the end of the bar it is about. Point at a warning to mark its bar; press its sentence to select the bar.

There are four kinds:

- **Away**: "Lea is away 26 Oct 2026 to 30 Oct 2026, while WEB-280 is planned with their work times."
- **Not enough days**: "WEB-280 needs 6 work days; there are 3 work days before its due date."
- **Delay**: "WEB-280 now ends 2 days after WEB-284 starts, and WEB-284 waits for it." Nothing moves by itself.
- **Too much at once**: "Lea has 9h planned on 27 Oct 2026 in Mobile app; the share there is 3h 36m."

Each warning offers the ways out that fit it:

- **Move it after** shows every issue that would move, with its days before and after the move, before anything is saved. **Move it** (or **Move all 3**) saves them in one step, with **Undo**.
- **Make it longer** shows the due date the issue needs; it changes only when you confirm it, with **Undo**.
- **Give it to someone else…** opens the usual person picker. Bugaboo never suggests whom: the people of the project are listed by name, and **Give it** next to a name makes that person the person on the bar. The others assigned after the first stay.
- **Keep and accept** keeps the plan as it is. The warning stays at the end of the list, faded, with who accepted it, until the dates of its issue change.

The list ends with "Away times show only as Away. Bugaboo never suggests whom to give an issue."

## On a phone

On a phone the timeline is a list grouped like the rows of the chart, and folded the same way. Each issue shows its status icon, Issue ID and title, its dates, its person, and a short bar over seven weeks with today marked, in the same colors as on a computer. **Not scheduled yet** is the last group.

Nothing is dragged on a phone. Tap an issue to open its card from the bottom, with **Set dates…**, **Add dependency…** and **Open**. **Warnings** opens the list of plan warnings the same way; pressing a warning closes the list and opens the card of its issue. A bar that goes on before or after the seven weeks says **Earlier** or **Later** at that end.
