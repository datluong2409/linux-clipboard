#!/usr/bin/env bash
#
# Linux Clipboard — full data wipe / uninstall helper.
#
# Run this AS YOUR NORMAL USER (not with sudo) to remove every trace the app
# leaves in your home directory:
#
#   * history database + cached images   (~/.local/share/<id>/)
#   * WebView cache / local storage       (~/.local/share/<id>/, ~/.cache/<id>/)
#   * settings                            (~/.config/<id>/)
#   * Wayland RemoteDesktop portal token  (~/.local/state/linux-clipboard/)
#   * GNOME toggle keyboard shortcut      (gsettings custom-keybinding slot)
#   * run-on-login entry                  (~/.config/autostart/*.desktop)
#
# Any running instance is stopped first, so the live app can't re-save data as
# it is wiped.
#
# It works for both install methods:
#   * .deb     — pass --remove-package to also purge the package via apt/dpkg.
#   * AppImage — cleans the per-user data; delete the .AppImage file yourself.
#
# The GNOME shortcut and autostart entry are per-user settings that a .deb's
# root-run uninstall (postrm) cannot reach — that is why this user-run script
# exists alongside `apt purge`.
#
# Usage:
#   ./uninstall.sh [--yes] [--remove-package] [--dry-run] [--help]
#
#   --yes, -y          Do not prompt for confirmation.
#   --remove-package   Also remove the .deb package (needs sudo).
#   --dry-run          Print what would be removed; change nothing.
#   --help, -h         Show this help.

set -euo pipefail

# --- Identity (must match the Rust backend; see CLAUDE.md "Data locations") ---
APP_ID="com.datluong.linuxclipboard"   # Tauri identifier → data/config/cache dir name
STATE_NAME="linux-clipboard"           # $XDG_STATE_HOME/<STATE_NAME>/portal.token
DEB_PACKAGE="linux-clipboard"          # dpkg package name
BIN_NAME="linux-clipboard"             # /usr/bin/<BIN_NAME>; process name for pkill
GNOME_SLOT="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/linux-clipboard/"
GNOME_SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
GNOME_KB_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"

ASSUME_YES=0
REMOVE_PACKAGE=0
DRY_RUN=0

# --- XDG base directories (respect overrides, fall back to the spec defaults) -
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}"
STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}"

usage() { sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    -y|--yes)          ASSUME_YES=1 ;;
    --remove-package)  REMOVE_PACKAGE=1 ;;
    --dry-run)         DRY_RUN=1 ;;
    -h|--help)         usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; echo "Try --help." >&2; exit 2 ;;
  esac
  shift
done

note() { printf '%s\n' "$*"; }

# rm wrapper honouring --dry-run.
rmrf() {
  for target in "$@"; do
    [ -e "$target" ] || continue
    if [ "$DRY_RUN" -eq 1 ]; then
      note "  [dry-run] rm -rf $target"
    else
      rm -rf "$target"
      note "  removed  $target"
    fi
  done
}

if [ "$(id -u)" -eq 0 ]; then
  note "WARNING: running as root — this cleans root's data and GNOME settings,"
  note "         not your normal user's. Re-run as your user for a full wipe."
  note ""
fi

# Detect how the app was installed (best effort) for a tailored final hint.
INSTALLED_VIA_DEB=0
if command -v dpkg-query >/dev/null 2>&1 &&
   dpkg-query -W -f='${Status}' "$DEB_PACKAGE" 2>/dev/null | grep -q "install ok installed"; then
  INSTALLED_VIA_DEB=1
fi

DATA_DIR="$DATA_HOME/$APP_ID"
CONFIG_DIR="$CONFIG_HOME/$APP_ID"
CACHE_DIR="$CACHE_HOME/$APP_ID"
STATE_DIR="$STATE_HOME/$STATE_NAME"
AUTOSTART_DIR="$CONFIG_HOME/autostart"

note "Linux Clipboard — this will remove:"
note "  data     $DATA_DIR"
note "  settings $CONFIG_DIR"
note "  cache    $CACHE_DIR"
note "  state    $STATE_DIR"
note "  GNOME toggle shortcut + run-on-login entry"
[ "$REMOVE_PACKAGE" -eq 1 ] && note "  the .deb package '$DEB_PACKAGE' (via sudo)"
note ""

if [ "$ASSUME_YES" -ne 1 ] && [ "$DRY_RUN" -ne 1 ]; then
  printf 'Continue? [y/N] '
  read -r reply
  case "$reply" in
    [yY]|[yY][eE][sS]) ;;
    *) note "Aborted."; exit 0 ;;
  esac
fi

