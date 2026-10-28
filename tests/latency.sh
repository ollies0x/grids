#!/bin/zsh
# Keypress-to-window-moved latency for whichever window manager is running: tests/latency.sh [rounds]
cd "${0:A:h}/.."
D=build/obj/drive; n=${1:-10}
open -n build/obj/TestWindow.app; sleep 1.5
[[ $($D front) == app.grids.testwindow ]] || { echo "test window not frontmost; not sending keys"; pkill -f TestWindow.app/Contents/MacOS/TestWindow; exit 1 }
times=()
for i in $(seq 1 $n); do
  [[ $($D front) == app.grids.testwindow ]] || { echo "focus moved; stopping"; break }
  k=$(( i % 2 ? 123 : 124 ))
  times+=$($D latency $k 11272192); sleep 0.25
done
pkill -f TestWindow.app/Contents/MacOS/TestWindow
print -l $times | sort -n | awk '{a[NR]=$1} END {printf "median %.1f ms, min %.1f, max %.1f over %d presses\n", a[int((NR+1)/2)], a[1], a[NR], NR}'
