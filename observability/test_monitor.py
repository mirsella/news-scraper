import io
import os
import struct
import unittest
from unittest.mock import patch

os.environ.setdefault("POSTHOG_PROJECT_TOKEN", "phc_test")
import monitor


def frame(body):
    return b"\x01\0\0\0" + struct.pack(">I", len(body)) + body


class MonitorTests(unittest.TestCase):
    def test_full_interval_not_just_last_200_lines(self):
        data = frame(b"level=error first\n") + frame(b"level=info ok\n") * 250
        lines = list(monitor.unpack_logs(io.BytesIO(data)))
        self.assertEqual(len(lines), 251)
        self.assertEqual(monitor.log_severity(lines[0]), 17)

    def test_split_lines_and_truncated_frame(self):
        data = frame(b"level=warn par") + frame(b"tial\nnext\n")
        self.assertEqual(
            list(monitor.unpack_logs(io.BytesIO(data))), ["level=warn partial", "next"]
        )
        with self.assertRaisesRegex(ValueError, "Truncated"):
            list(monitor.unpack_logs(io.BytesIO(frame(b"message")[:-1])))

    def test_actual_proxy_severities(self):
        for level in ("warning", "warn", "WRN"):
            self.assertEqual(
                monitor.log_severity(f"2026-11-09 level={level} problem"), 13
            )
        for level in ("error", "ERR", "panic", "FATAL"):
            self.assertEqual(monitor.log_severity(f"2026-11-09 {level} problem"), 17)
        self.assertIsNone(monitor.log_severity("level=info normal"))

    def test_delivery_failure_does_not_allow_cursor_commit(self):
        with patch.object(monitor, "post", return_value=False):
            with self.assertRaisesRegex(RuntimeError, "retaining log cursors"):
                monitor.deliver("traefik", [monitor.event("test", "traefik")], [])


if __name__ == "__main__":
    unittest.main()
