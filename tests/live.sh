#!/bin/zsh
# Live end-to-end checks against the running Grids, using only the self-test window.
cd "${0:A:h}/.."
D=build/obj/drive
# Actions without a target hit the FRONT window, so these tests only run once the test window
# is verifiably frontmost; otherwise they would move one of your real windows.
open -n build/obj/TestWindow.app; sleep 1.5
front_ok() { [[ $($D front) == app.grids.testwindow ]] }
read vx vy vw vh <<< "$($D screen)"
pass=0; fail=0
W=$($D wid)

# Cursor-driven steps act on whatever window is under the cursor; only run them when the
# test window is the topmost window at every point the step touches.
at() { # at <drive cmd> X Y ...: run a cursor step only if the test window is on top at X,Y
  if mine "$2,$3"; then $D "$@"; else echo "SKIP (test window not on top at $2,$3): $1"; return 1; fi
}
mine() { for pt in "$@"; do [[ $($D topat ${pt%,*} ${pt#*,}) == $W ]] || return 1; done }
check() { # name expected-x expected-w
  read x y w h <<< "$($D bounds)"
  if (( ${x:-0} >= $2 - 3 && ${x:-0} <= $2 + 3 && ${w:-0} >= $3 - 3 && ${w:-0} <= $3 + 3 )); then pass=$((pass+1)); echo "ok   $1 ($x $y $w $h)"
  else fail=$((fail+1)); echo "FAIL $1: got $x $y $w $h, want x=$2 w=$3"; fi
}
half=$((vw / 2))
front() { # run a front-window step only while the test window is in front
  if front_ok; then "$@"; else echo "SKIP (test window not frontmost): $*"; skipped=1; fi
}
# Real arrow keys carry the fn and keypad flags (0x800000, 0x200000).
front $D key 123 11272192; sleep 0.4; front_ok && check "hotkey ⌃⌥← left half" $vx $half
front $D key 124 11272192; sleep 0.4; front_ok && check "hotkey ⌃⌥→ right half" $((vx + vw - half)) $half
front $D key 36 786432; sleep 0.4; front_ok && check "hotkey ⌃⌥↩ maximize" $vx $vw
# URL actions name the test window explicitly, so focus never matters.
W=$($D wid)
open -g "grids://execute-action?name=first-third&window=$W"; sleep 0.6; check "URL first-third" $vx $((vw / 3))
open -g "rectangle-pro://execute-action?name=last-two-thirds&window=$W"; sleep 0.6; check "rectangle-pro URL last-two-thirds" $((vx + vw - vw * 2 / 3)) $((vw * 2 / 3))
open -g "grids://execute-action?name=center&window=$W"; sleep 0.6
read x y w h <<< "$($D bounds)"
at drag $((x + w / 2)) $((y + 12)) $((vx + 2)) $((vy + vh / 2)) && { sleep 0.8; check "drag to left edge snaps left half" $vx $half; }
read x y w h <<< "$($D bounds)"
at drag $((x + w / 2)) $((y + 12)) $((vx + vw / 2)) $((vy + vh / 2)) && { sleep 0.8; }
read x y w h <<< "$($D bounds)"
at throw $((x + w / 2)) $((y + h / 2)) 160 0 && { sleep 0.8; check "throw right → right half" $((vx + vw - half)) $half; }
at throw $((vx + vw - half / 2)) $((vy + vh / 2)) -160 0 && { sleep 0.8; check "throw left → left half" $vx $half; }
osascript -e 'tell application "Grids Test Window" to quit' 2>/dev/null; pkill -f TestWindow.app/Contents/MacOS/TestWindow
echo "live: $pass passed, $fail failed"
