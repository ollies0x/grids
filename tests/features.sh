#!/bin/zsh
# Live checks for the Pro features against the running Grids, using only the self-test window.
# Temporarily edits the config (restored on exit).
cd "${0:A:h}/.."
D=build/obj/drive
CFG="$HOME/Library/Application Support/Grids/config.json"
cp "$CFG" /tmp/grids-config-backup.json
cleanup() { cp /tmp/grids-config-backup.json "$CFG"; open -g "grids://reload"; pkill -f TestWindow.app/Contents/MacOS/TestWindow }
trap cleanup EXIT
cfg() { python3 -c "
import json,sys
c=json.load(open(sys.argv[1]))
exec(sys.argv[2])
json.dump(c,open(sys.argv[1],'w'),indent=2)" "$CFG" "$1"; open -g "grids://reload"; sleep 0.6; }

open -n build/obj/TestWindow.app; sleep 1.5
W=$($D wid)
read vx vy vw vh <<< "$($D screen)"
pass=0; fail=0

# Cursor-driven steps act on whatever window is under the cursor; only run them when the
# test window is the topmost window at every point the step touches.
at() { # at <drive cmd> X Y ...: run a cursor step only if the test window is on top at X,Y
  if mine "$2,$3"; then $D "$@"; else echo "SKIP (test window not on top at $2,$3): $1"; return 1; fi
}
mine() { for pt in "$@"; do [[ $($D topat ${pt%,*} ${pt#*,}) == $W ]] || return 1; done }
ok() { pass=$((pass+1)); echo "ok   $1"; }
bad() { fail=$((fail+1)); echo "FAIL $1"; }
near() { (( $1 >= $2 - 4 && $1 <= $2 + 4 )) }
act() { open -g "grids://execute-action?name=$1&window=$W"; sleep 0.5; }
bounds() { read x y w h <<< "$($D bounds)"; }

act center; bounds; home=($x $y $w $h)

# Stash: tuck left, reveal at the edge, hide on leaving, unstash home.
act stash-left; bounds
(( x + w <= vx + 20 )) && ok "stash-left tucks the window ($x)" || bad "stash-left left x=$x w=$w"
$D move $((vx + 1)) $((y + h / 2)); sleep 0.6; bounds
near $x $vx && ok "cursor at the edge reveals it" || bad "reveal: x=$x"
$D move $((vx + vw / 2 + 400)) $((vy + vh / 2)); sleep 0.8; bounds
(( x + w <= vx + 20 )) && ok "leaving hides it again" || bad "re-hide: x=$x"
act unstash; bounds
near $x ${home[1]} && ok "unstash returns home" || bad "unstash: x=$x want ${home[1]}"

# Layout with a repeat step.
cfg "c['layouts']=[{'name':'T','entries':[{'app':'app.grids.testwindow','appName':'T','steps':[{'command':'bottom-half','display':''},{'command':'top-half','display':''}],'titleMode':'any','title':''}],'others':'none'}]"
open -g "grids://execute-layout?name=T"; sleep 0.7; bounds
near $y $((vy + vh / 2)) && near $w $vw && ok "layout applies bottom half" || bad "layout: $x $y $w $h"
open -g "grids://execute-layout?name=T"; sleep 0.7; bounds
near $y $vy && ok "layout repeat applies its second step" || bad "layout repeat: $x $y $w $h"

# Pin Mode: the test app in a right-hand quarter column.
cfg "c['pin'].update({'app':'app.grids.testwindow','appName':'Test','width':0.25,'side':'right'})"
open -g "grids://execute-action?name=pin"; sleep 0.8; bounds
near $x $((vx + vw - vw / 4)) && near $w $((vw / 4)) && ok "pin puts the app in its column" || bad "pin: $x $y $w $h"
open -g "grids://execute-action?name=pin"; sleep 0.3
act center

# Quick Throw on control: sweep right.
cfg "c['quickThrow']=[{'modifiers':262144,'left':'left-half','right':'right-half','up':'maximize','down':'center'}]"
bounds; at quick $((x + w / 2)) $((y + h / 2)) 90 0 262144 59 && { sleep 0.7; bounds; }
near $x $((vx + vw / 2)) && ok "quick throw right → right half" || bad "quick throw: $x $y $w $h"
act center

# Move & Resize with control+shift held.
cfg "c['quickThrow']=[]; c['moveResize']={'moveModifiers':393216,'resizeModifiers':0}"
bounds; x0=$x; y0=$y
at hold $((x + w / 2)) $((y + h / 2)) 150 80 393216 56 && { sleep 0.5; bounds; }
near $x $((x0 + 150)) && near $y $((y0 + 80)) && ok "move with ⌃⇧ held follows the cursor" || bad "move: $x $y (from $x0 $y0)"
cfg "c['moveResize']={'moveModifiers':0,'resizeModifiers':0}"

# Snap panel at the top of the screen: drop on its third tile (right half).
cfg "c['snap'].update({'panelEnabled':True,'panelPosition':'top','panelItems':['left-half','maximize','right-half']})"
act center; bounds
pw=208; px=$((vx + vw / 2 - pw / 2)); ty=$((vy + 12 + 10 + 28))
at dragto $((x + w / 2)) $((y + 12)) $((x + w / 2)) $((y + 300)) $((px + 10 + 2 * 66 + 28)) $ty && { sleep 0.8; bounds; }
near $x $((vx + vw / 2)) && near $w $((vw / 2)) && ok "snap panel tile → right half" || bad "snap panel: $x $y $w $h"
cfg "c['snap']['panelEnabled']=False"

# Snap target from a custom entry.
cfg "c['custom']=[{'name':'Target','steps':[{'command':None,'place':{'position':'custom','x':0.25,'y':0.25,'w':0.5,'h':0.5},'display':''}],'snapTarget':True}]"
act center; bounds
at dragto $((x + w / 2)) $((y + 12)) $((x + w / 2)) $((y + 200)) $((vx + vw / 2)) $((vy + vh / 2)) && { sleep 0.8; bounds; }
near $x $((vx + vw / 4)) && near $w $((vw / 2)) && ok "drop on a snap target" || bad "snap target: $x $y $w $h"

# Double-click the title bar: maximize, then restore.
cfg "c['general']['doubleClickTitleBar']=True"
act center; bounds; cx=$x
at dclick $((x + w / 2)) $((y + 12)) && { sleep 0.8; bounds; }
near $w $vw && ok "double-click title bar maximizes" || bad "dclick: $x $y $w $h"
at dclick $((vx + vw / 2)) $((vy + 12)) && { sleep 0.8; bounds; }
(( w < vw - 50 )) && ok "double-click again restores" || bad "dclick restore: $x $y $w $h"

# Long Throw: a long sweep right picks the long sector (Last Third).
act center; bounds
at throw $((x + w / 2)) $((y + h / 2)) 420 0 && { sleep 0.8; bounds; }
near $x $((vx + vw - vw / 3)) && near $w $((vw / 3)) && ok "long throw right → last third" || bad "long throw: $x $y $w $h"

# Win arrow keys (⌃⇧ + arrows here) and hold-to-repeat, only with the test window in front.
cfg "c['winArrowModifiers']=393216; c['shortcuts']['nudge-right']={'keyCode':124,'modifierFlags':1441792}"
act center
if [[ $($D front) == app.grids.testwindow ]]; then
  $D key 124 10878976; sleep 0.4; bounds
  near $x $((vx + vw / 2)) && near $w $((vw / 2)) && ok "win → right half" || bad "win →: $x $y $w $h"
  $D key 126 10878976; sleep 0.4; bounds
  near $h $((vh / 2)) && near $x $((vx + vw / 2)) && ok "win ↑ → top right" || bad "win ↑: $x $y $w $h"
  act center; bounds; x0=$x
  $D keyhold 124 11927552 1.0; sleep 0.3; bounds
  (( x - x0 >= 5 * 30 )) && ok "holding Nudge Right repeats ($((x - x0)) px)" || bad "hold repeat moved $((x - x0)) px"
else
  echo "SKIP win arrows / hold (test window not frontmost)"
fi

echo "features: $pass passed, $fail failed"
