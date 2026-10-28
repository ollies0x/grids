#!/usr/bin/env python3
"""Rectangle Pro action parity audit.

The list below is every raw value of Rectangle Pro 3.95's WindowAction enum, read
from the action string table in the shipped binary (both architectures agree).
Run after changing the catalog or aliases:

    python3 scripts/parity.py

Exit 1 if a Pro action is neither in the Grids catalog nor in INTERNAL.
"""
import re
import sys
from pathlib import Path

# Rectangle Pro 3.95 WindowAction raw values, enum order.
PRO_ACTIONS = [
    "leftHalf", "rightHalf", "maximize", "maximizeHeight",
    "previousDisplay", "nextDisplay", "larger", "smaller",
    "bottomHalf", "topHalf", "center", "bottomLeft",
    "bottomRight", "topLeft", "topRight", "restore",
    "firstThird", "firstTwoThirds", "centerThird", "lastTwoThirds",
    "lastThird", "moveLeft", "moveRight", "moveUp",
    "moveDown", "almostMaximize", "fillLeft", "fillRight",
    "drag", "resize", "dial", "reticle",
    "manual", "centerHalf", "firstFourth", "secondFourth",
    "thirdFourth", "lastFourth", "topLeftSixth", "topCenterSixth",
    "topRightSixth", "bottomLeftSixth", "bottomCenterSixth", "bottomRightSixth",
    "close", "minimize", "quitApp", "hideApp",
    "winMod", "winUp", "winDown", "winLeft",
    "winRight", "restoreToScreen", "cascadeAll", "cascadeApp",
    "tile2x2", "tile2x3", "revealDesktopEdge", "appNextDisplay",
    "appPrevDisplay", "appCollateLeft", "appCollateRight", "appRightHalf",
    "appLeftHalf", "firstSixth", "lastSixth", "specified",
    "firstThreeFourths", "lastThreeFourths", "fullscreen", "reverseAll",
    "topLeftNinth", "topCenterNinth", "topRightNinth", "middleLeftNinth",
    "middleCenterNinth", "middleRightNinth", "bottomLeftNinth", "bottomCenterNinth",
    "bottomRightNinth", "topLeftThird", "topRightThird", "bottomLeftThird",
    "bottomRightThird", "topLeftEighth", "topCenterLeftEighth", "topCenterRightEighth",
    "topRightEighth", "bottomLeftEighth", "bottomCenterLeftEighth", "bottomCenterRightEighth",
    "bottomRightEighth", "stashLeft", "stashRight", "stashUp",
    "stashDown", "unstash", "cycleStashed", "toggleStashed",
    "unstashAll", "reflowPin", "hideAll", "nextDisplayRatio",
    "prevDisplayRatio", "appPlaceholder", "last", "nudgeLeft",
    "nudgeRight", "nudgeUp", "nudgeDown", "snapTopLeft",
    "snapTopRight", "snapBottomLeft", "snapBottomRight", "centerTwoThirds",
    "upperCenter", "tidy", "leftTodo", "rightTodo",
    "stashAllButFront", "stashAll", "fillTopLeft", "fillTopRight",
    "fillBottomLeft", "fillBottomRight", "alternateLeftHalf", "none",
    "nextSpace", "prevSpace", "doubleHeightUp", "doubleHeightDown",
    "doubleWidthLeft", "doubleWidthRight", "halveHeightUp", "halveHeightDown",
    "halveWidthLeft", "halveWidthRight", "centerProminently", "largerWidth",
    "smallerWidth", "centerOnCursorDisplay", "largerHeight", "smallerHeight",
    "maximizeWidth", "firstFifth", "secondFifth", "thirdFifth",
    "fourthFifth", "lastFifth", "centerThreeFourths", "topVerticalThird",
    "middleVerticalThird", "bottomVerticalThird", "topVerticalTwoThirds", "bottomVerticalTwoThirds",
    "tile1x2", "tile2x1", "tile1x3", "tile1x4",
    "leftAndQuarters", "rightAndQuarters", "reasonableSize", "unstashLeft",
    "unstashRight", "unstashUp", "unstashDown", "cycleStashLeft",
    "cycleStashRight", "cycleStashUp", "cycleStashDown", "toggleStashLeft",
    "toggleStashRight", "toggleStashUp", "toggleStashDown", "cascadeActiveApp",
    "tileAll", "tileActiveApp", "unminimizeAllWindows", "focusWindowUnderCursor",
    "prevDisplayMax", "nextDisplayMax", "openSettings", "leftThird",
    "centerVerticalThird", "centerVerticalTwoThirds", "rightThird", "leftTwoThirds",
    "rightTwoThirds", "topThird", "centerHorizontalThird", "centerHorizontalTwoThirds",
    "bottomThird", "topTwoThirds", "bottomTwoThirds", "leftFourth",
    "centerLeftFourth", "centerRightFourth", "rightFourth", "topFourth",
    "centerTopFourth", "centerBottomFourth", "bottomFourth", "rightThreeFourths",
    "bottomThreeFourths", "leftThreeFourths", "topThreeFourths", "centerVerticalThreeFourths",
    "centerHorizontalThreeFourths", "centerVerticalHalf", "centerHorizontalHalf", "topLeftSixthLandscape",
    "topCenterSixthLandscape", "topRightSixthLandscape", "bottomLeftSixthLandscape", "bottomCenterSixthLandscape",
    "bottomRightSixthLandscape", "topLeftSixthPortrait", "topRightSixthPortrait", "leftCenterSixthPortrait",
    "rightCenterSixthPortrait", "bottomLeftSixthPortrait", "bottomRightSixthPortrait", "topLeftTwoSixthsLandscape",
    "topLeftTwoSixthsPortrait", "topRightTwoSixthsLandscape", "topRightTwoSixthsPortrait", "bottomLeftTwoSixthsLandscape",
    "bottomLeftTwoSixthsPortrait", "bottomRightTwoSixthsLandscape", "bottomRightTwoSixthsPortrait", "firstFifthLandscape",
    "secondFifthLandscape", "thirdFifthLandscape", "fourthFifthLandscape", "lastFifthLandscape",
    "firstFifthPortrait", "secondFifthPortrait", "thirdFifthPortrait", "fourthFifthPortrait",
    "lastFifthPortrait", "firstSixthLandscape", "secondSixthLandscape", "thirdSixthLandscape",
    "fourthSixthLandscape", "fifthSixthLandscape", "lastSixthLandscape", "firstSixthPortrait",
    "secondSixthPortrait", "thirdSixthPortrait", "fourthSixthPortrait", "fifthSixthPortrait",
    "lastSixthPortrait", "unrevealDesktopEdge", "maximizeNoStage",
]

