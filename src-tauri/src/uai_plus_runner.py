"""Temporary Plus run supervisor; standard library only, never installed at boot.

The stock Launcher owns the display. Release that exact executable before
starting MaixPy, then restore it after the owned program has exited. Upstream
reference: KidBrightAI/kidbright_MAI_ide 7b549397, boards/kidbright-mai-plus/main.py.
This implementation deliberately does not stop every process under /maixapp.
"""
import fcntl
import importlib.metadata
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback

LAUNCHER = "/maixapp/apps/launcher/launcher"
LAUNCHER_DAEMON = "/maixapp/apps/launcher/launcher_daemon"
LAUNCHER_EXECUTABLES = (LAUNCHER_DAEMON, LAUNCHER)
LOCK = "/tmp/.vibekidbright-plus.lock"
STOP_FILE = ".vibekidbright-stop"
POLL_SECONDS = 0.1
GRACE_SECONDS = 3.0


def log(message):
    try:
        print("[Plus Run] " + message, flush=True)
    except OSError:
        pass  # Cleanup must still run if the USB PTY has already closed.


def process_identity(pid, proc=Path("/proc")):
    try:
        root = proc / str(pid)
        # comm can contain spaces/parentheses; fields after its final ')' start
        # with state (field 3). starttime is field 22.
        stat = (root / "stat").read_text().rsplit(")", 1)[1].split()
        if stat[0] == "Z":
            return None
        return ((root / "exe").resolve(strict=True).as_posix(), stat[19])
    except (OSError, ValueError, IndexError):
        return None


def display_processes(proc=Path("/proc")):
    launchers, conflicts = [], []
    for item in proc.iterdir():
        if not item.name.isdigit() or int(item.name) == os.getpid():
            continue
        pid = int(item.name)
        identity = process_identity(pid, proc)
        if identity is None:
            continue
        exe, _ = identity
        try:
            argv = (item / "cmdline").read_bytes().split(b"\0")
            argv = [arg.decode(errors="replace") for arg in argv if arg]
            cwd = (item / "cwd").resolve(strict=True).as_posix()
        except OSError:
            continue
        if exe in LAUNCHER_EXECUTABLES:
            launchers.append((pid, identity))
        elif exe.startswith("/maixapp/apps/"):
            conflicts.append((pid, exe))
        elif Path(exe).name.startswith("python"):
            app = next((arg for arg in argv[1:] if arg.startswith("/maixapp/apps/")
                        or arg in ("/root/main.py", "/root/run.py")), None)
            old_run = Path(cwd).name[len("vibekidbright-"):]
            if app or cwd.startswith("/maixapp/apps/") or (cwd.startswith("/tmp/vibekidbright-") and old_run.isdigit()):
                conflicts.append((pid, app or cwd))
    # Runtime 1.24.0's daemon waits for the GUI and can draw an error screen
    # when it dies. Release the daemon first, before touching its GUI child.
    launchers.sort(key=lambda item: LAUNCHER_EXECUTABLES.index(item[1][0]))
    return launchers, conflicts


def stop_launcher(pid, identity):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        if process_identity(pid) != identity:
            return
        try:
            os.kill(pid, sig)
        except ProcessLookupError:
            return
        deadline = time.monotonic() + GRACE_SECONDS
        while time.monotonic() < deadline:
            if process_identity(pid) != identity:
                return
            time.sleep(POLL_SECONDS)
    raise RuntimeError("Launcher did not release the display; Python was not started.")


def restore_launcher(command=None):
    # Another runtime may already have restored it. Do not launch duplicates.
    launchers, conflicts = display_processes()
    if launchers:
        log("Launcher is already running.")
    elif conflicts:
        log("Another board app is active; Launcher was not restarted.")
    else:
        command = command or ([LAUNCHER_DAEMON] if Path(LAUNCHER_DAEMON).is_file() else [LAUNCHER, "daemon"])
        subprocess.Popen(command, stdin=subprocess.DEVNULL,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True, close_fds=True)
        log("Launcher restart requested.")
        deadline = time.monotonic() + 5.0
        previous, stable = None, 0
        while time.monotonic() < deadline:
            restored, _ = display_processes()
            snapshot = tuple(restored)
            has_gui = any(identity[0] == LAUNCHER for _, identity in restored)
            has_daemon = any(identity[0] == LAUNCHER_DAEMON for _, identity in restored)
            ready = has_gui and (command[0] != LAUNCHER_DAEMON or has_daemon)
            stable = stable + 1 if ready and snapshot == previous else 0
            if stable >= 3:
                log("Launcher processes restored; confirm the board menu visually.")
                return
            previous = snapshot
            time.sleep(POLL_SECONDS)
        log("Launcher startup is not yet confirmed; allow the board menu to settle before Run.")


