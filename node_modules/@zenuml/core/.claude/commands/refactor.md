You are working on the VR Language Learning App project. The user has requested to refactor specific files tagged with @ symbols in their arguments: "$ARGUMENTS"

## Auto-Loaded Project Context:
@/CLAUDE.md
@/docs/ai-context/project-structure.md
@/docs/ai-context/docs-overview.md

## Step 1: Parse Tagged Files
Extract all @ tagged file paths from the user's arguments. Only process files that are explicitly tagged with @ symbols.

**Example parsing:**
- Input: "refactor @src/big-file.ts @components/Large.svelte"
- Extract: ["src/big-file.ts", "components/Large.svelte"]

## Step 2: Validate and Analyze Files
For each tagged file:
1. **Verify file exists** - If file doesn't exist, inform user and skip
2. **Read file contents** - Understand the structure and dependencies
3. **Analyze current directory structure** - Map existing patterns around the file

## Step 3: Intelligent Analysis Strategy Decision
Think deeply about the safest and most effective refactoring approach based on the auto-loaded project context. Based on the initial analysis from Step 2 and the auto-loaded project context, intelligently decide the optimal approach for each file:

### Strategy Options:

**Direct Refactoring** (0-1 sub-agents):
- Simple files with clear, obvious split points
- Files with minimal external dependencies
- Standard refactoring patterns (e.g., extract utils, split large classes)
- Low risk of breaking changes

**Focused Analysis** (2-3 sub-agents):
- Moderate complexity with specific concerns
- Files with moderate dependency footprint
- When one aspect needs deep analysis (e.g., complex dependencies OR intricate file structure)

**Comprehensive Analysis** (3+ sub-agents):
- High complexity files with multiple concerns
- Extensive dependency networks
- Novel refactoring patterns not seen in project
- High risk of breaking changes
- Files that are central to multiple systems

## Step 4: Execute Chosen Strategy

### For Direct Refactoring:
Proceed with straightforward refactoring using the initial analysis and project context.

### For Sub-Agent Approaches:
You have complete autonomy to design and launch sub-agents based on the specific refactoring needs identified. Consider these key investigation areas and design custom agents to cover what's most relevant:

**Core Investigation Areas to Consider:**
- **File Structure Analysis**: Logical component boundaries, split points, cohesion assessment
- **Dependency Network Mapping**: Import/export analysis, usage patterns, circular dependency risks
- **Project Pattern Compliance**: Directory structures, naming conventions, organizational patterns
- **Impact Assessment**: Test files, configuration files, build scripts that need updates
- **Import Update Analysis**: All files that import from the target file and need updated import paths
- **Technology Stack Considerations**: Language-specific patterns, framework conventions

**Autonomous Sub-Agent Design Principles:**
- **Custom Specialization**: Define agents based on the specific file's complexity and risks
- **Flexible Agent Count**: Use as many agents as needed - scale based on actual complexity
- **Adaptive Coverage**: Ensure critical aspects are covered without unnecessary overlap
- **Risk-Focused Analysis**: Prioritize investigation of the highest-risk refactoring aspects

**Sub-Agent Task Template:**
```
Task: "Analyze [SPECIFIC_INVESTIGATION_AREA] for safe refactoring of [TARGET_FILE] related to user request '$ARGUMENTS'"

Standard Investigation Workflow:
1. Review auto-loaded project context (CLAUDE.md, project-structure.md, docs-overview.md)
2. [CUSTOM_ANALYSIS_STEPS] - Investigate the specific area thoroughly
3. Return actionable findings that support safe and effective refactoring

Return comprehensive findings addressing this investigation area."
```

**CRITICAL: When launching sub-agents, always use parallel execution with a single message containing multiple Task tool invocations.**


## Step 5: Synthesize Analysis and Plan Refactoring

Think deeply about integrating findings from all sub-agent investigations for safe and effective refactoring. Combine findings from all agents to create optimal refactoring strategy:

### Integration Analysis
- **File Structure**: Use File Analysis Agent's component breakdown
- **Organization**: Apply Pattern Recognition Agent's directory recommendations
- **Safety**: Implement Dependency Analysis Agent's import/export strategy
- **Completeness**: Address Impact Assessment Agent's broader concerns

