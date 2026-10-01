#!/bin/sh
set -eu
cd /data
case "${1:-}" in
  init)
    shift
    if [ "${API_ENABLED:-0}" = "1" ]; then
      exec flock -x /data/.rustdesk-init.lock /usr/bin/rustdesk-api --initialize "$@"
    fi
    exec flock -x /data/.rustdesk-init.lock /usr/bin/rustdesk-api --initialize-keys "$@"
    ;;
  hbbs|hbbr|rustdesk-api|rustdesk-utils)
    process="$1"
    shift
    exec "/usr/bin/$process" "$@"
    ;;
  *) exec "$@" ;;
esac
