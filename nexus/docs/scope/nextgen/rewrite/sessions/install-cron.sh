#!/usr/bin/env bash
# Installs (or removes) the cron entry that drives the Nexus REWRITE loop every 5 minutes.
# The loop runs on the OS, so it survives a closed editor or a sleeping chat session.
#
#   ./install-cron.sh          install the 5-minute tick
#   ./install-cron.sh remove   remove it
#   ./install-cron.sh status   show whether it is installed + tail the log
#
# Kill switch (no crontab edit needed):  touch nexus/docs/scope/nextgen/rewrite/sessions/.loop.STOP
# Resume:                                rm   nexus/docs/scope/nextgen/rewrite/sessions/.loop.STOP
set -euo pipefail

TICK="/home/user/code/rust/starter/nexus/docs/scope/nextgen/rewrite/sessions/loop-tick.sh"
LOG="/home/user/code/rust/starter/nexus/docs/scope/nextgen/rewrite/sessions/cron.log"
MARKER="# nexus-rewrite-loop"
LINE="*/5 * * * * $TICK $MARKER"

chmod +x "$TICK"

case "${1:-install}" in
  install)
    # Drop any prior copy, then append ours. Idempotent.
    ( crontab -l 2>/dev/null | grep -v "$MARKER" || true; echo "$LINE" ) | crontab -
    echo "Installed. The loop fires every 5 minutes:"
    echo "  $LINE"
    echo "Watch it:   tail -f $LOG"
    echo "Pause it:   touch $(dirname "$TICK")/.loop.STOP"
    echo "Stop it:    $0 remove"
    ;;
  remove)
    crontab -l 2>/dev/null | grep -v "$MARKER" | crontab - || true
    echo "Removed the nexus-rewrite-loop cron entry."
    ;;
  status)
    if crontab -l 2>/dev/null | grep -q "$MARKER"; then
      echo "INSTALLED:"; crontab -l | grep "$MARKER"
    else
      echo "NOT installed."
    fi
    [[ -f "$LOG" ]] && { echo "--- last 15 log lines ---"; tail -15 "$LOG"; }
    ;;
  *)
    echo "usage: $0 [install|remove|status]" >&2; exit 2;;
esac
