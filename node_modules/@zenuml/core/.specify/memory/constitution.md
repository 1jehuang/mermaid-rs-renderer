<!-- Sync Impact Report
Version change: 1.0.0 → 1.1.0 (New principle added)
Modified principles: None
Added sections: II. Task Focus principle
Removed sections: None
Templates requiring updates: ✅ plan-template.md
Follow-up TODOs: RATIFICATION_DATE needs confirmation
-->

# ZenUML Core Constitution

## Core Principles

### I. Parser Simplicity

The ANTLR lexer and parser MUST remain simple and maintainable. Edge cases and
complex scenarios SHALL be handled in utility methods or the renderer, not in the
grammar. Reasonable fallbacks are preferred over grammar complexity. This ensures
the parser remains understandable, testable, and maintainable while avoiding
fragility from over-engineering.

### II. Task Focus

When working on one task, focus on that task. Do not try to fix unrelated issues
(code smell, inconsistency, or even defects). You SHALL mention unrelated issues
but MUST NOT fix them. This ensures changes remain scoped, reviewable, and
traceable to specific requirements.

## Governance

The Constitution supersedes all other development practices. Amendments require documentation and versioning.

**Version**: 1.1.0 | **Ratified**: TODO(RATIFICATION_DATE): Needs confirmation | **Last Amended**: 2025-10-06