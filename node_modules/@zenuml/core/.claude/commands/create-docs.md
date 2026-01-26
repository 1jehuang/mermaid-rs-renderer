You are working on the VR Language Learning App project. The user has requested to create or regenerate documentation with the arguments: "$ARGUMENTS"

## Auto-Loaded Project Context:
@/CLAUDE.md
@/docs/ai-context/project-structure.md
@/docs/ai-context/docs-overview.md

## CRITICAL: AI-Optimized Documentation Principles
All documentation must be optimized for AI consumption and future-proofing:
- **Structured & Concise**: Use clear sections, lists, and hierarchies. Provide essential information only.
- **Contextually Complete**: Include necessary context, decision rationale ("why"), and cross-references.
- **Pattern-Oriented**: Make architectural patterns, conventions, and data flow explicit.
- **Modular & Scalable**: Structure for partial updates and project growth.
- **Cross-references**: Link related concepts with file paths, function names, and stable identifiers


---

## Step 1: Analyze & Strategize

Using the auto-loaded project context, analyze the user's request and determine the optimal documentation strategy.

### 1.1. Parse Target & Assess Complexity
**Action**: Analyze `$ARGUMENTS` to identify the `target_path` and its documentation tier.

**Target Classification:**
- **Tier 3 (Feature-Specific)**: Paths containing `/src/` and ending in `/CONTEXT.md`
- **Tier 2 (Component-Level)**: Paths ending in component root `/CONTEXT.md`

**Complexity Assessment Criteria:**
- **Codebase Size**: File count and lines of code in target directory
- **Technology Mix**: Diversity of languages and frameworks (Python, TypeScript, etc.)
- **Architectural Complexity**: Dependency graph and cross-component imports
- **Existing Documentation**: Presence and state of any CLAUDE.md files in the area

### 1.2. Select Strategy
Think deeply about this documentation generation task and strategy based on the auto-loaded project context. Based on the assessment, select and announce the strategy.

**Strategy Logic:**
- **Direct Creation**: Simple targets (< 15 files, single tech, standard patterns)
- **Focused Analysis**: Moderate complexity (15-75 files, 2-3 techs, some novel patterns)
- **Comprehensive Analysis**: High complexity (> 75 files, 3+ techs, significant architectural depth)

---

## Step 2: Information Gathering (Analysis Phase)

Based on the chosen strategy, gather the necessary information.

### Strategy A: Direct Creation
Proceed directly to **Step 3.1**. Perform lightweight analysis during content generation.

### Strategy B: Focused or Comprehensive Analysis (Sub-Agent Orchestration)

#### 2.1. Sub-Agent Roles
Select from these specialized roles based on complexity assessment:
- **`Code_Analyzer`**: File structure, implementation patterns, logic flow, coding conventions
- **`Tech_Stack_Identifier`**: Frameworks, libraries, dependencies, technology-specific patterns
- **`Architecture_Mapper`**: Cross-component dependencies, integration points, data flow
- **`Doc_Validator`**: Existing documentation accuracy, gaps, valuable insights, content overlap analysis

#### 2.2. Launch Sub-Agents
**Execution Plan:**
- **Focused Analysis (2-3 agents)**: `Code_Analyzer` + `Tech_Stack_Identifier` + `Doc_Validator` (if existing docs)
- **Comprehensive Analysis (3-4 agents)**: All agents as needed

**CRITICAL: Launch agents in parallel using a single message with multiple Task tool invocations for optimal performance.**

**Task Template:**
```
Task: "As the [Agent_Role], analyze the codebase at `[target_path]` to support documentation generation.

Your focus: [role-specific goal, e.g., 'identifying all architectural patterns and dependencies']

Standard workflow:
1. Review auto-loaded project context (CLAUDE.md, project-structure.md, docs-overview.md)
2. Analyze the target path for your specialized area
3. Return structured findings for documentation generation

Return a comprehensive summary of your findings for this role."
```

---

## Step 3: Documentation Generation

Think deeply about synthesizing findings and generating comprehensive documentation. Using gathered information, intelligently synthesize and generate the documentation content.

### 3.1. Content Synthesis & Generation

#### For Direct Creation (No Sub-Agents)
**Code-First Analysis Methodology:**
1. **Directory Structure Analysis**: Map file organization and purposes using Glob/LS
2. **Import Dependency Analysis**: Use Grep to identify integration patterns and dependencies  
3. **Pattern Extraction**: Read key files to identify architectural patterns and coding conventions
4. **Technology Usage Analysis**: Detect frameworks, libraries, and technology-specific patterns
5. **Existing Documentation Assessment**: Read any current CLAUDE.md files for valuable insights

