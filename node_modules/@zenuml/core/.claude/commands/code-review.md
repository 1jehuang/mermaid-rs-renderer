# /code-review

*Performs focused multi-agent code review that surfaces only critical, high-impact findings for solo developers using AI tools.*

## Core Philosophy

This command prioritizes **needle-moving discoveries** over exhaustive lists. Every finding must demonstrate significant impact on:
- System reliability & stability
- Security vulnerabilities with real exploitation risk
- Performance bottlenecks affecting user experience
- Architectural decisions blocking future scalability
- Critical technical debt threatening maintainability

### 🚨 Critical Findings Only
Issues that could cause production failures, security breaches, or severe user impact within 48 hours.

### 🔥 High-Value Improvements
Changes that unlock new capabilities, remove significant constraints, or improve metrics by >25%.

### ❌ Excluded from Reports
Minor style issues, micro-optimizations (<10%), theoretical best practices, edge cases affecting <1% of users.


## Auto-Loaded Project Context:
@/CLAUDE.md
@/docs/ai-context/project-structure.md
@/docs/ai-context/docs-overview.md


## Command Execution

User provided context: "$ARGUMENTS"

### Step 1: Understand User Intent & Gather Context

#### Parse the Request
Analyze the natural language input to determine:
1. **What to review**: Parse file paths, component names, feature descriptions, or commit references
2. **Review focus**: Identify any specific concerns mentioned (security, performance, etc.)
3. **Scope inference**: Intelligently determine the breadth of review needed

Examples of intent parsing:
- "the authentication flow" → Find all files related to auth across the codebase
- "voice pipeline implementation" → Locate voice processing components
- "recent changes" → Parse git history for relevant commits
- "the API routes" → Identify all API endpoint files

#### Read Relevant Documentation
Before allocating agents, **read the documentation** to understand:
1. Use `/docs/ai-context/docs-overview.md` to identify relevant docs
2. Read documentation related to the code being reviewed:
   - Architecture docs for subsystem understanding
   - API documentation for integration points
   - Security guidelines for sensitive areas
   - Performance considerations for critical paths
3. Build a mental model of risks, constraints, and priorities

This context ensures intelligent agent allocation based on actual project knowledge.

### Step 2: Define Mandatory Coverage Areas

Every code review MUST analyze these core areas, with depth determined by scope:

#### 🎯 Mandatory Coverage Areas:

1. **Critical Path Analysis**
   - User-facing functionality that could break
   - Data integrity and state management
   - Error handling and recovery mechanisms

2. **Security Surface**
   - Input validation and sanitization
   - Authentication/authorization flows
   - Data exposure and API security

3. **Performance Impact**
   - Real-time processing bottlenecks
   - Resource consumption (memory, CPU)
   - Scalability constraints

4. **Integration Points**
   - API contracts and boundaries
   - Service dependencies
   - External system interactions

#### 📊 Dynamic Agent Allocation:

Based on review scope, allocate agents proportionally:

**Small to medium Scope (small set of files or small feature)**
- 2-3 agents covering mandatory areas
- Each agent handles 1-2 coverage areas
- Focus on highest-risk aspects

**Large Scope (many files, major feature or subsystem)**
- 4-6 agents with specialized focus
- Each mandatory area gets dedicated coverage
- Additional agents for cross-cutting concerns

### Step 3: Dynamic Agent Generation

Based on scope analysis and mandatory coverage areas, dynamically create specialized agents:

#### Agent Generation Strategy:

**With your documentation knowledge from Step 1, think deeply** about optimal agent allocation:
- Leverage your understanding of the project architecture and risks
- Consider the specific documentation you read about this subsystem
- Apply insights about critical paths and security considerations
- Use documented boundaries and integration points to partition work
- Factor in any performance or scalability concerns from the docs

Use your understanding of the project to intuitively determine:
1. **How many agents are needed** - Let the code's complexity and criticality guide you
2. **How to partition the work** - Follow natural architectural boundaries
3. **Which specializations matter most** - Focus agents where risk is highest

**Generate Specialized Agents**

   For each allocated agent, create a focused role:

   **Example for 6-agent allocation:**
   - Agent 1: Critical_Path_Validator (user flows + error handling)
   - Agent 2: Security_Scanner (input validation + auth)
   - Agent 3: API_Security_Auditor (data exposure + boundaries)
   - Agent 4: Performance_Profiler (bottlenecks + resource usage)
   - Agent 5: Scalability_Analyst (constraints + growth paths)
   - Agent 6: Integration_Verifier (dependencies + contracts)

   **Example for 3-agent allocation:**
   - Agent 1: Security_Performance_Analyst (security + performance areas)
   - Agent 2: Critical_Path_Guardian (functionality + integrations)
   - Agent 3: Risk_Quality_Assessor (technical debt + code quality)

#### Dynamic Focus Areas:

Each agent receives specialized instructions based on:
- **File characteristics**: API endpoints → security focus
- **Code patterns**: Loops/algorithms → performance focus
- **Dependencies**: External services → integration focus
- **User touchpoints**: UI/voice → critical path focus

### Step 4: Execute Dynamic Multi-Agent Review

**Before launching agents, pause and think deeply:**
- What are the real risks in this code?
- Which areas could cause the most damage if they fail?
- Where would a solo developer need the most help?

Generate and launch agents based on your thoughtful analysis:

