#!/bin/sh

/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/hbbr || exit 1
/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/hbbs || exit 1

if [ "${API_ENABLED}" = "1" ]; then
  /package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/rustdesk-api || exit 1
  /usr/bin/wget -q -T 3 -O - http://127.0.0.1:${API_PORT:-21114}/health/live >/dev/null || exit 1
fi
