# The mail server

Bugaboo sends e-mail through your organization's mail server. E-mail is **off** until an administrator has entered the mail server, tried it and switched it on. It is *Organization settings › Connections › E-mail & notifications* (the permission "Change organization settings"). The first start offers the same step, which you can skip.

## Entering the mail server

- **Mail server** and **Mail server port**: where the mail server is, as its provider tells you.
- **Protect the connection**: *Switch to encryption (usual)*, *Encrypted from the start* or *No encryption*. A user name or a password is sent without encryption only to a mail server on the same machine as Bugaboo; for any other machine it is refused (`mail.insecure`).
- **Mail server user name** and **Password**: leave the user name empty for a mail server that asks for none. The password is stored encrypted and never shown again; the page says only whether one is set. **Remove the password** takes it away.
- **Send e-mail from**: the address e-mails come from.

**Save the mail server** saves all of it. Bugaboo trusts the certificate authorities that are built into it; a mail server with a certificate of your own company cannot use encryption yet.

## Try it

**Send me a test e-mail** sends one e-mail to your own address through the mail server as saved, and tells what the mail server answered, in plain words:

- *The mail server did not accept the e-mail*: check the user name, the password and the address e-mails are sent from.
- *Bugaboo could not reach the mail server*: check the host name, the port and the connection.
- *E-mail is not set up*: enter the mail server first.
- *Sent too soon*: a person can send one test e-mail every 10 seconds.

## Switch it on

**Send e-mail** switches e-mail on. Nothing is sent before. **Short e-mails** leaves all issue text out of the e-mails.

## Signing up

Signing up without an invitation needs e-mail: Bugaboo sends the web address that lets the person choose a name and a password. While e-mail is off, signing up is closed, and the switch under *People* says so.
