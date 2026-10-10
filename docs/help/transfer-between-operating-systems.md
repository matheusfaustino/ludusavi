# Transfer between operating systems
Although Ludusavi itself runs on Windows, Linux, and Mac,
it does not automatically support backing up on one OS and restoring on another.

This is a complex problem to solve because
games do not necessarily store data in the same way on each OS.
Ludusavi only knows where each game stores its data on a given OS,
but does not know which save locations correspond to each other,
or even if any of them do correspond.
Some games store data in completely different and incompatible ways on different OSes.

In simple cases, you may be able to configure [redirects](/docs/help/redirects.md)
to translate between specific Windows and Linux paths,
but this would generally require multiple redirects tailored to each game.
In more complex cases, this is not practical or feasible.

Ludusavi has experimental support for Windows and Wine prefixes, described below.
You can follow this ticket for any future updates:
https://github.com/mtkennerly/ludusavi/issues/194

## Windows and Wine
Ludusavi supports an experimental feature to translate between Windows paths and Wine prefixes.
This currently only supports files, not registry values.

As this is an experimental feature, you must enable it via the [config file](/docs/help/configuration-file.md),
by setting `scan.redirectWine: true`.

To restore into a Wine prefix, a game must have a "preferred" Wine prefix.
Ludusavi uses the first of these that exists:

* The first Wine prefix in the game's custom entry.
* A prefix that Ludusavi detects from a launcher, such as Heroic or Lutris.
  For Proton prefixes, Ludusavi prefers the `pfx` subfolder.
* The Steam Proton prefix (`steamapps/compatdata/<id>/pfx`) for the game's Steam ID or non-Steam shortcut.
  This lets you restore a non-Steam shortcut's saves on a computer where the shortcut has a different ID.

If the original Wine prefix still exists, Ludusavi restores files to their original paths.

If several files would be restored to the same path,
Ludusavi restores only one of them and skips the others.
It prefers files that weren't redirected, and then files from a Proton prefix.

There are some exclusions where the automatic Wine redirects will not apply:

* No preferred Wine prefix
* Wine prefix does not already exist
* Wine prefix has multiple potential user folders
* Windows path is on a complex UNC drive instead of a simple drive letter

### Back up on both systems
Don't use the same backup folder on Windows and Linux.
Each backup only includes the files that the current system can see,
so a backup on one system drops the other system's files from the latest backup.

Instead, give each system its own backup folder.
To restore saves from the other system, run `ludusavi restore --path <other backup folder>`.
