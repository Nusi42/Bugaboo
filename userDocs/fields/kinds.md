# Kinds of information

When you create a field you choose what kind of information it holds. The kind cannot be changed afterwards.

| Kind | What it holds | Example |
|------|---------------|---------|
| Text | A short text | Customer name |
| Long text | A text with Markdown, edited like a description, with a lock while somebody edits | Notes |
| Whole number | A number without a decimal part | Affected users |
| Decimal number | A number with up to six digits after the point | Budget |
| Yes / No | One of two answers | Needs a refund |
| Web address | An address that starts with http or https | Customer page |
| Date | A day | Promised by |
| Date and time | A day and a time | Seen at |
| Duration | A length of time, typed like `1h 30m` | Time to fix |
| Pick one | One choice of a list | Severity |
| Pick several | Several choices of a list | Platforms |
| Person | One person | Tester |
| People | Several people | Reviewers |
| Group | One group | Responsible team |
| Issue | One issue | Duplicate of |

A person in a field is shown the way the information *Assigned to* is shown to the reader: by name, by role only or as "Someone". A group is shown by name to people who see names when somebody is mentioned, and an issue by its ID to people who see that issue.

In this version the screens show people, groups and issues in these fields and can take the value away; choosing them comes with the pickers of the issue page. The programming interface accepts them already.
