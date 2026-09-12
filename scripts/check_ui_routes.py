#!/usr/bin/env python3
"""UI route and token smoke checks. Offline. No pip deps.

Fails if V01–V17 routes (including `/advisor`, the F-13/F-14 entry) are
missing from packages/ui/src/routes.ts or if C03 token contracts are absent.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROUTES = ROOT / "packages/ui/src/routes.ts"
APP = ROOT / "packages/ui/src/App.tsx"
TOKENS = ROOT / "packages/ui-tokens/tokens.js"

REQUIRED_PATHS = [
    "/checkup",
    "/inspector",
    "/compare",
    "/receipts",
    "/receipts/:id",
    "/assets",
    "/assets/:id",
    "/sessions",
    "/sessions/:id",
    "/monitor",
    "/lab",
    "/lab/:id",
    "/sync",
    "/doctor",
    "/policy",
    "/standards",
    "/standards/:id",
    "/settings",
    "/exceptions",
    "/team/compliance",
    "/care-plan/:findingId",
    "/integrations",
    "/integrations/:id",
    "/advisor",
]


def main() -> int:
    errors: list[str] = []
    for path in (ROUTES, APP, TOKENS):
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")
    if errors:
        print("\n".join(errors))
        return 1
    routes = ROUTES.read_text(encoding="utf-8")
    app = APP.read_text(encoding="utf-8")
    tokens = TOKENS.read_text(encoding="utf-8")
    for item in REQUIRED_PATHS:
        if item not in routes:
            errors.append(f"routes.ts missing {item}")
        needle = item.split(":")[0]
        if needle not in app:
            errors.append(f"App.tsx missing {needle}")
    if "expectedIsNotTruthState = true" not in tokens:
        errors.append("tokens.js must state Expected is not truth_state")
    if "unknownIsNotSeverity = true" not in tokens:
        errors.append("tokens.js must state Unknown is not severity")
    if errors:
        print("\n".join(errors))
        return 1
    print("ui-routes-ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
