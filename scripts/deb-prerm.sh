#!/bin/sh
#
# Debian maintainer script: prerm (installed into the .deb by Tauri via
# `bundle.linux.deb.preRemoveScript` in tauri.conf.json).
#
# dpkg runs this as root BEFORE the package files are removed, on both
# `remove` (which is also the remove phase of `apt purge` / `dpkg -P`) and
# `upgrade`. Without it, a running tray instance keeps executing from the
# now-deleted binary, so removal leaves a zombie tray icon behind, and on
# upgrade the stale old process still owns the tauri single-instance lock —
# the new binary's `--toggle` would just forward to it. So stop every running
# instance first (all users' — we are root here).
#
# Best-effort throughout: never fail the removal, so this always exits 0.

# /usr/bin/linux-clipboard. The process name (comm) the kernel exposes is
# capped at 15 chars — "linux-clipboard" is exactly 15, so `pkill -x` matches
# it in full. (The sibling `linux-clipboard-uninstall` helper is a shell
# script, so its comm is the interpreter, not this name — never a false hit.)
BIN_NAME="linux-clipboard"

stop_app() {
  command -v pkill >/dev/null 2>&1 || return 0

  # Ask it to quit first so it can pull its tray icon down and exit cleanly.
  pkill -TERM -x "$BIN_NAME" 2>/dev/null || true

  # Give it up to ~2s to go away, then force-kill any survivor.
  if command -v pgrep >/dev/null 2>&1; then
    i=0
    while [ "$i" -lt 10 ] && pgrep -x "$BIN_NAME" >/dev/null 2>&1; do
      sleep 0.2
      i=$((i + 1))
    done
  else
    sleep 1
  fi
  pkill -KILL -x "$BIN_NAME" 2>/dev/null || true
}

case "$1" in
  remove|upgrade)
    stop_app
    ;;
esac

exit 0
