#!/bin/bash
set -eu
if [ "$(id -u)" = 0 ]; then
  if [ ! -f /home/tester/.goal-machine-id ]; then
    dbus-uuidgen > /home/tester/.goal-machine-id
  fi
  cp /home/tester/.goal-machine-id /etc/machine-id
  cp /home/tester/.goal-machine-id /var/lib/dbus/machine-id
  if [ -f /goal/ca.crt ]; then
    cp /goal/ca.crt /usr/local/share/ca-certificates/goal.crt
    update-ca-certificates >/dev/null
  fi
  mkdir -p /run/dbus /home/tester/.config/rustdesk /home/tester/evidence /home/tester/transfers /home/tester/Documents
  rm -f /run/dbus/pid /tmp/.X99-lock /tmp/.X11-unix/X99
  dbus-daemon --system --fork
  # This mount is the container's private cgroup namespace, never a host bind.
  mount -o remount,rw /sys/fs/cgroup
  /usr/lib/elogind/elogind > /home/tester/evidence/elogind.log 2>&1 &
  for attempt in $(seq 1 100); do loginctl list-sessions >/dev/null 2>&1 && break; sleep 0.1; done
  if [ ! -f /home/tester/.config/rustdesk/RustDesk2.toml ]; then
    python3 - <<'PY'
import json, os
from pathlib import Path
options = {'custom-rendezvous-server': os.environ['GOAL_ID_SERVER'],
           'relay-server': os.environ['GOAL_RELAY_SERVER'],
           'api-server': 'https://api.goal.test', 'key': os.environ['GOAL_SERVER_KEY']}
Path('/home/tester/.config/rustdesk/RustDesk2.toml').write_text(
    '[options]\n' + ''.join(json.dumps(k) + ' = ' + json.dumps(v) + '\n' for k,v in options.items()))
PY
  fi
  chown -R tester:tester /home/tester
  export XDG_SESSION_TYPE=x11 XDG_SESSION_CLASS=user XDG_SESSION_DESKTOP=openbox XDG_SEAT=seat0
  exec runuser -u tester -- dbus-run-session -- /opt/goal/desktop.sh
fi
mkdir -p /home/tester/.pki/nssdb
mkdir -p /home/tester/.local/share/applications /home/tester/.config
cat > /home/tester/.local/share/applications/goal-browser.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Goal test browser
Exec=google-chrome --no-sandbox --no-first-run --no-default-browser-check --disable-dev-shm-usage --user-data-dir=/home/tester/.config/goal-browser %U
MimeType=x-scheme-handler/http;x-scheme-handler/https;
EOF
cat > /home/tester/.config/mimeapps.list <<'EOF'
[Default Applications]
x-scheme-handler/http=goal-browser.desktop
x-scheme-handler/https=goal-browser.desktop
EOF
if [ ! -f /home/tester/.pki/nssdb/cert9.db ]; then
  certutil -N --empty-password -d sql:/home/tester/.pki/nssdb
fi
if [ -f /goal/ca.crt ]; then
  certutil -A -d sql:/home/tester/.pki/nssdb -n goal-test-ca -t 'C,,' -i /goal/ca.crt
fi
Xvfb :99 -screen 0 1280x900x24 -ac -nolisten tcp > /home/tester/evidence/xvfb.log 2>&1 &
for attempt in $(seq 1 100); do xdpyinfo >/dev/null 2>&1 && break; sleep 0.1; done
openbox > /home/tester/evidence/openbox.log 2>&1 &
x11vnc -display :99 -localhost -nopw -forever -shared -rfbport 5900 > /home/tester/evidence/vnc.log 2>&1 &
websockify --web /usr/share/novnc/ 6080 localhost:5900 > /home/tester/evidence/novnc.log 2>&1 &
rustdesk > /home/tester/evidence/client.log 2>&1 &
exec sleep infinity
