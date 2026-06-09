# Transfer between operating systems
Although Ludusavi itself runs on Windows, Linux, and Mac,
it does not automatically support backing up on one OS and restoring on another
for native platform paths.

This is a complex problem to solve because
games do not necessarily store data in the same way on each OS.
Ludusavi only knows where each game stores its data on a given OS,
but does not know which save locations correspond to each other,
or even if any of them do correspond.
Some games store data in completely different and incompatible ways on different OSes.

## Native cross-OS (limited)

For native Windows, Linux, and macOS paths that are not through Wine,
Ludusavi cannot automatically determine if saves are equivalent.
In simple cases, you may be able to configure [redirects](/docs/help/redirects.md)
to translate between specific Windows and Linux paths,
but this would generally require multiple redirects tailored to each game.

You can follow this ticket for future updates on native cross-OS support:
https://github.com/mtkennerly/ludusavi/issues/194
