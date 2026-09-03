# Writing policy

Applies to every document under `docs/` and the repository READMEs. Adopted after the WP0 freeze; editorial changes made under this policy do not alter requirements.

## Normative text

1. One rule per normative sentence. A sentence contains one MUST, SHOULD, or MAY obligation.
2. Rationale follows in a separate, non-normative paragraph or a parenthetical *Rationale:* sentence. It never shares a sentence with the rule.
3. Exceptions, evidence requirements, and failure behaviour are separate sentences.
4. No sentence over roughly 45 words unless it is a table definition or a list item that enumerates.
5. RFC 2119 keywords are capitalized in specifications and ADRs. Papers and the plan use lower-case English.
6. Identifiers (`field_names`, requirement IDs, test IDs, event names) are never respelled or reworded.

## Ownership and repetition

7. Every decision has one owning document. Other documents state it in one sentence and link.
8. Repetition is permitted only where it prevents unsafe implementation: fail-closed behaviour, authoritative identifier selection, no identity reuse before reclamation, per-operation gateway authentication, evaluation-arm attribution policy, no pooling of incomparable control-arm results.
9. Review history belongs in the revision history, not in active prose. Phrases such as "review showed", "a second review observed", or "this revision decides" are removed; the final rationale stays.
10. Frozen decisions are written in the present tense. Completed work is written in the past tense.

## Register

11. Direct claims. Not "this paper argues that X" but "X".
12. No evaluative filler: *honest, meaningful, coherent, central, practical, deliberately, explicitly, rigorously, silently, naturally, clearly* — unless the word carries a testable criterion.
13. No formulaic self-description: "the strongest objection is", "the central contribution is", "the nearest payoff is".
14. Antithesis ("not X but Y", "not merely X") only where it defines a boundary, not for emphasis.

## Spelling

15. Oxford spelling: *-ize* and *-ization* (authorization, realization, organization), *-our* (behaviour), *-ue* (catalogue), *artefact*, *labelled*, *centre*. Field names and code identifiers keep their frozen spelling regardless.

## Checks

Before commit: fenced-block parity, relative links, requirement-ID uniqueness and resolution, test-ID resolution, JSON examples parse, README version table matches headers, revision histories ascending. An editorial commit additionally diffs the set of RFC 2119 sentences before and after and explains any difference.
