# Original Linux client acceptance

This harness provisions two disposable Linux desktops, original RustDesk client
binaries, HTTPS API, and Keycloak 26.4.0. It does not simulate client protocols.
Use an official 1.4.9 amd64 Debian package and verify its SHA-256 against the
GitHub release asset digest before building. Place it as `client.deb` and an
official Google Chrome Debian package as `browser.deb` in a separate build context
alongside `Dockerfile`, `desktop.sh`, and `desktop_probe.py` from this directory.

```sh
docker build --target runtime -f docker/Dockerfile -t <unique-server-image> .
docker build --target supervisor -f docker/Dockerfile -t <unique-server-image>-s6 .
docker build -t <unique-desktop-image> <desktop-build-context>
python3 tests/native/run.py start --directory <private-run-directory> \
  --image <unique-server-image> --client-image <unique-desktop-image>
```

The manifest records exact containers, volumes, network, image identities, desktop
URLs and random file fixture hashes. Credentials are generated with mode 0600.
Only loopback noVNC and the fixture-control HTTP endpoint are published. Actual
Web/native login uses HTTPS with a test CA installed only in the desktop
containers. Chrome uses a dedicated profile and normal certificate validation.

Each desktop uses a real elogind/PAM X11 user session. Containers need a private
cgroup namespace, SYS_ADMIN and an unconfined AppArmor profile to remount their
own cgroup namespace writable. No host cgroup or client configuration is mounted.
These privileges belong only to the disposable desktops; server containers keep
their ordinary runtime settings. The client executable and shared libraries are
never patched. `loginctl show-session c1` must show user tester, type x11, state
active. Without a real console session, the original client refuses file
transfer despite working screen capture.

Open the manifest's noVNC URLs to operate the original client UI. Complete the
acceptance matrix in the goal report. For remote input, start
`python3 /opt/goal/desktop_probe.py` as tester on B, then type a unique marker and
click its button **through A's RustDesk remote session**. The visible changing
clock and recorded keyboard/mouse events provide independent evidence.

Direct connections require client connection logs and an A-to-B TCP socket.
Use the original client's `/r` ID suffix for forced relay, and record hbbr's
paired session, traffic, and both clients' sockets to hbbr. A listening port or
successful connection message alone is insufficient.

For each mode, send A's `transfers/source` folder into B's
`transfers/<mode>-from-a`, and receive B's original source into A's
`transfers/<mode>-from-b`, exclusively through the client UI. Create empty target
directories beforehand. After the UI transfer finishes, verify actual bytes:

```sh
python3 tests/native/run.py verify-transfers --directory <private-run-directory> --mode direct
python3 tests/native/run.py verify-transfers --directory <private-run-directory> --mode relay
python3 tests/native/run.py collect --directory <private-run-directory>
python3 tests/native/run.py cleanup --directory <private-run-directory>
```

The verifier compares destination hashes and sizes with source files and the
original provisioning hashes; it never copies files. Collection redacts JWTs,
credentials and OAuth query secrets. Inspect screenshots separately to exclude
passwords. Cleanup removes only manifest resources and verifies containers,
volumes and network are absent, then deletes private credentials and CA keys.
Screenshots, hashes, sanitized logs and manifest are retained. An interrupted or
unexecuted required flow remains pending; provisioning is not full acceptance.