def stop_program(child):
    # child was created with start_new_session=True: this process group belongs
    # to this run, never to ws_shell, the interactive shell or an unrelated app.
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGKILL):
        if child.poll() is not None:
            return child.returncode
        try:
            os.killpg(child.pid, sig)
        except ProcessLookupError:
            pass
        try:
            return child.wait(timeout=GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            log("Program has not exited; escalating its Stop signal.")
    # A task stuck in an uninterruptible driver can outlive SIGKILL. Keep the
    # supervisor/lock and running state until wait() actually confirms exit;
    # never restore the display or print a completed-run marker prematurely.
    log("Program has not exited after termination; still waiting for the driver.")
    return child.wait()


def run(root):
    stopped = False

    def request_stop(_signum, _frame):
        nonlocal stopped
        stopped = True

    previous = {}
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        previous[sig] = signal.signal(sig, request_stop)
    child = None
    launcher_changed = False
    restore_command = None
    try:
        with open(LOCK, "a") as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise RuntimeError("Another vibeKidbright program is still running. Stop it first.")
            launchers, conflicts = display_processes()
            if conflicts:
                details = ", ".join(f"PID {pid}: {path}" for pid, path in conflicts)
                raise RuntimeError("Another program may own the display. Exit that board app first: " + details)
            try:
                if launchers:
                    restore_command = ([LAUNCHER_DAEMON] if any(identity[0] == LAUNCHER_DAEMON for _, identity in launchers)
                                       else [LAUNCHER, "daemon"])
                pending = launchers
                for _attempt in range(3):
                    if not pending:
                        break
                    for pid, identity in pending:
                        log(f"Releasing {Path(identity[0]).name} (PID {pid}).")
                        launcher_changed = True
                        stop_launcher(pid, identity)
                    pending, conflicts = display_processes()
                    if conflicts:
                        raise RuntimeError("A different board app started during display handoff. Python was not started.")
                if pending:
                    raise RuntimeError("Launcher keeps restarting; Python was not started.")
                if stopped or (root / STOP_FILE).exists():
                    return 130
                try:
                    version = importlib.metadata.version("MaixPy")
                except importlib.metadata.PackageNotFoundError:
                    version = "not reported by package metadata"
                log(f"Python {sys.version.split()[0]}; MaixPy {version}.")
                log("Starting main.py; display output still needs visual confirmation.")
                # API checks and the user program share one native MaixPy
                # process. Never import maix in this supervisor/probe process.
                child = subprocess.Popen([sys.executable, "-u", str(root / ".vibekidbright-runtime.py"), "--run"], cwd=root,
                                         stdin=subprocess.DEVNULL, start_new_session=True)
                while child.poll() is None:
                    if stopped or (root / STOP_FILE).exists():
                        log("Stopping the Python process owned by this run.")
                        stop_program(child)
                        break
                    time.sleep(POLL_SECONDS)
                result = child.wait()
                return result if result >= 0 else 128 - result
            finally:
                if child is not None and child.poll() is None:
                    stop_program(child)
                if launcher_changed and (child is None or child.poll() is not None):
                    restore_launcher(restore_command)
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def main():
    marker = sys.argv[1] if len(sys.argv) == 2 else ""
    nonce = marker[len("__VIBE_PLUS_EXIT_"):-1]
    if not nonce.isdigit() or marker != f"__VIBE_PLUS_EXIT_{nonce}:":
        raise SystemExit("Invalid Plus run marker.")
    result = 1
    try:
        result = run(Path(__file__).resolve().parent)
    except BaseException:
        traceback.print_exc()
    finally:
        print("\n" + marker + str(result), flush=True)
    raise SystemExit(result)


if __name__ == "__main__":
    main()
