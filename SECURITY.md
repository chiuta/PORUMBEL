# Security

PORUMBEL listens on your local network and writes files to your disk, so it is built defensively:

* every request needs the access token (compared in constant time); an empty token locks everything;
* only image files whose first bytes match their extension (JPEG, PNG, TIFF, BMP) are stored, under a plain
  file name (no paths, no reserved Windows names, no control or direction-override characters), size-limited,
  written to a temporary file and moved in only when complete; concurrent uploads cannot overwrite each other;
* the HTTP side bounds headers, bodies and connection time, drops half-open connections after 10 s and lets a new
  connection displace the oldest idle one when the table is full;
* the control panel sends a strict Content-Security-Policy, never renders file names as HTML, and changing actions
  accept only the token in the query or a header, not browser-cached Basic credentials;
* ADB mode accepts only direct children of the camera folder, kills a stuck `adb` after `adb_timeout` seconds and
  deletes from the phone (if you enabled it) only after the copy has the same size;
* synced folders: symbolic links are not followed.

**Known limits.** The traffic is plain HTTP: the token and the photos can be read by anyone who can see your
Wi-Fi traffic. Use a network you trust, ADB mode, or `bind = 127.0.0.1`. Do not expose the port to the internet.

Report problems privately to alexio@trom.tf (or through GitHub's *Report a vulnerability*).
