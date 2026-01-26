# 🔧 Command Templates

Orchestration templates that enable Claude Code to coordinate multi-agent workflows for different development tasks.

## Overview

After reading the [main kit documentation](../README.md), you'll understand how these commands fit into the integrated system. Each command:

- **Auto-loads** the appropriate documentation tier for its task
- **Spawns specialized agents** based on complexity 
- **Integrates MCP servers** when external expertise helps
- **Maintains documentation** to keep AI context current

### 🚀 Automatic Context Injection

All commands benefit from automatic context injection via the `subagent-context-injector.sh` hook:

- **Core documentation auto-loaded**: Every command and sub-agent automatically receives `@/docs/CLAUDE.md`, `@/docs/ai-context/project-structure.md`, and `@/docs/ai-context/docs-overview.md`
- **No manual context loading**: Sub-agents spawned by commands automatically have access to essential project documentation
- **Consistent knowledge**: All agents start with the same foundational understanding

## Available Commands

### 📊 `/full-context`
**Purpose**: Comprehensive context gathering and analysis when you need deep understanding or plan to execute code changes.

**When to use**:
- Starting work on a new feature or bug
- Need to understand how systems interconnect
- Planning architectural changes
- Any task requiring thorough analysis before implementation

**How it works**: Adaptively scales from direct analysis to multi-agent orchestration based on request complexity. Agents read documentation, analyze code, map dependencies, and consult MCP servers as needed.

### 🔍 `/code-review` 
**Purpose**: Get multiple expert perspectives on code quality, focusing on high-impact findings rather than nitpicks.

**When to use**:
- After implementing new features
- Before merging important changes
- When you want security, performance, and architecture insights
- Need confidence in code quality

**How it works**: Spawns specialized agents (security, performance, architecture) that analyze in parallel. Each agent focuses on critical issues that matter for production code.

### 🧠 `/gemini-consult` *(Requires Gemini MCP Server)*
**Purpose**: Engage in deep, iterative conversations with Gemini for complex problem-solving and architectural guidance.

**When to use**:
- Tackling complex architectural decisions
- Need expert guidance on implementation approaches
- Debugging intricate issues across multiple files
- Exploring optimization strategies
- When you need a thinking partner for difficult problems

**How it works**: Creates persistent conversation sessions with Gemini, automatically attaching project context and MCP-ASSISTANT-RULES.md. Supports iterative refinement through follow-up questions and implementation feedback.

**Key features**:
- Context-aware problem detection when no arguments provided
- Persistent sessions maintained throughout problem lifecycle
- Automatic attachment of foundational project documentation
- Support for follow-up questions with session continuity

### 📝 `/update-docs`
**Purpose**: Keep documentation synchronized with code changes, ensuring AI context remains current.

**When to use**:
- After modifying code
- After adding new features
- When project structure changes
- Following any significant implementation

**How it works**: Analyzes what changed and updates the appropriate CLAUDE.md files across all tiers. Maintains the context that future AI sessions will rely on.

### 📄 `/create-docs`
**Purpose**: Generate initial documentation structure for existing projects that lack AI-optimized documentation.

**When to use**:
- Adopting the framework in an existing project
- Starting documentation from scratch
- Need to document legacy code
- Setting up the 3-tier structure

**How it works**: Analyzes your project structure and creates appropriate CLAUDE.md files at each tier, establishing the foundation for AI-assisted development.

### ♻️ `/refactor`
**Purpose**: Intelligently restructure code while maintaining functionality and updating all dependencies.

**When to use**:
- Breaking up large files
- Improving code organization
- Extracting reusable components
- Cleaning up technical debt

**How it works**: Analyzes file structure, maps dependencies, identifies logical split points, and handles all import/export updates across the codebase.

### 🤝 `/handoff`
**Purpose**: Preserve context when ending a session or when the conversation becomes too long.

**When to use**:
- Ending a work session
- Context limit approaching
- Switching between major tasks
- Supplementing `/compact` with permanent storage

**How it works**: Updates the handoff documentation with session achievements, current state, and next steps. Ensures smooth continuation in future sessions.

## Integration Patterns

### Typical Workflow
```bash
/full-context "implement user notifications"    # Understand
# ... implement the feature ...
/code-review "review notification system"       # Validate  
/update-docs "document notification feature"    # Synchronize
/handoff "completed notification system"        # Preserve
```

### Quick Analysis
```bash
/full-context "why is the API slow?"           # Investigate
# ... apply fixes ...
/update-docs "document performance fixes"       # Update context
```

### Major Refactoring
```bash
/full-context "analyze authentication module"   # Understand current state
/refactor "@auth/large-auth-file.ts"          # Restructure
/code-review "review refactored auth"          # Verify quality
/update-docs "document new auth structure"     # Keep docs current
```

### Complex Problem Solving
```bash
/gemini-consult "optimize real-time data pipeline" # Start consultation
# ... implement suggested approach ...
/gemini-consult                                    # Follow up with results
/update-docs "document optimization approach"      # Capture insights
```

## Customization

Each command template can be adapted:

- **Adjust agent strategies** - Modify how many agents spawn and their specializations
- **Change context loading** - Customize which documentation tiers load
- **Tune MCP integration** - Adjust when to consult external services
- **Modify output formats** - Tailor results to your preferences

Commands are stored in `.claude/commands/` and can be edited directly.

## Key Principles

1. **Commands work together** - Each command builds on others' outputs
2. **Documentation stays current** - Commands maintain their own context
3. **Complexity scales naturally** - Simple tasks stay simple, complex tasks get sophisticated analysis
4. **Context is continuous** - Information flows between sessions through documentation

---

*For detailed implementation of each command, see the individual command files in this directory.*