#### For Sub-Agent Strategies  
**Synthesis Integration Process:**
1. **Compile Core Findings**: Merge agent findings for immediate documentation generation
2. **Extract Cross-Tier Patterns**: Identify system-wide patterns that may impact foundational documentation
3. **Resolve Information Conflicts**: When code contradicts existing docs, use code as source of truth
4. **Identify Content Gaps**: Find areas needing new documentation based on analysis
5. **Apply Project Conventions**: Use coding standards and naming conventions from the auto-loaded /CLAUDE.md
6. **Content Overlap Identification**: From Doc_Validator findings, identify existing documentation that overlaps with target content for later migration analysis

#### Content Generation Process
**For Both Approaches:**
1. **Select Template**: Choose Tier 2 or Tier 3 based on target classification
2. **Apply Content Treatment Strategy**:
   - **Preserve**: Validated architectural insights from existing documentation
   - **Enhance**: Extend existing patterns with newly discovered implementation details
   - **Replace**: Outdated content that conflicts with current code reality
   - **Create**: New documentation for undocumented patterns and decisions
3. **Populate Sections**: Fill template sections with synthesized findings
4. **Ensure Completeness**: Include architectural decisions, patterns, dependencies, and integration points
5. **Follow AI-Optimized Principles**: Structure for AI consumption with clear cross-references

### 3.2. Template Guidelines

**Tier 2 (Component-Level):**
```markdown
# [Component Name] - Component Context

## Purpose
[Component purpose and key responsibilities]

## Current Status: [Status]
[Status with evolution context and rationale]

## Component-Specific Development Guidelines
[Technology-specific patterns and conventions]

## Major Subsystem Organization
[High-level structure based on actual code organization]

## Architectural Patterns
[Core patterns and design decisions]

## Integration Points
[Dependencies and connections with other components]
```

**Tier 3 (Feature-Specific):**
```markdown
# [Feature Area] Documentation

## [Area] Architecture
[Key architectural elements and integration patterns]

## Implementation Patterns
[Core patterns and error handling strategies]

## Key Files and Structure
[File organization with purposes]

## Integration Points
[How this integrates with other parts of the system]

## Development Patterns
[Testing approaches and debugging strategies]
```

---

## Step 4: Finalization & Housekeeping

### 4.1. Write Documentation File
**Action**: Write the generated content to the target path.

### 4.2. Update Documentation Registry

#### Update docs-overview.md
**For new documentation files:**
- Add to appropriate tier section (Feature-Specific or Component-Level)
- Follow established entry format with path and description
- Maintain alphabetical ordering within sections

**For updated existing files:**
- Verify entry exists and description is current
- Update any changed purposes or scopes

#### Update Project Structure (if needed)
**If new directories were created:**
- Update file tree in `/docs/ai-context/project-structure.md`
- Add directory comments explaining purpose
- Maintain tree structure formatting and organization

### 4.3. Quality Validation
**Action**: Verify tier appropriateness, code accuracy, cross-reference validity, and consistency with existing documentation patterns.

### 4.4. Tier 1 Validation & Recommendations

**Action**: Compare discovered code patterns against foundational documentation to identify inconsistencies and improvement opportunities.

#### Process
1. **Discover Tier 1 Files**: Read `/docs/ai-context/docs-overview.md` to identify all foundational documentation files
2. **Read Foundational Docs**: Load discovered Tier 1 files to understand documented architecture
3. **Cross-Tier Analysis**: Using analysis findings from previous steps, compare:
   - **Technology Stack**: Discovered frameworks/tools vs documented stack
   - **Architecture Patterns**: Implementation reality vs documented decisions  
   - **Integration Points**: Actual dependencies vs documented integrations
4. **Generate Recommendations**: Output evidence-based suggestions for foundational documentation updates

### 4.5. Content Migration & Redundancy Management

**Action**: Intelligently manage content hierarchy and eliminate redundancy across documentation tiers.

#### Cross-Reference Analysis
1. **Identify Related Documentation**: Using Doc_Validator findings from Step 3.1 synthesis and target tier classification, identify existing documentation that may contain overlapping content
2. **Content Overlap Detection**: Compare new documentation content with existing files to identify:
   - **Duplicate Information**: Identical content that should exist in only one location
   - **Hierarchical Overlaps**: Content that exists at wrong tier level (implementation details in architectural docs)
   - **Cross-Reference Opportunities**: Content that should be linked rather than duplicated