# --- 0. Stop the running app (this user's) so it can't rewrite what we wipe --
# A live instance holds history in memory and re-saves settings/DB on exit, so
# it would undo the wipe below; stop it first. Runs as the invoking user, so it
# only touches this user's instance (the .deb's root-run prerm covers all users).
if command -v pkill >/dev/null 2>&1 && pgrep -x "$BIN_NAME" >/dev/null 2>&1; then
  note "Stopping running Linux Clipboard…"
  if [ "$DRY_RUN" -eq 1 ]; then
    note "  [dry-run] pkill -x $BIN_NAME"
  else
    pkill -TERM -x "$BIN_NAME" 2>/dev/null || true
    i=0
    while [ "$i" -lt 10 ] && pgrep -x "$BIN_NAME" >/dev/null 2>&1; do
      sleep 0.2
      i=$((i + 1))
    done
    pkill -KILL -x "$BIN_NAME" 2>/dev/null || true
    note "  stopped"
  fi
fi

# --- 1. Files: DB, images, WebView cache, settings, portal token ------------
note "Removing data files…"
rmrf "$DATA_DIR" "$CONFIG_DIR" "$CACHE_DIR" "$STATE_DIR"

# --- 2. Autostart (run-on-login) .desktop entry -----------------------------
# auto-launch names the file after the product ("Linux Clipboard.desktop") and
# does not embed the identifier, so match by name/Exec content too.
if [ -d "$AUTOSTART_DIR" ]; then
  note "Removing run-on-login entry…"
  for f in "$AUTOSTART_DIR"/*.desktop; do
    [ -e "$f" ] || continue
    if grep -qiE 'linux[- ]clipboard|com\.datluong\.linuxclipboard' "$f" 2>/dev/null; then
      rmrf "$f"
    fi
  done
fi

# --- 3. GNOME custom keyboard shortcut (dconf, per-user) --------------------
# Reset our dedicated slot, then drop it from the custom-keybindings list —
# without disturbing any other slot (e.g. 'custom0').
if command -v gsettings >/dev/null 2>&1 &&
   gsettings list-schemas 2>/dev/null | grep -qx "$GNOME_SCHEMA"; then
  current="$(gsettings get "$GNOME_SCHEMA" custom-keybindings 2>/dev/null || echo '@as []')"
  if printf '%s' "$current" | grep -qF "$GNOME_SLOT"; then
    note "Removing GNOME toggle shortcut…"
    if [ "$DRY_RUN" -eq 1 ]; then
      note "  [dry-run] gsettings reset-recursively $GNOME_KB_SCHEMA:$GNOME_SLOT"
      note "  [dry-run] drop $GNOME_SLOT from $GNOME_SCHEMA custom-keybindings"
    else
      gsettings reset-recursively "$GNOME_KB_SCHEMA:$GNOME_SLOT" 2>/dev/null || true

      # Rebuild the array without our slot (paths never contain commas).
      inner="${current#@as }"; inner="${inner#[}"; inner="${inner%]}"
      new="["; first=1
      old_ifs="$IFS"; IFS=','
      for raw in $inner; do
        item="$(printf '%s' "$raw" | sed "s/^[[:space:]]*[\"']//; s/[\"'][[:space:]]*$//")"
        [ -z "$item" ] && continue
        [ "$item" = "$GNOME_SLOT" ] && continue
        if [ "$first" -eq 1 ]; then new="$new'$item'"; first=0; else new="$new, '$item'"; fi
      done
      IFS="$old_ifs"
      new="$new]"
      [ "$new" = "[]" ] && new="@as []"
      gsettings set "$GNOME_SCHEMA" custom-keybindings "$new"
      note "  updated  $GNOME_SCHEMA custom-keybindings"
    fi
  fi
fi

# --- 4. Optionally remove the .deb package ----------------------------------
if [ "$REMOVE_PACKAGE" -eq 1 ]; then
  if [ "$INSTALLED_VIA_DEB" -eq 1 ]; then
    note "Removing package '$DEB_PACKAGE' (needs sudo)…"
    if [ "$DRY_RUN" -eq 1 ]; then
      note "  [dry-run] sudo apt-get purge -y $DEB_PACKAGE"
    elif command -v apt-get >/dev/null 2>&1; then
      sudo apt-get purge -y "$DEB_PACKAGE"
    else
      sudo dpkg -P "$DEB_PACKAGE"
    fi
  else
    note "Package '$DEB_PACKAGE' is not installed via dpkg — skipping package removal."
  fi
fi

note ""
if [ "$DRY_RUN" -eq 1 ]; then
  note "Dry run complete — nothing was changed."
else
  note "Done. Linux Clipboard data has been removed."
  if [ "$INSTALLED_VIA_DEB" -eq 1 ] && [ "$REMOVE_PACKAGE" -ne 1 ]; then
    note "The app is still installed. Remove it with:  sudo apt purge $DEB_PACKAGE"
  fi
fi
