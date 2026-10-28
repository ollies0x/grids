#!/bin/zsh
# Screenshot a window by owner name without focusing it: scripts/winshot.sh "Grids Settings" out.png
owner=$1; out=$2
id=$(osascript -l JavaScript -e "ObjC.import('CoreGraphics'); var a=ObjC.deepUnwrap(ObjC.castRefToObject(\$.CGWindowListCopyWindowInfo(1,0))); var w=a.filter(w=>w.kCGWindowOwnerName=='$owner' && w.kCGWindowIsOnscreen)[0]; w? w.kCGWindowNumber : ''")
[[ -z $id ]] && { echo "no window for $owner"; exit 1 }
screencapture -x -o -l $id $out && echo $out