```
For each dynamically generated agent:
  Task: "As [Agent_Role], analyze [assigned_coverage_areas] in [target_scope].

  MANDATORY COVERAGE CHECKLIST:
  ☐ Critical Path: [assigned aspects]
  ☐ Security: [assigned aspects]
  ☐ Performance: [assigned aspects]
  ☐ Integration: [assigned aspects]

  HIGH-IMPACT REVIEW MANDATE:
  Focus ONLY on findings that significantly move the needle for a solo developer.

  Review workflow:
  1. Review auto-loaded project context (CLAUDE.md, project-structure.md, docs-overview.md)
  2. Analyze your assigned coverage areas with deep focus
  3. For complex issues, use:
     - mcp__gemini__consult_gemini for architectural analysis
     - mcp__context7__get-library-docs for framework best practices
  4. Cross-reference with other coverage areas for systemic issues
  5. Document ONLY high-impact findings:

     ## [Coverage_Area] Analysis by [Agent_Role]

     ### 🚨 Critical Issues (Production Risk)
     - Issue: [description]
     - Location: [file:line_number]
     - Impact: [quantified - downtime hours, users affected, data at risk]
     - Fix: [specific code snippet]
     - Consequence if ignored: [what happens in 48 hours]

     ### 🎯 Strategic Improvements (Capability Unlocks)
     - Limitation: [what's currently blocked]
     - Solution: [architectural change or implementation]
     - Unlocks: [new capability or scale]
     - ROI: [effort hours vs benefit quantified]

     ### ⚡ Quick Wins (Optional)
     - Only include if <2 hours for >20% improvement
     - Must show measurable impact

  REMEMBER: Every finding must pass the 'so what?' test for a solo developer."
```

#### Parallel Execution Strategy:

**Launch all agents simultaneously** for maximum efficiency


### Step 5: Synthesize Findings with Maximum Analysis Power

After all sub-agents complete their analysis:

**ultrathink**

Activate maximum cognitive capabilities to:

1. **Filter for Impact**
   - Discard all low-priority findings
   - Quantify real-world impact of each issue
   - Focus on production risks and capability unlocks

2. **Deep Pattern Analysis**
   - Identify systemic issues vs isolated problems
   - Find root causes across agent reports
   - Detect subtle security vulnerabilities

3. **Strategic Prioritization**
   - Calculate ROI for each improvement
   - Consider solo developer constraints
   - Create actionable fix sequence
   ```markdown
   # Code Review Summary

   **Reviewed**: [scope description]
   **Date**: [current date]
   **Overall Quality Score**: [A-F grade with justification]

   ## Key Metrics
   - Security Risk Level: [Critical/High/Medium/Low]
   - Performance Impact: [description]
   - Technical Debt: [assessment]
   - Test Coverage: [if applicable]
   ```

### Step 6: Present Comprehensive Review

Structure the final output as:

```markdown
# 🔍 Code Review Report

## Executive Summary
[High-level findings and overall assessment]

## 🚨 Production Risks (Fix Within 48 Hours)
[Only issues that could cause downtime, data loss, or security breaches]

## 🎯 Strategic Improvements (High ROI)
[Only changes that unlock capabilities or improve metrics >25%]

## ⚡ Quick Wins (Optional)
[Only if <2 hours effort for significant improvement]

## Detailed Analysis

### Security Assessment
[Detailed security findings from Security_Auditor]

### Performance Analysis
[Detailed performance findings from Performance_Analyzer]

### Architecture Review
[Detailed architecture findings from Architecture_Validator]

### Code Quality Evaluation
[Detailed quality findings from Quality_Inspector]

[Additional sections based on sub-agents used]

## Action Plan
1. Critical fixes preventing production failures
2. High-ROI improvements unlocking capabilities

## Impact Matrix
| Issue | User Impact | Effort | ROI |
|-------|-------------|--------|-----|
| [Only high-impact issues with quantified metrics] |
```

### Step 7: Interactive Follow-up

After presenting the review, offer interactive follow-ups. For example:
- "Would you like me to fix any of the critical issues?"
- "Should I create a detailed refactoring plan for any component?"
- "Do you want me to generate tests for uncovered code?"
- "Should I create GitHub issues for tracking these improvements?"

## Implementation Notes

1. **Use parallel Task execution** for all sub-agents to minimize review time
2. **Include file:line_number references** for easy navigation
3. **Balance criticism with recognition** of good practices
4. **Provide actionable fixes**, not just problem identification
5. **Consider project phase** and priorities when recommending changes
6. **Use MCP servers** for specialized analysis when beneficial
7. **Keep security findings sensitive** - don't expose vulnerabilities publicly

## Error Handling

### Coverage Verification

Before presenting results, verify complete coverage:

```
☑ Critical Path Analysis: [Covered by agents X, Y]
☑ Security Surface: [Covered by agents Y, Z]
☑ Performance Impact: [Covered by agents X, Z]
☑ Integration Points: [Covered by agents W, X]
```

If any area lacks coverage, deploy additional focused agents.

## Error Handling

If issues occur during review:
- **Ambiguous input**: Use search tools to find relevant files before asking for clarification
- **File not found**: Search for similar names or components across the codebase
- **Large scope detected**: Dynamically scale agents based on calculated complexity
- **No files found**: Provide helpful suggestions based on project structure
- **Coverage gaps**: Deploy supplementary agents for missed areas
