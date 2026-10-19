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
    (1, 0, "quire-driver", "the quire command", True),
    (2, 0, "quire-integration", "tests the whole chain", True),
    (3, 0, "quire-verification", "plans verification", True),
    (0, 1, "quire-specification", "the standard", True),
    (1, 1, "quire-spec-language", "compiler · evaluator", False),
    (2, 1, "quire-contract-ir", "contract model", False),
    (3, 1, "quire-contract-codegen", "code · tests · proofs", False),
    (4, 1, "quire-contract-runtime", "runs generated checks", False),
    (0, 2, "quire-observation", "qualified observations", False),
    (1, 2, "quire-protocol", "protocol conformance", True),
    (2, 2, "quire-analyze", "SMT consistency", False),
    (0, 3, "quire-mltl", "MLTL over observations", False),
]
FEEDS = [
    ("quire-specification", "quire-spec-language"),
    ("quire-spec-language", "quire-contract-ir"),
    ("quire-contract-ir", "quire-contract-codegen"),
    ("quire-contract-codegen", "quire-contract-runtime"),
    ("quire-spec-language", "quire-protocol"),
    ("quire-contract-ir", "quire-analyze"),
    ("quire-observation", "quire-protocol"),
    ("quire-observation", "quire-mltl"),
]
SHARED = ["quire-exact", "quire-semantic-value", "quire-walk", "quire-canonical",
          "quire-verification-contracts"]


def ecosystem(here="quire-analyze"):
    """The Quire repositories: the main chain, its branches, and what runs and shares it."""
    f = Figure(
        "ecosystem",
        980,
        476,
        "The Quire repositories. The main chain runs from the standard, quire-specification, to "
        "the compiler, quire-spec-language, then quire-contract-ir, quire-contract-codegen and "
        "quire-contract-runtime. quire-spec-language also feeds quire-protocol, and "
        "quire-contract-ir feeds quire-analyze. quire-observation feeds quire-protocol and "
        "quire-mltl. Above the chain, quire-driver runs it, quire-integration tests it and "
        "quire-verification plans verification. Below it, shared libraries used throughout: "
        "quire-exact, quire-semantic-value, quire-walk, quire-canonical and "
        "quire-verification-contracts.",
    )
    w, h, step = 176, 50, 201
    rows = {0: 20, 1: 120, 2: 210, 3: 290}
    where = {}
    for col, row, name, sub, private in REPOS:
        x, y = col * step, rows[row]
        where[name] = (x, y)
        role = "logic" if name == here else "ink"
        f.box(x, y, w, h, role, name, sub, dashed=private)
    for a, b in FEEDS:
        (ax, ay), (bx, by) = where[a], where[b]
        if ay == by:
            f.line([(ax + w, ay + h / 2), (bx - 3, by + h / 2)], "model")
        else:
            f.line([(ax + w / 2, ay + h), (bx + w / 2, by - 3)], "model")
    f.text(0, 12, "ORCHESTRATION", "muted", "mono", SMALL)
    f.text(0, 112, "THE CHAIN", "muted", "mono", SMALL)
    f.text(0, 382, "SHARED LIBRARIES · USED THROUGHOUT", "muted", "mono", SMALL)
    gap = 10
    widths = [FONTS["title"].width(name, TITLE) + 24 for name in SHARED]
    spare = (980 - sum(widths) - gap * (len(SHARED) - 1)) / len(SHARED)
    x = 0
    for name, w0 in zip(SHARED, widths):
        w0 = round(w0 + spare)
        f.box(x, 390, w0, h, "logic" if name == here else "ink", name)
        x += w0 + gap
    f.legend(0, 472, [("logic", "this repository", "box"), ("ink", "private", "dash"),
                      ("model", "feeds", "fill")])
    return f


FIGURES = [hero, kinds, differential, ecosystem]
