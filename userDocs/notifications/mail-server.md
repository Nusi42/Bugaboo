# The mail server

Bugaboo sends e-mail through your organization's mail server. E-mail is **off** until an administrator has entered the mail server, tried it and switched it on. It is *Organization settings › Connections › E-mail & notifications* (the permission "Change organization settings"). The first start offers the same step, which you can skip.

## Entering the mail server

- **Mail server** and **Mail server port**: where the mail server is, as its provider tells you.
- **Protect the connection**: *Switch to encryption (usual)*, *Encrypted from the start* or *No encryption*. A user name or a password is sent without encryption only to a mail server on the same computer as Bugaboo; for any other computer it is refused: *The mail server is set to send its user name or password without encryption to another computer…*.
- **Mail server user name** and **New password**: leave the user name empty for a mail server that asks for none. The password is stored encrypted and never shown again; the page says only whether one is set. **Remove the password** takes it away. If you change the mail server, its port or the user name, the password is removed too: a new mail server needs its password again.
- **Send e-mail from**: the address e-mails come from.

**Save the mail server** saves all of it. Encryption works with mail servers whose certificate comes from a well-known provider. A certificate your company created itself does not work yet.

## Test e-mail

**Send me a test e-mail** sends one e-mail to your own address through the mail server as saved, and tells what the mail server answered, in plain words:

- *The mail server did not accept the e-mail*: check the user name, the password and the address e-mails are sent from.
- *Bugaboo could not reach the mail server*: check the mail server, its port and the connection.
- *E-mail is not set up*: enter the mail server first.
- *You can send another test e-mail in … seconds*: how long a person waits is on [Times and counts](numbers.md).

## Switch it on

**Send e-mail** switches e-mail on. Nothing is sent before. **Short e-mails** leaves all issue text and the names of projects out of the e-mails.

## Signing up

Signing up without an invitation needs e-mail: Bugaboo sends the link that lets the person choose a name and a password. While e-mail is off, signing up is closed, and the switch under *People* says so.
