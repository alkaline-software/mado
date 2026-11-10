#!/bin/sh
# Quit the dev build of Mado and start the build in target/debug again,
# a few seconds after the agent's turn that ran this is over, and return at
# once. For an agent working inside the app: when the agent's session is a
# child of the app, the app cannot be quit while the turn is still being
# written. The build, once open, ends any other Mado, the installed one
# included: two would each take the shortcuts (`sys::end_other_copies`).
#
#   scripts/relaunch.sh [seconds]                 default 15
#   scripts/relaunch.sh --installed [seconds]
#
# With --installed what is started is /Applications/Mado.app, not the
# build: the end of a release, when target/ has been deleted
# (scripts/release-clean.sh).
#
# The wait is a state and then the countdown. The person reads the reply
# and types the next message in those seconds; counted from when the script
# ran, they have passed by the time a long reply is written, and the app
# quits under the person's typing. So the countdown starts once the turn is
# over, and what says so is RELAUNCH_WHILE: a shell command that succeeds
# for as long as the turn runs. A product whose agent keeps a record of its
# session sets it to a test of that record, for instance
#
#   RELAUNCH_WHILE='grep -q "\"status\":\"busy\"" /path/to/the/session/record'
#
# found by walking up from this script's process to the agent's. Without
# it the script waits for the shell that ran it to exit, which is the end
# of the agent's command and not of its turn, and the seconds should be
# raised to cover the reply.

cd "$(dirname "$0")/.." || exit 1
installed=""
[ "$1" = --installed ] && { installed=1; shift; }
delay="${1:-15}"

if [ "$(uname)" = Darwin ]; then
    dev="$(pwd)/target/debug/dev.noindex/Mado.app/Contents/MacOS/Mado"
else
    [ -n "$installed" ] && { echo "relaunch: --installed is for a Mac, where the release leaves /Applications/Mado.app" >&2; exit 1; }
    dev="$(pwd)/target/debug/Mado"
fi

DEV="$dev" CALLER="$PPID" WHILE="${RELAUNCH_WHILE:-}" DELAY="$delay" INSTALLED="$installed" nohup sh -c '
    n=0
    if [ -n "$WHILE" ]; then
        while sh -c "$WHILE" >/dev/null 2>&1 && [ "$n" -lt 1800 ]; do
            sleep 1
            n=$((n + 1))
        done
    else
        while kill -0 "$CALLER" 2>/dev/null && [ "$n" -lt 1800 ]; do
            sleep 1
            n=$((n + 1))
        done
    fi
    sleep "$DELAY"
    if [ -z "$INSTALLED" ]; then
        pkill -a -f "$DEV"
        sleep 2
        exec scripts/dev-app.sh
    fi
    pkill -x Mado
    sleep 2
    # As dev-app.sh opens the build: the environment a Dock launch has,
    # and none of the variables of the agent that ran this.
    exec env -i \
        HOME="$HOME" USER="$USER" LOGNAME="${LOGNAME:-$USER}" SHELL="${SHELL:-/bin/zsh}" \
        PATH=/usr/bin:/bin:/usr/sbin:/sbin \
        TMPDIR="$(getconf DARWIN_USER_TEMP_DIR)" \
        SSH_AUTH_SOCK="$(launchctl getenv SSH_AUTH_SOCK)" \
        /usr/bin/open -n /Applications/Mado.app
' >/dev/null 2>&1 &
if [ -n "$installed" ]; then
    echo "The installed Mado opens ${delay}s after this turn ends"
    exit 0
fi
echo "Mado relaunches ${delay}s after this turn ends"
