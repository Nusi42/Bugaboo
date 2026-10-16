# Using a key in a script

A script sends its access key in the header `Authorization`, in front of the word `Bearer`:

```text
Authorization: Bearer <your access key>
```

For example, with `curl`, to list the invitations that are open (you need the permission to invite people for this):

```text
curl --header "Authorization: Bearer <your access key>" https://your-bugaboo.example.com/api/v1/auth/invitations
```

Write the key in a file that only you can read, or in a setting of the program that runs the script. Do not write it in the script itself, and never commit it to a repository: Bugaboo's own checks look for keys in everything that is committed, and so should yours.

## What to expect

- A script needs no cookie, and no second header to protect against forged requests. The key is all it needs.
- A key can never invite people, change roles, decide who has which role or change the settings of the organization, even if you picked "Everything I can do". Asked for that, Bugaboo answers "You are not allowed to do that" (403).
- A key that may see the status of the server reads it at `/api/v1/admin/status`, for example for a monitoring script.
- Asked for something the key may not do, Bugaboo answers "You are not allowed to do that" (403). Asked with a key that does not work, it answers "Please sign in to continue" (401), the same for a key that ran out, one that was deleted and one that never existed.
- Asked for the places of your own sign-in (your password, your second step, your passkeys, the places where you are signed in, your keys), a key is turned away (403).
- A key can send 300 requests a minute. Over that, Bugaboo answers 429 and says in `Retry-After` how many seconds to wait. Ask for fewer, bigger answers with `limit` and `fields` instead of many small ones. An administrator can raise the number in the settings file (`access_key_requests_per_minute`).
- Many wrong keys in a short time from one place keep that place waiting before it may try another wrong key (429). A key that works is never held up by that, however many wrong keys others send from the same place. Still, check what your script sends: a wrong key is a failed try.

The description of all routes, with what each one needs, is on the page `/api/v1/openapi.json` of your Bugaboo.
