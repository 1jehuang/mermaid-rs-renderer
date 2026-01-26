# Project Structure Template

This document provides a template for documenting the complete technology stack and file tree structure for your project. **AI agents MUST read this file to understand the project organization before making any changes.**

## Technology Stack Template

### Backend Technologies
Document your backend technology choices:
- **[Language] [Version]** with **[Package Manager]** - Dependency management and packaging
- **[Web Framework] [Version]** - Web framework with specific features (async, type hints, etc.)
- **[Server] [Version]** - Application server configuration
- **[Configuration] [Version]** - Configuration management approach

Example:
```
- Python 3.11+ with Poetry - Dependency management and packaging
- FastAPI 0.115.0+ - Web framework with type hints and async support
- Uvicorn 0.32.0+ - ASGI server with standard extras
- Pydantic Settings 2.5.2+ - Configuration management with type validation
```

### Integration Services & APIs
Document external services and integrations:
- **[Service Name] [API/SDK Version]** - Purpose and usage pattern
- **[AI Service] [Version]** - AI/ML service integration details
- **[Database] [Version]** - Data storage and management
- **[Monitoring] [Version]** - Observability and logging

### Real-time Communication
Document real-time features:
- **[WebSocket Library]** - Real-time communication patterns
- **[HTTP Client]** - Async HTTP communication
- **[Message Queue]** - Event processing (if applicable)

### Development & Quality Tools
Document development toolchain:
- **[Formatter] [Version]** - Code formatting
- **[Linter] [Version]** - Code quality and linting
- **[Type Checker] [Version]** - Static type checking
- **[Testing Framework] [Version]** - Testing approach
- **[Task Runner]** - Build automation and task orchestration

### Frontend Technologies (if applicable)
Document frontend technology stack:
- **[Language] [Version]** - Frontend development language
- **[Framework] [Version]** - UI framework
- **[Build Tool] [Version]** - Development and build tooling
- **[Deployment] [Version]** - Deployment and hosting approach

### Future Technologies
Document planned technology additions:
- **[Planned Technology]** - Future integration plans
- **[Platform]** - Target platform expansion
- **[Service]** - Planned service integrations

## Complete Project Structure Template

```
[PROJECT-NAME]/
├── README.md                           # Project overview and setup
├── CLAUDE.md                           # Master AI context file
├── [BUILD-FILE]                        # Build configuration (Makefile, package.json, etc.)
├── .gitignore                          # Git ignore patterns
├── .[IDE-CONFIG]/                      # IDE workspace configuration
│   ├── settings.[ext]                  # IDE settings
│   ├── extensions.[ext]                # Recommended extensions
│   └── launch.[ext]                    # Debug configurations
├── [BACKEND-DIR]/                      # Backend application
│   ├── CONTEXT.md                      # Backend-specific AI context
│   ├── src/                            # Source code
│   │   ├── config/                     # Configuration management
│   │   │   └── settings.[ext]          # Application settings
│   │   ├── core/                       # Core business logic
│   │   │   ├── CONTEXT.md              # Core logic patterns
│   │   │   ├── services/               # Business services
│   │   │   │   ├── [service1].[ext]    # Service implementations
│   │   │   │   └── [service2].[ext]
│   │   │   ├── models/                 # Data models
│   │   │   │   ├── [model1].[ext]      # Model definitions
│   │   │   │   └── [model2].[ext]
│   │   │   └── utils/                  # Utility functions
│   │   │       ├── logging.[ext]       # Structured logging
│   │   │       ├── validation.[ext]    # Input validation
│   │   │       └── helpers.[ext]       # Helper functions
│   │   ├── api/                        # API layer
│   │   │   ├── CONTEXT.md              # API patterns and conventions
│   │   │   ├── routes/                 # API route definitions
│   │   │   │   ├── [resource1].[ext]   # Resource-specific routes
│   │   │   │   └── [resource2].[ext]
│   │   │   ├── middleware/             # API middleware
│   │   │   │   ├── auth.[ext]          # Authentication middleware
│   │   │   │   ├── logging.[ext]       # Request logging
│   │   │   │   └── validation.[ext]    # Request validation
│   │   │   └── schemas/                # Request/response schemas
│   │   │       ├── [schema1].[ext]     # Data schemas
│   │   │       └── [schema2].[ext]
│   │   └── integrations/               # External service integrations
│   │       ├── CONTEXT.md              # Integration patterns
│   │       ├── [service1]/             # Service-specific integration
│   │       │   ├── client.[ext]        # API client
│   │       │   ├── models.[ext]        # Integration models
│   │       │   └── handlers.[ext]      # Response handlers
│   │       └── [service2]/
│   ├── tests/                          # Test suite
│   │   ├── unit/                       # Unit tests
│   │   ├── integration/                # Integration tests
│   │   └── fixtures/                   # Test fixtures and data
│   ├── [PACKAGE-FILE]                  # Package configuration
│   └── [ENV-FILE]                      # Environment configuration
├── [FRONTEND-DIR]/                     # Frontend application (if applicable)
│   ├── CONTEXT.md                      # Frontend-specific AI context
│   ├── src/                            # Source code
│   │   ├── components/                 # UI components
│   │   │   ├── CONTEXT.md              # Component patterns
│   │   │   ├── common/                 # Shared components
│   │   │   └── [feature]/              # Feature-specific components
│   │   ├── pages/                      # Page components/routes
│   │   │   ├── [page1].[ext]           # Page implementations
│   │   │   └── [page2].[ext]
│   │   ├── stores/                     # State management
│   │   │   ├── CONTEXT.md              # State management patterns
│   │   │   ├── [store1].[ext]          # Store implementations
│   │   │   └── [store2].[ext]
│   │   ├── api/                        # API client layer
│   │   │   ├── CONTEXT.md              # Client patterns
│   │   │   ├── client.[ext]            # HTTP client setup
│   │   │   └── endpoints/              # API endpoint definitions
│   │   ├── utils/                      # Utility functions
│   │   │   ├── logging.[ext]           # Client-side logging
│   │   │   ├── validation.[ext]        # Form validation
│   │   │   └── helpers.[ext]           # Helper functions
│   │   └── assets/                     # Static assets
│   ├── tests/                          # Frontend tests
│   ├── [BUILD-CONFIG]                  # Build configuration
│   └── [PACKAGE-FILE]                  # Package configuration
├── docs/                               # Documentation
│   ├── ai-context/                     # AI-specific documentation
│   │   ├── project-structure.md        # This file
│   │   ├── docs-overview.md            # Documentation architecture
│   │   ├── system-integration.md       # Integration patterns
│   │   ├── deployment-infrastructure.md # Infrastructure docs
│   │   └── handoff.md                  # Task management
│   ├── api/                            # API documentation
│   ├── deployment/                     # Deployment guides
│   └── development/                    # Development guides
├── scripts/                            # Automation scripts
│   ├── setup.[ext]                     # Environment setup
│   ├── deploy.[ext]                    # Deployment scripts
│   └── maintenance/                    # Maintenance scripts
├── [INFRASTRUCTURE-DIR]/               # Infrastructure as code (if applicable)
│   ├── [PROVIDER]/                     # Cloud provider configurations
│   ├── docker/                         # Container configurations
│   └── monitoring/                     # Monitoring and alerting
└── [CONFIG-FILES]                      # Root-level configuration files
```


---

*This template provides a comprehensive foundation for documenting project structure. Adapt it based on your specific technology stack, architecture decisions, and organizational requirements.*