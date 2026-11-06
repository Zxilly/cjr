"""Run process regressions in a controlling terminal (Unix; requires pexpect).

This exercises terminal foreground transfer/restoration that redirected CI
stdio cannot cover. Usage: python3 scripts/terminal_acceptance.py
"""

import os
import sys

import pexpect


def main():
    child = pexpect.spawn(
        "cargo",
        [
            "test",
            "--locked",
            "--test",
            "process",
            "--",
            "--nocapture",
            "--test-threads=1",
        ],
        env=dict(
            os.environ,
            CJR_TEST_REQUIRE_TTY="1",
            CARGO_TERM_COLOR="never",
            CARGO_TERM_PROGRESS_WHEN="never",
        ),
        encoding="utf-8",
        timeout=120,
    )
    child.logfile_read = sys.stdout
    try:
        child.expect(pexpect.EOF)
    finally:
        child.close(force=True)
    if child.signalstatus is not None or child.exitstatus != 0:
        raise SystemExit(
            f"terminal regression failed: exit={child.exitstatus}, signal={child.signalstatus}"
        )


if __name__ == "__main__":
    main()