### Refactoring Strategy Decision
Based on synthesized analysis, determine:
- **Split granularity**: How many files and what logical divisions
- **Directory structure**: Same-level, subdirectory, or existing directory placement
- **Import/export strategy**: How to restructure exports and update all consuming files
- **File naming**: Following project conventions and clarity

### Risk Assessment
- **Breaking changes**: Identify and mitigate potential issues
- **Dependency conflicts**: Plan import/export restructuring
- **Test impacts**: Plan for test file updates
- **Documentation needs**: Identify doc updates required

## Step 6: Refactoring Value Assessment

### Evaluate Refactoring Worth
After synthesizing all analysis, critically evaluate whether the proposed refactoring will actually improve the codebase:

**Positive Indicators (Worth Refactoring):**
- File significantly exceeds reasonable size limits (500+ lines for components, 1000+ for utilities)
- Clear separation of concerns violations (UI mixed with business logic, multiple unrelated features)
- High cyclomatic complexity that would be reduced
- Repeated code patterns that could be abstracted
- Poor testability that would improve with modularization
- Dependencies would become cleaner and more maintainable
- Aligns with project's architectural patterns

**Negative Indicators (Not Worth Refactoring):**
- File is already well-organized despite its size
- Splitting would create artificial boundaries that reduce clarity
- Would introduce unnecessary complexity or abstraction
- Dependencies would become more convoluted
- File serves a single, cohesive purpose effectively
- Refactoring would violate project conventions
- Minimal actual improvement in maintainability

### Decision Point
Based on the assessment:

**If Refactoring IS Worth It:**
- Print clear summary of benefits: "✅ This refactoring will improve the codebase by: [specific benefits]"
- Proceed automatically to Step 7 (Execute Refactoring)

**If Refactoring IS NOT Worth It:**
- Be brutally honest about why: "❌ This refactoring is not recommended because: [specific reasons]"
- Explain what makes the current structure acceptable
- Ask user explicitly: "The file is currently well-structured for its purpose. Do you still want to proceed with the refactoring? (yes/no)"
- Only continue if user confirms

## Step 7: Execute Refactoring

Implement the refactoring based on the synthesized analysis:

### File Creation Order
1. **Create directories** - Create any new subdirectories needed
2. **Create core files** - Start with main/index files
3. **Create supporting files** - Types, utils, constants
4. **Update imports** - Fix all import/export statements
5. **Update original file** - Replace with new modular structure

### Import/Export Management
- **Update all consuming files** - Modify import statements to point to new file locations
- **Restructure exports** - Organize exports in the new file structure
- **Update relative imports** - Fix paths throughout the codebase
- **Follow naming conventions** - Use project's established patterns

### Quality Assurance
- **Preserve functionality** - Ensure no breaking changes
- **Maintain type safety** - Keep all TypeScript types intact
- **Follow coding standards** - Apply project's style guidelines
- **Test compatibility** - Verify imports work correctly


## Step 8: Quality Verification

For each refactored file:
- **Check imports** - Verify all imports resolve correctly
- **Run type checks** - Ensure TypeScript compilation passes
- **Test functionality** - Confirm no breaking changes
- **Validate structure** - Ensure new organization follows project patterns


## Error Handling
- **File not found** - Skip and inform user
- **Not worth refactoring** - Skip files that are good as is and give users an explanation.
- **Parse errors** - Report syntax issues and skip
- **Import conflicts** - Resolve or report issues

## Summary Format
Provide a comprehensive summary of:
- **Analysis Results**: Key findings from each sub-agent
- **Refactoring Strategy**: Chosen approach and rationale
- **Value Assessment**: Whether refactoring improves the code (from Step 6)
- **Files Created**: New structure with explanations (if refactoring proceeded)
- **Dependencies Fixed**: Import/export changes made (if refactoring proceeded)
- **Issues Encountered**: Any problems and resolutions

Now proceed with multi-agent analysis and refactoring of the tagged files: $ARGUMENTS