#### Smart Content Migration Strategy
**Content Classification Framework:**
- **Tier-Appropriate Duplication**: High-level architectural context can exist at both Tier 2 and Tier 3 with different detail levels
- **Migration Candidates**: Detailed implementation patterns, specific code examples, feature-specific technical details
- **Reference Targets**: Stable architectural decisions, design rationale, cross-cutting concerns

**Migration Decision Logic:**
1. **For Tier 3 Creation (Feature-Specific)**:
   - **Extract from Tier 2**: Move feature-specific implementation details to new Tier 3 file
   - **Preserve in Tier 2**: Keep high-level architectural overview and design decisions
   - **Add Cross-References**: Link Tier 2 overview to detailed Tier 3 implementation

2. **For Tier 2 Creation (Component-Level)**:
   - **Consolidate from Multiple Tier 3**: Aggregate architectural insights from existing feature docs
   - **Preserve Tier 3 Details**: Keep implementation specifics in feature documentation
   - **Create Navigation Structure**: Add references to relevant Tier 3 documentation

#### Content Migration Execution
**Migration Process:**
1. **Identify Source Content**: Extract content that should migrate from existing files
2. **Content Transformation**: Adapt content to appropriate tier level (architectural vs implementation focus)
3. **Update Source Files**: Remove migrated content and add cross-references to new location
4. **Preserve Context**: Ensure source files maintain coherence after content removal
5. **Validate Migrations**: Confirm no broken references or lost information

**Safety Framework:**
- **Conservative Defaults**: When uncertain, preserve content in original location and add references
- **Content Preservation**: Never delete content without creating it elsewhere first
- **Migration Reversibility**: Document all migrations to enable rollback if needed

---

## Step 5: Generate Summary

Provide a comprehensive summary including:

### Documentation Creation Results
- **Documentation type and location** (Tier 2 or Tier 3)
- **Strategy used** (Direct Creation, Focused Analysis, or Comprehensive Analysis)
- **Key patterns documented** (architectural decisions, implementation patterns)
- **Registry updates made** (docs-overview.md, project-structure.md entries)

### Tier 1 Architectural Intelligence
**Based on Step 4.4 analysis, provide structured recommendations:**

#### Critical Updates Needed
- **File**: [specific foundational doc path]
- **Issue**: [specific inconsistency with evidence]
- **Recommendation**: [specific update needed]
- **Evidence**: [code references supporting the recommendation]

#### Architecture Enhancement Opportunities  
- **Gap Identified**: [missing foundational documentation area]
- **Scope**: [what should be documented]
- **Rationale**: [why this deserves foundational documentation]
- **Implementation Evidence**: [code patterns discovered]

#### Documentation Health Assessment
- **Alignment Score**: [overall consistency between code and docs]
- **Most Accurate Areas**: [foundational docs that match implementation well]
- **Areas Needing Attention**: [foundational docs with significant gaps/inconsistencies]
- **Systematic Improvement Priority**: [recommended order for addressing issues]

#### Content Migration Results
**Document all content hierarchy changes and redundancy eliminations:**

- **Content Migrated From**: [source file path] → [target file path]
  - **Content Type**: [e.g., "implementation patterns", "technical details", "architectural decisions"]
  - **Rationale**: [why this content belongs at the target tier]
  - **Cross-References Added**: [navigation links created between tiers]

- **Content Preserved At**: [broader tier file]
  - **Content Type**: [e.g., "architectural overview", "design decisions", "integration patterns"]
  - **Rationale**: [why this content remains at the broader tier]

- **Redundancies Eliminated**: 
  - **Duplicate Content Removed**: [specific duplications eliminated]
  - **Hierarchical Corrections**: [content moved to appropriate tier level]
  - **Reference Consolidations**: [areas where links replaced duplication]

- **Migration Safety**: 
  - **Content Preserved**: [confirmation that no information was lost]
  - **Rollback Information**: [documentation of changes for potential reversal]
  - **Validation Results**: [confirmation of no broken references]

#### Next Documentation Steps (Optional Recommendations)
- **Feature-Specific Documentation Candidates**: [suggest additional Tier 3 docs that would be valuable]
- **Cross-Component Documentation Needs**: [identify other components needing similar analysis]
- **Documentation Debt Eliminated**: [summary of redundancies and inconsistencies resolved]

---

Now proceed to create/regenerate documentation based on the request: $ARGUMENTS
