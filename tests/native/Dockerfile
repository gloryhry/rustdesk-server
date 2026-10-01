FROM debian:bookworm-slim AS login-runtime
RUN apt-get update && apt-get install -y --no-install-recommends elogind libpam-elogind

FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl openssl python3 python3-tk dbus dbus-x11 xvfb openbox xterm \
    x11vnc novnc websockify xdotool wmctrl x11-utils iproute2 tcpdump \
    libgtk-3-0t64 libxcb-randr0 libxdo3 libxfixes3 libxcb-shape0 libxcb-xfixes0 \
    libasound2t64 libsystemd0 libva2 libva-drm2 libva-x11-2 libegl1 libgl1 libgl1-mesa-dri \
    libgstreamer-plugins-base1.0-0 gstreamer1.0-pipewire libnss3-tools \
    fonts-dejavu fonts-noto-cjk && rm -rf /var/lib/apt/lists/*
COPY client.deb browser.deb /tmp/
RUN dpkg-deb -x /tmp/client.deb / && apt-get update && \
    apt-get install -y --no-install-recommends /tmp/browser.deb && \
    rm -rf /var/lib/apt/lists/* /tmp/*.deb && useradd -m -u 1001 tester
COPY --from=login-runtime /lib/elogind/ /usr/lib/elogind/
COPY --from=login-runtime /lib/x86_64-linux-gnu/libelogind.so.0.29.0 /usr/lib/x86_64-linux-gnu/
COPY --from=login-runtime /lib/x86_64-linux-gnu/security/pam_elogind.so /usr/lib/x86_64-linux-gnu/security/
COPY --from=login-runtime /bin/loginctl /usr/bin/loginctl
COPY --from=login-runtime /usr/share/dbus-1/system.d/org.freedesktop.login1.conf /usr/share/dbus-1/system.d/
RUN ln -s libelogind.so.0.29.0 /usr/lib/x86_64-linux-gnu/libelogind.so.0 && \
    printf '\nsession required pam_elogind.so\n' >> /etc/pam.d/runuser && ldconfig
COPY desktop.sh /opt/goal/desktop.sh
COPY desktop_probe.py /opt/goal/desktop_probe.py
RUN chmod 755 /opt/goal/desktop.sh && ln -s /usr/share/rustdesk/rustdesk /usr/bin/rustdesk
ENV DISPLAY=:99 LIBGL_ALWAYS_SOFTWARE=1
EXPOSE 6080
ENTRYPOINT ["/opt/goal/desktop.sh"]
