# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX
"""quire-analyze's documentation figures, rendered by the docs-figures skill.

Regenerate from the repository root with the agent-ix/dev-tools skill:

    python3 -m pip install fonttools
    python3 <dev-tools>/skills/docs-figures/scripts/render.py docs/images/figures.py

Add --check to confirm the committed SVGs are current.
"""

from docs_figures import FONTS, SMALL, TITLE, Figure  # noqa: F401

STYLE = "ix-docs"

# One role per kind of thing, fixed across the Quire repositories.
ROLES = [
    ("code", "authored source", "box"),
    ("logic", "compiler stage", "box"),
    ("model", "checked package / IR", "box"),
    ("evidence", "outcome", "box"),
    ("ink", "input", "box"),
]




# ---- figures -----------------------------------------------------------------


def hero():
    """README hero: contracts become one SMT query, and the answer keeps its honesty."""
    f = Figure(
        "hero",
        980,
        244,
        "Quire Analyze turns Contract IR clauses into an analysis request, lowers it to an SMT-LIB "
        "query over Booleans, runs a pinned solver (Z3 or cvc5) and classifies the reply. The "
        "answer is satisfied, refuted with a verified witness model, unknown, timeout, unsupported "
        "or tool-error.",
    )
    xs = [0, 207, 415, 622, 830]
    w = 150
    f.box(xs[0], 0, w, 56, "model", "contract clauses", "from Contract IR")
    f.box(xs[1], 0, w, 56, "logic", "request", "a role per clause")
    f.box(xs[2], 0, w, 56, "model", "SMT-LIB query", "QF_UF · Booleans")
    f.box(xs[3], 0, w, 56, "ink", "pinned solver", "Z3 or cvc5")
    f.box(xs[4], 0, w, 56, "logic", "classify", "replay the model")
    for i in range(4):
        role = "ink" if i == 2 else "logic" if i == 0 else "model"
        f.line([(xs[i] + w, 28), (xs[i + 1] - 3, 28)], role)
    f.note(xs[1] + 90, 84, "non-Boolean constructs are refused when lowering", "muted", anchor="start")
    f.line([(xs[4] + w // 2, 56), (xs[4] + w // 2, 100)], "evidence", arrow=False)
    cols = [
        (0, "satisfied", "consistent · implied", "logic"),
        (251, "refuted + witness", "a model that shows why", "evidence"),
        (502, "unknown · timeout", "no answer, said so", "evidence"),
        (753, "unsupported · tool-error", "no answer, said so", "evidence"),
    ]
    centres = [x + 110 for x, *_ in cols]
    f.line([(centres[0], 100), (xs[4] + w // 2, 100)], "evidence", arrow=False)
    for (x, title, sub, role), cx in zip(cols, centres):
        f.line([(cx, 100), (cx, 127)], role)
        f.box(x, 130, 220, 56, role, title, sub)
    f.legend(0, 226, [("model", "checked IR / query", "box"), ("logic", "processing step", "box"),
                      ("ink", "external solver", "box"), ("evidence", "outcome", "box")])
    return f


def kinds():
    """The five questions, as what is asserted and what each solver reply means."""
    rows = [
        ("consistency", "assumptions + selected", "sat: satisfied", "unsat: refuted"),
        ("contradiction", "assumptions + left + right", "sat: refuted", "unsat: satisfied"),
        ("implication", "antecedents + NOT consequent", "sat: refuted", "unsat: satisfied"),
        ("redundancy", "peers + NOT candidate", "sat: refuted", "unsat: satisfied"),
        ("dead antecedent", "assumptions + antecedent", "sat: refuted", "unsat: satisfied"),
    ]
    f = Figure(
        "kinds",
        980,
        330,
        "Each of the five analysis kinds asserts a fixed set of statements and reads the solver's "
        "reply in a fixed way. Consistency is satisfied when the solver answers sat. Contradiction, "
        "implication, redundancy and dead antecedent assert a negation, so unsat means satisfied "
        "and sat is a counterexample.",
    )
    f.text(0, 14, "QUESTION", "muted", "mono", SMALL)
    f.text(250, 14, "WHAT THE QUERY ASSERTS", "muted", "mono", SMALL)
    f.text(600, 14, "SOLVER SAYS SAT", "muted", "mono", SMALL)
    f.text(790, 14, "SOLVER SAYS UNSAT", "muted", "mono", SMALL)
    for i, (name, asserts, sat, unsat) in enumerate(rows):
        y = 28 + i * 58
        f.box(0, y, 220, 46, "logic", name)
        f.box(250, y, 320, 46, "model", asserts)
        sat_role = "logic" if "satisfied" in sat else "evidence"
        unsat_role = "logic" if "satisfied" in unsat else "evidence"
        f.box(600, y, 160, 46, sat_role, sat.split(": ")[1])
        f.box(790, y, 190, 46, unsat_role, unsat.split(": ")[1])
        f.line([(220, y + 23), (247, y + 23)], "logic")
        f.line([(570, y + 23), (597, y + 23)], "model", arrow=False)
    return f


def differential():
    """Two solvers on one query, and the four ways they can relate."""
    f = Figure(
        "differential",
        980,
        244,
        "One SMT-LIB query is sent to both Z3 and cvc5 and each reply is classified. The pair is an "
        "agreement only when both are conclusive, equal and any witness model is verified; "
        "otherwise it is a disagreement, unavailable or inconclusive. The CLI exits 0 or 1 on an "
        "agreement and 3 on anything else.",
    )
    f.box(0, 104, 150, 56, "model", "SMT-LIB query", "one digest")
    f.box(200, 40, 130, 56, "ink", "Z3", "pinned build")
    f.box(200, 168, 130, 56, "ink", "cvc5", "pinned build")
    f.box(380, 40, 130, 56, "logic", "classify", "status + model")
    f.box(380, 168, 130, 56, "logic", "classify", "status + model")
    f.box(550, 104, 120, 56, "logic", "compare", "both records")
    f.line([(150, 132), (175, 132), (175, 68), (197, 68)], "model")
    f.line([(150, 132), (175, 132), (175, 196), (197, 196)], "model")
    f.line([(330, 68), (377, 68)], "ink")
    f.line([(330, 196), (377, 196)], "ink")
    f.line([(510, 68), (530, 68), (530, 132), (547, 132)], "logic")
    f.line([(510, 196), (530, 196), (530, 132), (547, 132)], "logic")
    outs = [
        ("agreement", "equal, conclusive · exit 0 or 1", "logic"),
        ("disagreement", "opposite answers · exit 3", "evidence"),
        ("unavailable", "solver missing or unpinned · exit 3", "evidence"),
        ("inconclusive", "unknown, timeout, error · exit 3", "evidence"),
    ]
    f.line([(670, 132), (685, 132)], "logic", arrow=False)
    f.line([(685, 26), (685, 238)], "logic", arrow=False)
    for i, (name, sub, role) in enumerate(outs):
        y = 4 + i * 70
        f.line([(685, y + 22), (707, y + 22)], role)
        f.box(710, y, 270, 44, role, name, sub)
    return f


REPOS = [
    ("ORCHESTRATION", [("quire-verification", True), ("quire-driver", True), ("quire-integration", True)]),
    ("BACKENDS", [("quire-contract-codegen", False), ("quire-contract-runtime", False),
                  ("quire-analyze", False), ("quire-protocol", True), ("quire-mltl", False)]),
    ("LANGUAGE", [("quire-specification", True), ("quire-spec-language", False), ("quire-contract-ir", False)]),
    ("FOUNDATIONS", [("quire-exact", False), ("quire-semantic-value", False), ("quire-walk", False),
                     ("quire-canonical", False), ("quire-verification-contracts", False),
                     ("quire-observation", False)]),
]


def ecosystem(here="quire-analyze"):
    """The Quire repositories in bands; each band builds on the ones below."""
    f = Figure(
        "ecosystem",
        980,
        364,
        "The Quire repositories in four bands. Foundations: quire-exact, quire-semantic-value, "
        "quire-walk, quire-canonical, quire-verification-contracts, quire-observation. Language: "
        "quire-specification, quire-spec-language, quire-contract-ir. Backends: "
        "quire-contract-codegen, quire-contract-runtime, quire-analyze, quire-protocol, "
        "quire-mltl. Orchestration: quire-verification, quire-driver, quire-integration.",
    )
    y = 0
    gap = 10
    for band, repos in REPOS:
        f.text(0, y + 12, band, "muted", "mono", SMALL)
        widths = [FONTS["title"].width(name, TITLE) + 24 for name, _ in repos]
        spare = (980 - sum(widths) - gap * (len(repos) - 1)) / len(repos)
        x = 0
        for (name, private), w in zip(repos, widths):
            w = round(w + spare)
            role = "logic" if name == here else "ink"
            f.box(x, y + 20, w, 50, role, name, "private" if private else "public", dashed=private)
            x += w + gap
        y += 88
    f.legend(0, 360, [("logic", "this repository", "box"), ("ink", "private", "dash")])
    return f


FIGURES = [hero, kinds, differential, ecosystem]
