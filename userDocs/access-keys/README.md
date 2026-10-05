# Access keys

An **access key** is a secret for your scripts and apps. With it a program can sign in as you, but it can do at most what you can do, and it always runs out.

You make, look at and delete your keys in *My settings*, under "Access keys".

## Making a key

1. Open *My settings* and then "Access keys", and press "Make an access key".
2. Give the key a name that tells you what it is for, like "Nightly report".
3. Choose **What it may do**:
   - **See only**: the key can look at what you can look at and change nothing.
   - **See and change**: the key can also change what you may change.
   - **Everything I can do**: the key can do all that you can do.
   - Under "More options" you can pick single permissions instead.
4. If you want, choose the **projects** the key is for. The key then works in those projects and in the projects inside them, and nowhere else. A key for projects gives nothing for the whole organization.
5. Choose when the key **runs out**. If you do not choose, it runs out after 90 days. It never lasts longer than the longest time your administrators set (one year at most).
6. Type your password again and press "Make the key".

Bugaboo shows the key **once**. Copy it now and keep it somewhere safe, like a password manager. Bugaboo stores only what it needs to recognize the key, so nobody, you and the administrators included, can read it again. If you lose it, delete the key and make a new one.

## What a key can and cannot do

- A key follows you. When an administrator takes a role away from you, the keys you made lose it as well, at once. A key never has more than you have.
- A key waits for your second step. When your organization starts to ask for two-step sign-in and you have not set it up yet, your keys do not work until you have; then they work again, nothing is lost.
- A key cannot change how you sign in: not your password, not your second step, not your passkeys, not the places where you are signed in, and not your keys. For that you sign in with your password.
- A key that has run out, or that you deleted, does not work, and Bugaboo does not say which of the two it is.
- Every use of a key is counted. A key that sends too many requests in a minute has to wait a little. Other keys, and you, are not held up by it.

## Looking at and deleting your keys

The list shows each key by its name and the first letters after `bb_`, what it may do, which projects it is for, when it runs out and when it was last used (to the minute). To delete a key, press "Delete" next to it. Bugaboo asks you to confirm, because a deleted key cannot come back. The program that uses the key stops working at once.

Making and deleting a key is written down in the security log, and so is every try with a key that does not work.

## Using a key in a script

See [Using a key in a script](use-a-key.md).
