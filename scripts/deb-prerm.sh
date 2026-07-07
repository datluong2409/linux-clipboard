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

# The installed binary. We match running instances by their resolved
# executable path (/proc/<pid>/exe), NEVER by process name.
#
# Why not `pkill -x linux-clipboard`: the kernel's process name (comm) is
# capped at 15 chars and, for a script started via its shebang, is the
# SCRIPT's own filename — not the interpreter. dpkg installs this script as
# "linux-clipboard.prerm", whose first 15 chars are exactly "linux-clipboard",
# so `pkill -x linux-clipboard` would match (and SIGTERM) this very prerm,
# aborting removal with "prerm subprocess was killed by signal (Terminated)".
# Skipping our own PID and confirming the exe path sidesteps that collision.
BIN_PATH="/usr/bin/linux-clipboard"

# Print the PID of every running app instance, never including this script's
# own PID. pgrep -x is a cheap first pass (the real app's comm does equal the
# binary name); the exe check is what makes it correct.
app_pids() {
  for pid in $(pgrep -x linux-clipboard 2>/dev/null || true); do
    [ "$pid" = "$$" ] && continue
    if [ "$(readlink -f "/proc/$pid/exe" 2>/dev/null)" = "$BIN_PATH" ]; then
      printf '%s\n' "$pid"
    fi
  done
}

stop_app() {
  command -v pgrep >/dev/null 2>&1 || return 0

  # Ask it to quit first so it can pull its tray icon down and exit cleanly.
  for pid in $(app_pids); do kill -TERM "$pid" 2>/dev/null || true; done

  # Give it up to ~2s to go away, then force-kill any survivor.
  i=0
  while [ "$i" -lt 10 ] && [ -n "$(app_pids)" ]; do
    sleep 0.2
    i=$((i + 1))
  done
  for pid in $(app_pids); do kill -KILL "$pid" 2>/dev/null || true; done
}

case "$1" in
  remove|upgrade)
    stop_app
    ;;
esac

exit 0
