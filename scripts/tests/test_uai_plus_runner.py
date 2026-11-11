"""Supervisor control-flow checks; mocks do not certify Maix drivers/hardware."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import MagicMock, call, patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("plus_runner", ROOT / "src-tauri/src/uai_plus_runner.py")
runner = importlib.util.module_from_spec(spec)
with patch.dict(sys.modules, {"fcntl": types.SimpleNamespace(LOCK_EX=2, LOCK_NB=4, flock=lambda *_: None)}):
    spec.loader.exec_module(runner)


class PlusSupervisor(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.patches = [
            patch.object(runner, "LOCK", str(self.root / "lock")),
            patch.object(runner, "signal", types.SimpleNamespace(SIGINT=2, SIGTERM=15, SIGHUP=1,
                                                                SIGKILL=9, signal=MagicMock(return_value=None))),
            patch.object(runner, "log"),
            patch.object(runner.importlib.metadata, "version", return_value="fixture"),
        ]
        for item in self.patches:
            item.start()
            self.addCleanup(item.stop)

    def child(self, result=0):
        child = MagicMock(pid=321, returncode=result)
        child.poll.return_value = result
        child.wait.return_value = result
        return child

    def test_launcher_releases_before_python_and_restores_after_exit(self):
        actions = []
        identity = (runner.LAUNCHER, "start-1")
        with patch.object(runner, "display_processes", side_effect=[([(77, identity)], []), ([], [])]), \
             patch.object(runner, "stop_launcher", side_effect=lambda *_: actions.append("release")), \
             patch.object(runner.subprocess, "Popen", side_effect=lambda *a, **kw: actions.append(("run", a, kw)) or self.child()), \
             patch.object(runner, "restore_launcher", side_effect=lambda command: actions.append("restore")):
            self.assertEqual(runner.run(self.root), 0)
        self.assertEqual(actions[0], "release")
        self.assertEqual(actions[-1], "restore")
        command = actions[1]
        self.assertEqual(command[1][0], [sys.executable, "-u", str(self.root / ".vibekidbright-runtime.py"), "--run"])
        self.assertTrue(command[2]["start_new_session"])

    def test_unrelated_board_app_blocks_run_without_signalling_any_process(self):
        with patch.object(runner, "display_processes", return_value=([], [(19, "/maixapp/apps/camera/main.py")])), \
             patch.object(runner, "stop_launcher") as stop, patch.object(runner.subprocess, "Popen") as launch:
            with self.assertRaisesRegex(RuntimeError, "Exit that board app first"):
                runner.run(self.root)
            stop.assert_not_called()
            launch.assert_not_called()

    def test_launcher_is_restored_if_python_cannot_start(self):
        with patch.object(runner, "display_processes", side_effect=[([(77, (runner.LAUNCHER, "start"))], []), ([], [])]), \
             patch.object(runner, "stop_launcher"), \
             patch.object(runner.subprocess, "Popen", side_effect=OSError("no Python")), \
             patch.object(runner, "restore_launcher") as restore:
            with self.assertRaisesRegex(OSError, "no Python"):
                runner.run(self.root)
            restore.assert_called_once()

    def test_control_file_stops_owned_program_even_when_it_ignores_ctrl_c(self):
        (self.root / runner.STOP_FILE).touch()
        with patch.object(runner, "display_processes", return_value=([], [])), \
             patch.object(runner.subprocess, "Popen") as launch:
            self.assertEqual(runner.run(self.root), 130)
            launch.assert_not_called()
        (self.root / runner.STOP_FILE).unlink()
        child = self.child()
        child.poll.side_effect = [None, 0, 0]
        def launch(*_args, **_kwargs):
            (self.root / runner.STOP_FILE).touch()
            return child
        with patch.object(runner, "display_processes", return_value=([], [])), \
             patch.object(runner.subprocess, "Popen", side_effect=launch), \
             patch.object(runner, "stop_program") as stop:
            runner.run(self.root)
            stop.assert_called_once_with(child)

    def test_stop_escalates_only_its_child_process_group(self):
        child = self.child(-9)
        child.poll.return_value = None
        child.wait.side_effect = [subprocess.TimeoutExpired("fixture", 3),
                                  subprocess.TimeoutExpired("fixture", 3), -9]
        with patch.object(runner.os, "killpg", create=True) as kill:
            self.assertEqual(runner.stop_program(child), -9)
        self.assertEqual(kill.call_args_list, [call(321, 2), call(321, 15), call(321, 9)])

    def test_reused_pid_does_not_receive_launcher_signal(self):
        with patch.object(runner, "process_identity", return_value=("/usr/bin/python3", "different")), \
             patch.object(runner.os, "kill") as kill:
            runner.stop_launcher(77, (runner.LAUNCHER, "original"))
            kill.assert_not_called()

    def test_process_scan_distinguishes_launcher_service_and_other_runs(self):
        entries = []
        identities = {}
        for pid, exe, argv, cwd in [
            (77, runner.LAUNCHER, [runner.LAUNCHER, "daemon"], "/root"),
            (78, "/usr/bin/python3", ["python3", "/root/ws_shell.py"], "/root"),
            (79, "/usr/bin/python3", ["python3", "-c", "encoded"], "/tmp/vibekidbright-1234"),
            (80, "/usr/bin/python3", ["python3", "/maixapp/apps/camera/main.py"], "/root"),
            (81, "/maixapp/apps/launcher/launcher_daemon", ["launcher_daemon"], "/root"),
            (82, "/opt/launcher-copy", ["launcher"], "/root"),
        ]:
            entry = MagicMock()
            entry.name = str(pid)
            cmd, directory = MagicMock(), MagicMock()
            cmd.read_bytes.return_value = b"\0".join(arg.encode() for arg in argv)
            directory.resolve.return_value.as_posix.return_value = cwd
            entry.__truediv__.side_effect = lambda part, cmd=cmd, directory=directory: cmd if part == "cmdline" else directory
            entries.append(entry)
            identities[pid] = (exe, str(pid))
        proc = MagicMock()
        proc.iterdir.return_value = entries
        with patch.object(runner, "process_identity", side_effect=lambda pid, _proc: identities[pid]), \
             patch.object(runner.os, "getpid", return_value=1000):
            launchers, conflicts = runner.display_processes(proc)
        self.assertEqual([pid for pid, _ in launchers], [81, 77])
        self.assertEqual([pid for pid, _ in conflicts], [79, 80])

    def test_runtime_124_stops_daemon_before_gui_and_restores_daemon_entrypoint(self):
        actions = []
        processes = [(81, (runner.LAUNCHER_DAEMON, "daemon-start")), (77, (runner.LAUNCHER, "gui-start"))]
        with patch.object(runner, "display_processes", side_effect=[(processes, []), ([], [])]), \
             patch.object(runner, "stop_launcher", side_effect=lambda pid, identity: actions.append(pid)), \
             patch.object(runner.subprocess, "Popen", return_value=self.child()), \
             patch.object(runner, "restore_launcher") as restore:
            runner.run(self.root)
            self.assertEqual(actions, [81, 77])
            restore.assert_called_once_with([runner.LAUNCHER_DAEMON])

    def test_respawned_gui_is_released_before_user_python_starts(self):
        first = [(77, (runner.LAUNCHER, "old"))]
        respawn = [(78, (runner.LAUNCHER, "new"))]
        with patch.object(runner, "display_processes", side_effect=[(first, []), (respawn, []), ([], [])]), \
             patch.object(runner, "stop_launcher") as stop, \
             patch.object(runner.subprocess, "Popen", return_value=self.child()) as launch, \
             patch.object(runner, "restore_launcher"):
            runner.run(self.root)
        self.assertEqual([item.args[0] for item in stop.call_args_list], [77, 78])
        launch.assert_called_once()

    def test_continuous_launcher_respawn_blocks_native_python(self):
        processes = [(77, (runner.LAUNCHER, "restart"))]
        with patch.object(runner, "display_processes", return_value=(processes, [])), \
             patch.object(runner, "stop_launcher"), patch.object(runner.subprocess, "Popen") as launch, \
             patch.object(runner, "restore_launcher"):
            with self.assertRaisesRegex(RuntimeError, "keeps restarting"):
                runner.run(self.root)
        launch.assert_not_called()

    def test_uninterruptible_child_is_not_reported_as_exited_after_timeout(self):
        child = self.child()
        child.poll.return_value = None
        child.wait.side_effect = [subprocess.TimeoutExpired("fixture", 3)] * 3 + [0]
        with patch.object(runner.os, "killpg", create=True):
            self.assertEqual(runner.stop_program(child), 0)
        self.assertEqual(child.wait.call_args_list[-1], call())

    def test_lock_prevents_concurrent_maix_initialization(self):
        with patch.object(runner.fcntl, "flock", side_effect=BlockingIOError()), \
             patch.object(runner, "display_processes") as scan:
            with self.assertRaisesRegex(RuntimeError, "still running"):
                runner.run(self.root)
            scan.assert_not_called()

    def test_nonzero_exit_and_cleanup_are_preserved(self):
        with patch.object(runner, "display_processes", return_value=([], [])), \
             patch.object(runner.subprocess, "Popen", return_value=self.child(7)), \
             patch.object(runner, "restore_launcher") as restore:
            self.assertEqual(runner.run(self.root), 7)
            restore.assert_not_called()

    def test_signal_handlers_are_restored_on_launch_failure(self):
        with patch.object(runner, "display_processes", side_effect=OSError("proc unavailable")):
            with self.assertRaises(OSError):
                runner.run(self.root)
        self.assertEqual(runner.signal.signal.call_count, 6)


if __name__ == "__main__":
    unittest.main()
