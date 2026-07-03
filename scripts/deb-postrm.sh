#!/bin/sh
#
# Debian maintainer script: postrm (installed into the .deb by Tauri via
# `bundle.linux.deb.postRemoveScript` in tauri.conf.json).
#
# dpkg runs this as root. On `purge` (`apt purge` / `dpkg -P`) we delete the
# per-user data Linux Clipboard writes into every user's home directory:
#
#   ~/.local/share/com.datluong.linuxclipboard/   (history db, images, WebView)
#   ~/.config/com.datluong.linuxclipboard/         (settings.json)
#   ~/.cache/com.datluong.linuxclipboard/          (WebView cache)
#   ~/.local/state/linux-clipboard/                (Wayland portal token)
#   ~/.config/autostart/*.desktop                  (run-on-login entry)
#
# It cannot touch dconf (the GNOME toggle shortcut) — that is a per-user
# setting reachable only from the user's session, so the shipped
# `linux-clipboard-uninstall` helper handles that. See scripts/uninstall.sh.
#
# Following Debian policy, `remove` keeps user data; only `purge` wipes it.
# Best-effort throughout: never fail the removal, so this always exits 0.

APP_ID="com.datluong.linuxclipboard"
STATE_NAME="linux-clipboard"

purge_user_data() {
  home="$1"
  [ -n "$home" ] || return 0
  [ -d "$home" ] || return 0

  rm -rf "$home/.local/share/$APP_ID" \
         "$home/.config/$APP_ID" \
         "$home/.cache/$APP_ID" \
         "$home/.local/state/$STATE_NAME" 2>/dev/null || true

  autostart="$home/.config/autostart"
  if [ -d "$autostart" ]; then
    for f in "$autostart"/*.desktop; do
      [ -e "$f" ] || continue
      if grep -qiE 'linux[- ]clipboard|com\.datluong\.linuxclipboard' "$f" 2>/dev/null; then
        rm -f "$f" 2>/dev/null || true
      fi
    done
  fi
}

case "$1" in
  purge)
    if command -v getent >/dev/null 2>&1; then
      # Enumerate real home dirs. getent (not a /home/* glob) is required
      # because homes can be nested, e.g. domain users at /home/domain/user.
      # The /home/*|/root allowlist keeps us clear of system accounts.
      getent passwd | while IFS=':' read -r _login _pw _uid _gid _gecos home _shell; do
        case "$home" in
          /home/*|/root) purge_user_data "$home" ;;
        esac
      done
    else
      for home in /root /home/*; do
        purge_user_data "$home"
      done
    fi
    ;;
  remove|upgrade|failed-upgrade|abort-install|abort-upgrade|disappear)
    # Keep user data on plain remove/upgrade (Debian policy).
    ;;
esac

exit 0
