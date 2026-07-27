#!/usr/bin/env sh
# SPDX-License-Identifier: MPL-2.0
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# check-idris2-proofs.sh — the proof gate for typell's Idris 2 ABI modules.
# Pattern imported from kategoria (scripts/check-idris2-proofs.sh there).
#
# HARD-FAIL DESIGN: exits non-zero when idris2 is missing. A proof gate
# that silently passes without its prover is not a gate.
#
# Modules are checked under a TYPELL/ABI/ directory layout matching their
# `module TYPELL.ABI.*` names (built in a temp dir; the repo keeps them
# at src/abi/).
#
# QUARANTINE: Layout.idr is template scaffolding with three named holes,
# Refl-unprovable divisibility claims, and verifiers that verify nothing;
# no module imports it. It is EXCLUDED from the pass-required set and
# reported loudly below. If it ever starts passing, this script says so —
# then move it into MODULES.

set -u

if ! command -v idris2 >/dev/null 2>&1; then
    echo "FATAL: idris2 not found on PATH - refusing to skip proof checking." >&2
    echo "Install Idris 2 (0.7.0/0.8.0 verified per PROOF-STATUS.adoc) and re-run." >&2
    exit 1
fi

echo "prover: $(idris2 --version)"

MODULES="Types Foreign Soundness InferenceSoundness LevelMonotonicity"
QUARANTINED="Layout"

WORK="${TMPDIR:-/tmp}/typell-proof-check.$$"
mkdir -p "$WORK/TYPELL/ABI"
cp src/abi/*.idr "$WORK/TYPELL/ABI/"
trap 'rm -rf "$WORK"' EXIT

pass=0
fail=0

for m in $MODULES; do
    if (cd "$WORK" && idris2 --check "TYPELL/ABI/$m.idr" >/dev/null 2>&1); then
        pass=$((pass+1))
        printf 'PASS %s\n' "$m"
    else
        fail=$((fail+1))
        printf 'FAIL %s\n' "$m"
        (cd "$WORK" && idris2 --check "TYPELL/ABI/$m.idr" 2>&1 | head -12)
    fi
done

for m in $QUARANTINED; do
    if (cd "$WORK" && idris2 --check "TYPELL/ABI/$m.idr" >/dev/null 2>&1); then
        printf 'QUARANTINED %s: NOW PASSES — promote it into MODULES.\n' "$m"
    else
        printf 'QUARANTINED %s: still fails (expected; see PROOF-STATUS.adoc).\n' "$m"
    fi
done

# Axiom-smuggling scan on the pass-required modules. Strip `--` line
# comments and `|||` doc lines so prose ABOUT the constructs is fine.
smuggled=$(
    for m in $MODULES; do
        f="src/abi/$m.idr"
        [ -f "$f" ] || continue
        sed -e 's/--.*$//' -e '/^[[:space:]]*|||/d' "$f" \
            | grep -nE '(^|[^A-Za-z_])(postulate|believe_me|assert_total)([^A-Za-z_]|$)|%hint|\?[a-zA-Z_][a-zA-Z0-9_]*' \
            | sed "s|^|$f:|"
    done
)
if [ -n "$smuggled" ]; then
    echo "FATAL: axiom smuggling or holes detected outside comments:" >&2
    echo "$smuggled" >&2
    exit 1
fi

echo "proof-check: PASS=$pass FAIL=$fail (quarantined: $QUARANTINED)"
if [ "$fail" -ne 0 ] || [ "$pass" -eq 0 ]; then
    exit 1
fi
