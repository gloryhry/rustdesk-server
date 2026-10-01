"""Check actual CLI consumers with pipes and a Unix pseudo-terminal; no service startup."""
import errno
import os
import pty
import subprocess
import sys
import tempfile

with tempfile.TemporaryDirectory(prefix='rustdesk-terminal-check-') as directory:
    env = {'HOME': directory, 'XDG_CONFIG_HOME': directory, 'TERM': 'xterm'}
    for binary in sys.argv[1:]:
        command = [binary, '--invalid-argument-for-regression']
        result = subprocess.run(command, cwd=directory, env=env, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
        assert result.returncode != 0 and b'error:' in result.stderr
        assert b'\x1b[' not in result.stdout + result.stderr
        master, slave = pty.openpty()
        try:
            result = subprocess.run(command, cwd=directory, env=env, stdin=subprocess.DEVNULL,
                                    stdout=slave, stderr=slave, timeout=5)
            assert result.returncode != 0
            os.close(slave)
            slave = None
            output = b''
            while True:
                try:
                    chunk = os.read(master, 4096)
                    if not chunk:
                        break
                    output += chunk
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
            assert b'error:' in output and b'\x1b[' in output
        finally:
            os.close(master)
            if slave is not None:
                os.close(slave)
