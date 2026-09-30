#!/bin/sh
set -eu
/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/hbbs >/dev/null
/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/hbbr >/dev/null
nc -z -w 2 127.0.0.1 21116
nc -z -w 2 127.0.0.1 21117
/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/rustdesk-api >/dev/null
curl --fail --silent --max-time 3 "http://127.0.0.1:${API_PORT:-21114}/health/ready" >/dev/null
