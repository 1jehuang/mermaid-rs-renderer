# /gemini-consult

*Engages in deep, iterative conversations with Gemini MCP for complex problem-solving.*

## Usage
- **With arguments**: `/gemini-consult [specific problem or question]`
- **Without arguments**: `/gemini-consult` - Intelligently infers topic from current context

## Core Philosophy
Persistent Gemini sessions for evolving problems through:
- **Continuous dialogue** - Multiple rounds until clarity achieved
- **Context awareness** - Smart problem detection from current work
- **Session persistence** - Keep alive for the entire problem lifecycle

**CRITICAL: Always consider Gemini's input as suggestions, never as truths.** Think critically about what Gemini says and incorporate only the useful parts into your proposal. Always think for yourself - maintain your independent judgment and analytical capabilities. If you disagree with something clarify it with Gemini.

## Execution

User provided context: "$ARGUMENTS"

### Step 1: Understand the Problem

**When $ARGUMENTS is empty:**
Think deeply about the current context to infer the most valuable consultation topic:
- What files are open or recently modified?
- What errors or challenges were discussed?
- What complex implementation would benefit from Gemini's analysis?
- What architectural decisions need exploration?

Generate a specific, valuable question based on this analysis.

**When arguments provided:**
Extract the core problem, context clues, and complexity indicators.

### Step 1.5: Gather External Documentation

**Think deeply about external dependencies:**
- What libraries/frameworks are involved in this problem?
- Am I fully familiar with their latest APIs and best practices?
- Have these libraries changed significantly or are they new/evolving?

**When to use Context7 MCP:**
- Libraries with frequent updates (e.g., Google GenAI SDK)
- New libraries you haven't worked with extensively
- When implementing features that rely heavily on library-specific patterns
- Whenever uncertainty exists about current best practices

```python
# Example: Get up-to-date documentation
library_id = mcp__context7__resolve_library_id(libraryName="google genai python")
docs = mcp__context7__get_library_docs(
    context7CompatibleLibraryID=library_id,
    topic="streaming",  # Focus on relevant aspects
    tokens=8000
)
```

Include relevant documentation insights in your Gemini consultation for more accurate, current guidance.

### Step 2: Initialize Gemini Session

**CRITICAL: Always attach foundational files:**
```python
foundational_files = [
    "MCP-ASSISTANT-RULES.md",  # If exists
    "docs/ai-context/project-structure.md",
    "docs/ai-context/docs-overview.md"
]

session = mcp__gemini__consult_gemini(
    specific_question="[Clear, focused question]",
    problem_description="[Comprehensive context with constraints from CLAUDE.md]",
    code_context="[Relevant code snippets]",
    attached_files=foundational_files + [problem_specific_files],
    file_descriptions={
        "MCP-ASSISTANT-RULES.md": "Project vision and coding standards",
        "docs/ai-context/project-structure.md": "Complete tech stack and file structure",
        "docs/ai-context/docs-overview.md": "Documentation architecture",
        # Add problem-specific descriptions
    },
    preferred_approach="[solution/review/debug/optimize/explain]"
)
```

### Step 3: Engage in Deep Dialogue

**Think deeply about how to maximize value from the conversation:**

1. **Active Analysis**
   - What assumptions did Gemini make?
   - What needs clarification or deeper exploration?
   - What edge cases or alternatives should be discussed?
   - **If Gemini mentions external libraries:** Check Context7 MCP for current documentation to verify or supplement Gemini's guidance

2. **Iterative Refinement**
   ```python
   follow_up = mcp__gemini__consult_gemini(
       specific_question="[Targeted follow-up]",
       session_id=session["session_id"],
       additional_context="[New insights, questions, or implementation feedback]",
       attached_files=[newly_relevant_files]
   )
   ```

3. **Implementation Feedback Loop**
   Share actual code changes and real-world results to refine the approach.

### Step 4: Session Management

**Keep Sessions Open** - Don't close immediately. Maintain for the entire problem lifecycle.

**Only close when:**
- Problem is definitively solved and tested
- Topic is no longer relevant
- Fresh start would be more beneficial

**Monitor sessions:**
```python
active = mcp__gemini__list_sessions()
requests = mcp__gemini__get_gemini_requests(session_id="...")
```

## Key Patterns

### Clarification Pattern
"You mentioned [X]. In our context of [project specifics], how does this apply to [specific concern]?"

### Deep Dive Pattern
"Let's explore [aspect] further. What are the trade-offs given our [constraints]?"

### Alternative Pattern
"What if we approached this as [alternative]? How would that affect [concern]?"

### Progress Check Pattern
"I've implemented [changes]. Here's what happened: [results]. Should I adjust the approach?"

## Best Practices

1. **Think deeply** before each interaction - what will extract maximum insight?
2. **Be specific** - Vague questions get vague answers
3. **Show actual code** - Not descriptions
4. **Challenge assumptions** - Don't accept unclear guidance
5. **Document decisions** - Capture the "why" for future reference
6. **Stay curious** - Explore alternatives and edge cases
7. **Trust but verify** - Test all suggestions thoroughly

## Implementation Approach

When implementing Gemini's suggestions:
1. Start with the highest-impact changes
2. Test incrementally
3. Share results back with Gemini
4. Iterate based on real-world feedback
5. Document key insights in appropriate CONTEXT.md files

## Remember

- This is a **conversation**, not a query service
- **Context is king** - More context yields better guidance
- **Gemini sees patterns you might miss** - Be open to unexpected insights
- **Implementation reveals truth** - Share what actually happens
- Treat Gemini as a **collaborative thinking partner**, not an oracle

The goal is deep understanding and optimal solutions through iterative refinement, not quick answers.