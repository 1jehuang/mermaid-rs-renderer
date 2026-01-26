# MCP Assistant Rules - [Project Name]

## Project Context
[Brief description of what your project does and its main purpose. Keep it concise - 2-3 sentences max.]

### Core Vision & Architecture
- **Product Goal**: [Primary goal of your product]
- **Target Platform**: [Primary platform(s) - web, mobile, desktop, etc.]
- **Architecture**: [High-level architecture overview]
- **Key Technologies**: [Main technologies/frameworks used]

### Key Technical Principles
[List 4-6 core technical principles that guide your project]
- **Example**: Session-based architecture with clear boundaries
- **Example**: API-first design with versioning from day one
- **Example**: Security by default - validate all inputs at boundaries
- **Example**: Observable systems with structured logging

**Note:** The complete project structure and technology stack are provided in the attached `project-structure.md` file.

## Key Project Standards

### Core Principles
[List your fundamental development principles]
- Follow KISS, YAGNI, and DRY - prefer proven solutions over custom implementations
- Never mock, use placeholders, or omit code - always implement fully
- Be brutally honest about whether an idea is good or bad
- [Add project-specific principles]

### Code Organization
[Define your code organization standards]
- Keep files under [X] lines - split by extracting utilities, constants, types
- Single responsibility per file with clear purpose
- Prefer composition over inheritance
- [Add language/framework specific organization rules]

### [Language] Standards
[Replace with your primary language and its standards]
- Type safety requirements
- Naming conventions (classes, functions, constants)
- Documentation requirements (docstring style, required elements)
- Error handling patterns

### Error Handling & Logging
- Use specific exceptions with helpful messages
- Structured logging only - define your logging approach
- [Specify logging categories or patterns]
- Every request needs correlation ID for tracing

### API Design
[If applicable - define API standards]
- RESTful with consistent URL patterns
- Version from day one (/v1/, /v2/)
- Consistent response format
- Proper HTTP status codes

### Security & State
- Never trust external inputs - validate at boundaries
- [Define session/state management approach]
- [Specify data retention policies]
- Keep secrets in environment variables only

## Project-Specific Guidelines
[Add any project-specific guidelines that AI assistants should know]

### Domain-Specific Rules
[Add rules specific to your problem domain]

### Integration Points
[List key integration points or external services]

### Performance Considerations
[Add any performance-critical aspects]

## Important Constraints
- You cannot create, modify, or execute code
- You operate in a read-only support capacity
- Your suggestions are for the primary AI (Claude Code) to implement
- Focus on analysis, understanding, and advisory support

## Quick Reference
[Add frequently needed information]
- Key commands: [List common commands]
- Important paths: [List critical file paths]
- Documentation links: [Add links to detailed docs]