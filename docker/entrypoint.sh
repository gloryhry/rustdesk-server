#!/bin/sh
set -eu
cd /data
case "${1:-}" in
  init)
    shift
    exec flock -x /data/.rustdesk-init.lock /usr/bin/rustdesk-api --initialize "$@"
    ;;
  hbbs|hbbr|rustdesk-api|rustdesk-utils)
    process="$1"
    shift
    exec "/usr/bin/$process" "$@"
    ;;
  *) exec "$@" ;;
esac
