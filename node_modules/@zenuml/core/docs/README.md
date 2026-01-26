# Documentation System Guide

This guide explains how the 3-tier documentation architecture powers the Claude Code Development Kit and why it provides superior results compared to traditional documentation approaches.

## Critical Foundation Files

Two files form the cornerstone of the entire documentation system:

1. **docs-overview.md** - The central routing guide that directs AI agents to appropriate documentation based on task complexity. This file maps your entire documentation structure and enables intelligent context loading.

2. **project-structure.md** - The comprehensive overview of your project's complete file structure and technology stack. This file is required reading for all AI agents and must be attached to Gemini consultations.

These foundation files ensure AI agents always have the essential context needed to understand your project and navigate to relevant documentation.

## Why the 3-Tier System

### Traditional Documentation Problems

Standard documentation approaches create friction for AI-assisted development:

- **Context Overload** - AI agents must process entire documentation sets for simple tasks
- **Maintenance Burden** - Every code change cascades to multiple documentation locations
- **Stale Content** - Documentation diverges from implementation reality
- **No AI Optimization** - Human-readable formats lack structure for machine processing

### The 3-Tier Solution

The kit solves these problems through hierarchical organization:

**Tier 1: Foundation (Rarely Changes)**
- Project-wide standards, architecture decisions, technology stack
- Auto-loads for every AI session
- Provides consistent baseline without redundancy
- Uses CLAUDE.md as the master context file

**Tier 2: Component (Occasionally Changes)**
- Component boundaries, architectural patterns, integration points
- Loads only when working within specific components
- Isolates architectural decisions from implementation details
- Uses CONTEXT.md files at component roots

**Tier 3: Feature (Frequently Changes)**
- Implementation specifics, technical details, local patterns
- Co-located with code for immediate updates
- Minimizes documentation cascade when code changes
- Uses CONTEXT.md files within feature directories

## Benefits vs Traditional Systems

### 1. Intelligent Context Loading

**Traditional**: AI loads entire documentation corpus regardless of task
**3-Tier**: Commands load only relevant tiers based on complexity

Example:
- Simple query → Tier 1 only (minimal tokens)
- Component work → Tier 1 + relevant Tier 2
- Deep implementation → All relevant tiers

### 2. Maintenance Efficiency

**Traditional**: Update multiple documents for each change
**3-Tier**: Updates isolated to appropriate tier

Example:
- API endpoint change → Update only Tier 3 API documentation
- New component → Add Tier 2 documentation, Tier 1 unchanged
- Coding standard → Update only Tier 1, applies everywhere

### 3. AI Performance Optimization

**Traditional**: AI struggles to find relevant information
**3-Tier**: Structured hierarchy guides AI to precise context

The system provides:
- Clear routing logic for agent navigation
- Predictable documentation locations
- Efficient token usage through targeted loading

## Integration with Kit Components

### Command Integration

Commands leverage the 3-tier structure for intelligent operation:

```
Command Execution → Analyze Task Complexity → Load Appropriate Tiers
                                            ↓
                                   Simple: Tier 1 only
                                   Component: Tiers 1-2
                                   Complex: All relevant tiers
```

### MCP Server Integration

External AI services receive proper context through the tier system:

- **Gemini Consultations** - Auto-attach `project-structure.md` (Tier 1)
- **Context7 Lookups** - Happen within established project context
- **Recommendations** - Align with documented architecture

### Multi-Agent Routing

The documentation structure determines agent behavior:

- Number of agents spawned based on tiers involved
- Each agent receives targeted documentation subset
- Parallel analysis without context overlap

## Key Files and Their Roles

### Foundation Files (ai-context/)

**docs-overview.md**
- Template for implementing 3-tier documentation
- Maps documentation structure for AI navigation
- [View Template](ai-context/docs-overview.md)

**project-structure.md**
- Complete technology stack and file organization
- Required reading for all AI agents
- Auto-attaches to Gemini consultations
- [View Template](ai-context/project-structure.md)

**system-integration.md**
- Cross-component communication patterns
- Integration architectures for multi-agent analysis
- [View Template](ai-context/system-integration.md)

**deployment-infrastructure.md**
- Infrastructure patterns and constraints
- Deployment context for AI recommendations
- [View Template](ai-context/deployment-infrastructure.md)

**handoff.md**
- Session continuity between AI interactions
- Task state preservation
- [View Template](ai-context/handoff.md)

### Context Templates

**CLAUDE.md** (Tier 1)
- Master AI context with coding standards
- Project-wide instructions and patterns
- [View Template](CLAUDE.md)

**CONTEXT-tier2-component.md**
- Component-level architectural context
- [View Template](CONTEXT-tier2-component.md)

**CONTEXT-tier3-feature.md**
- Feature-specific implementation details
- [View Template](CONTEXT-tier3-feature.md)

## Implementation Strategy

### 1. Start with Templates

Use provided templates as foundation:
- Copy and customize for your project
- Maintain consistent structure
- Focus on AI-consumable formatting

### 2. Follow Natural Boundaries

Let your architecture guide tier placement:
- Stable decisions → Tier 1
- Component design → Tier 2
- Implementation details → Tier 3

### 3. Co-locate Documentation

Place CONTEXT.md files with related code:
```
backend/
├── CONTEXT.md         # Backend architecture (Tier 2)
└── src/
    └── api/
        └── CONTEXT.md # API implementation (Tier 3)
```

### 4. Maintain Hierarchy

Ensure clear relationships:
- Tier 3 references Tier 2 patterns
- Tier 2 follows Tier 1 standards
- No circular dependencies

### 5. Use Documentation Commands

The kit provides commands to manage documentation:
- **`/create-docs`** - Generate initial documentation structure for projects without existing docs
- **`/update-docs`** - Regenerate and update documentation after code changes to keep everything current

## Measuring Success

The 3-tier system succeeds when:

1. **AI agents find context quickly** - No searching through irrelevant documentation
2. **Updates stay localized** - Changes don't cascade unnecessarily
3. **Documentation stays current** - Co-location ensures updates happen
4. **Commands work efficiently** - Appropriate context loads automatically
5. **MCP servers provide relevant advice** - External AI understands your project

---

*Part of the Claude Code Development Kit - see [main documentation](../README.md) for complete system overview.*