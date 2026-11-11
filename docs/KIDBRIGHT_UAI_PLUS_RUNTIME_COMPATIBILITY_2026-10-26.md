# Plus runtime compatibility — 26 October 2026

The current integration targets the user's **MaixPy 4.12.5 / Runtime 1.24.0**
installation and checks the required APIs rather than rejecting newer version
numbers. The user has confirmed Connect, Run and Stop returning to the board menu
on this installation; the session evidence is recorded below. This confirmation
does not certify all future releases, functions or board images.

## Failure and correction

The user ran the previous supervisor and reported that the board displayed
`run app failed, code: -1`. Their log showed Python 3.11.6, MaixPy 4.12.5 and
Launcher PID release, followed by multimedia initialization without the expected
screen output. The previous fix only stopped the GUI executable.

Read-only inspection of the supplied image found **two distinct executables**:

- `/maixapp/apps/launcher/launcher_daemon`, started by `/etc/init.d/S06maixapp`;
- `/maixapp/apps/launcher/launcher`, the GUI child.

The daemon binary contains the exact `run app failed, code: %d` message, plus
`wait_launcher_exit` and `on_signal` symbols. This establishes the diagnostic's
origin in that image. The GUI/daemon conflict is the source-grounded explanation
for the reported behavior; direct process inspection from this environment timed
out, so an on-device causal trace was not captured. The user subsequently confirmed
that the corrected Run/Stop flow works on their board, as recorded below.
Sipeed's [run instructions](https://github.com/sipeed/MaixCDK/blob/main/docs/doc/README.md)
also require stopping `launcher_daemon` before running a program independently.

The supervisor now stops the exact daemon **before** its GUI, verifies process
identity/start time before signalling, and rescans for a respawned GUI before
importing native MaixPy. A continually respawning launcher blocks Run. It restores
the original entrypoint after the owned Python process has exited: daemon for
the supplied layout, or GUI daemon mode for the alternative known layout. It
does not stop arbitrary apps, `ws_shell` or `maixvision_server`, and does not
rewrite startup scripts or the supplied image.

## Version and API detection

1. Connect verifies TLS/WebSocket and `/tmp` through the separate JSON stat
   channel only. It never sends shell input, waits for Python metadata, imports
   native `maix`, opens the display/camera or writes files. Runtime information
   stays unavailable until Run; that does not block a successful connection.
2. MaixPy version comes from installed package metadata, with a literal AST-based
   `maix/version.py` fallback. The image's stale `version.pyi` says 4.6.0; that
   stub is deliberately not used as version evidence.
3. Runtime version comes from `/maixapp/maixcam_lib.version` (plain version or a
   JSON version field). Missing metadata remains `unknown`; it is not filled with
   the user's reported version or inferred from the image filename.
4. Run uploads the probe as a private file instead of pasting its source into a
   PTY command line. After releasing Launcher, it emits standard-library version
   metadata before native import. A single native MaixPy process then checks
   the required display/image/loop APIs, reports any missing API by name, and then
   executes `main.py`. There is no version-number allowlist or artificial upper
   version limit. New versions work through this path when these APIs and the
   board transport/layout remain compatible.
5. The UI displays detected MaixPy/Runtime versions and receives the actual API
   check result. Exit codes are accepted only after a complete newline-terminated
   marker, including when WebSocket output splits a multi-digit code.

## Supplied image evidence

[image-reference.json](research/uai-plus-runtime-2026-10-26/image-reference.json)
records the image filename/size/SHA-256 and selected file hashes, symbols and
API names. The 1,556,086,784-byte image has SHA-256
`287883ec84279574ecb70eb3856d956b23e1a9840a6ad4e946e74054deb7b422`.
Inspection used `dissect.extfs` without mounting, booting, changing the image or
executing any of its files. Certificates/private keys were not extracted.

The image's `maix/version.py` establishes 4.12.5. It does **not** contain
`/maixapp/maixcam_lib.version`; Runtime 1.24.0 is the user's report about their
installed board, not a claim inferred from the untouched image. `time.sleep_ms`
is exported dynamically by `maix/__maix_time__.py`, so checking only the time
stub would incorrectly mark it missing.

## Validation

| Local validation | Result |
|---|---|
| Frontend suite | 192 tests passed |
| Rust library suite | 230 tests passed |
| Plus Python examples/supervisor/runtime | 28 tests passed |
| TypeScript / production frontend build | Passed |
| Native Windows development build | Passed after the Connect correction; the desktop app was closed to release WebView2Loader.dll before rebuilding |
| Production UI with IPC fixtures | Passed; detected version label, Connect/Run/Stop and controls at 1280/1536 px; no page errors or external requests |
| Bundled knowledge synchronization / source provenance manifest | Verified; only the two changed knowledge documents had their normalized source hashes refreshed |

### User-confirmed device session

The user's 10:28:09–10:28:39 log and subsequent confirmation establish one
successful Connect → Run → Stop cycle on the physical mAi Plus board. The device
reported Python 3.11.6, MaixPy 4.12.5 and Runtime 1.24.0, then
`api_checked: true` with no missing APIs. The supervisor released
`launcher_daemon` (PID 296) and `launcher` (PID 297), launched `main.py`, and handled
Stop by stopping the process owned by that run. The log records multimedia cleanup,
Launcher restoration and exit code 0. The user explicitly confirmed that the run
worked and Stop returned the screen to its original board menu.

[Device session evidence](research/uai-plus-runtime-2026-10-26/device-session.json)
distinguishes the user's physical observation from the agent's local tests. The
two identical exit markers in this log belong to the same run, not two separate
passes. A second Run after Stop and newer installed runtime versions have not
been confirmed in this session.

[UI fixture evidence](research/uai-plus-runtime-2026-10-26/ui-probe.json) and
[preview](previews/uai-plus-runtime-workspace.png) use synthetic native responses,
not a board connection.

Local Python tests cover the image API declarations, baseline 4.12.5/1.24.0,
synthetic newer versions with the same API contract, missing APIs, unknown
metadata, daemon/GUI ordering, respawns, scoped Stop and cleanup. Newer-version
fixtures are explicitly synthetic, not claims that those releases exist or were
tested on hardware. Rust tests cover metadata framing, bounds, a stat-only Connect,
unknown values, complete output markers and the existing transport/review boundaries.

## Connection regression correction

The user subsequently reported repeated `Plus Python metadata probe timed out`
messages at 09:43–09:44, interspersed with USB connection timeouts. The previous
Connect path incorrectly required a Python command in the interactive PTY to
produce metadata before accepting the connection. This made optional version
information a prerequisite and was a regression from the working stat-only path.

That command and wait have been removed from Connect. The fixture now deliberately
provides no shell prompt, Python response or runtime metadata: it serves one JSON
stat and asserts that Connect sends no PTY command or upload. A successful
connection enables Run; versions appear when Run emits its metadata. Failed
baseline checks attempt a bounded WebSocket close to release the session.

The exact reason the PTY command produced no metadata on the board is not proven.
An encoded line being below the kernel's canonical input limit is not evidence
that the board's interactive shell accepts it. The current file-based Run path
does not rely on that assumption. A subsequent read-only transport check from
this environment also timed out during WebSocket handshake; it sent no Python,
file or Stop request and is not a physical connection pass.

The user's subsequent session above confirms the corrected Connect/Run/Stop path
and the visual return to the menu. For additional regression coverage, repeat Run
after Stop and test other programs or installed versions separately. An upload or
native import alone remains insufficient evidence for a program's physical output.