# Actions the Pro UI does not offer as shortcuts: they enter a cursor/drag mode,
# are sentinels, or are placeholders used while the shortcut list is built.
INTERNAL = {
    "drag", "resize", "dial", "reticle", "manual", "winMod", "appPlaceholder", "none",
}

src = (Path(__file__).resolve().parent.parent / "src" / "actions.rs").read_text()
catalog = set(re.findall(r'a\("([a-z0-9-]+)"', src))

aliases = {}
for m in re.finditer(r'"([a-z0-9-]+)"(?:\s*\|\s*"([a-z0-9-]+)")?\s*=>\s*"([a-z0-9-]*)"', src):
    first, second, target = m.groups()
    aliases[first] = target
    if second:
        aliases[second] = target


def resolve(raw):
    kebab = re.sub(r"([A-Z])", lambda m: "-" + m.group(1).lower(), raw)
    for suffix in ("-landscape", "-portrait"):
        if kebab.endswith(suffix):
            kebab = kebab[: -len(suffix)]
    return aliases.get(aliases.get(kebab, kebab), aliases.get(kebab, kebab))


missing = []
for raw in PRO_ACTIONS:
    if raw in INTERNAL:
        continue
    if resolve(raw) not in catalog:
        missing.append((raw, resolve(raw)))

print(f"Rectangle Pro actions: {len(PRO_ACTIONS)}")
print(f"Grids catalog ids:     {len(catalog)}")
print(f"Covered:               {len(PRO_ACTIONS) - len(INTERNAL) - len(missing)}")
print(f"Internal/non-action:   {len(INTERNAL)}")
if missing:
    print("\nMISSING:")
    for raw, ours in missing:
        print(f"  {raw} -> {ours}")
    sys.exit(1)
print("\nEvery Pro action is implemented or internal.")
