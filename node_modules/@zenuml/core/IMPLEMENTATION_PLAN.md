# Implementation Plan: Named Parameters Support

## Overview
Add support for named parameters in method calls, allowing syntax like `A.method(param=value)` alongside existing positional parameters.

## Current State Analysis

### Grammar Structure
- **Parser**: `src/g4/sequenceParser.g4` defines parameter rules at lines 220-230
- **Current parameter rule**: `parameter: declaration | expr`
- **Parameters rule**: `parameters: parameter (COMMA parameter)* COMMA?`
- **Invocation**: Method calls use `invocation: OPAR parameters? CPAR`

### Parser Logic
- **SignatureText.ts**: Handles parameter display via `getFormattedText()`
- **Parser.types.ts**: Defines `Parameter` and `Parameters` interfaces
- **Current behavior**: Parameters are parsed as expressions or type declarations

## Implementation Stages

### Stage 1: Grammar Extension
**Goal**: Extend ANTLR grammar to support named parameter syntax
**Success Criteria**: 
- Grammar accepts `param=value` syntax
- Backward compatibility with existing positional syntax
- Parser generates successfully

**Tasks**:
- [x] Add `namedParameter` rule: `ID ASSIGN expr`
- [x] Update `parameter` rule to include `namedParameter | declaration | expr`
- [x] Add tests for grammar parsing
- [x] Regenerate parser with `pnpm antlr`

**Tests**: 
- [x] `A.method(x=1, y=2)` parses correctly
- [x] `A.method(1, name="test")` mixed parameters work
- [x] Existing `A.method(1, 2)` still works

**Status**: Complete

### Stage 2: Parser Type Extensions
**Goal**: Extend parser types and interfaces to handle named parameters
**Success Criteria**:
- Named parameter data accessible in parser contexts
- Type definitions support both named and positional parameters

**Tasks**:
- [x] Extend `Parameter` interface to distinguish parameter types
- [x] Add `NamedParameter` interface with name and value properties
- [x] Update `SignatureText.ts` to handle named parameter formatting
- [x] Update type assertions and context handling

**Tests**:
- [x] Named parameters accessible via parser API
- [x] `getFormattedText()` displays named parameters correctly
- [x] Type safety maintained

**Status**: Complete

### Stage 3: Rendering Support
**Goal**: Update React components to render named parameters appropriately
**Success Criteria**:
- Named parameters displayed in sequence diagrams
- Clear visual distinction between parameter types
- Consistent formatting with existing design

**Tasks**:
- [x] Update message rendering components (via SignatureText.ts)
- [x] Enhance parameter display logic (formatParameters helper)
- [x] Ensure consistent formatting with existing design
- [x] Test rendering in development environment

**Tests**:
- [x] Named parameters render correctly: `method(userId=123,name="John")`
- [x] Mixed parameters work: `method(123,name="John",active=true)`
- [x] Backward compatibility maintained: `oldMethod(1,2,3)`
- [x] Complex scenarios: `mixedCall("first",second=456,"third")`

**Status**: Complete

### Stage 4: Test Coverage & Integration
**Goal**: Comprehensive testing and integration with existing features
**Success Criteria**:
- Full test coverage for named parameter scenarios
- E2E tests validate end-to-end functionality
- No regressions in existing functionality

**Tasks**:
- [ ] Unit tests for parser logic
- [ ] Component tests for rendering
- [ ] E2E tests with Playwright
- [ ] Performance regression testing
- [ ] Documentation updates

**Tests**:
- All test suites pass
- Performance benchmarks maintained
- Edge cases handled properly

**Status**: Not Started

### Stage 5: Documentation & Examples
**Goal**: User-facing documentation and examples
**Success Criteria**:
- Clear documentation of named parameter syntax
- Working examples in demo site
- Migration guidance for users

**Tasks**:
- [ ] Update grammar documentation
- [ ] Add examples to demo site
- [ ] Update README with new syntax
- [ ] API documentation updates

**Tests**:
- Documentation examples work correctly
- Demo site renders named parameter examples
- No broken links or outdated information

**Status**: Not Started

## Technical Considerations

### Backward Compatibility
- Existing positional parameter syntax must continue working
- Mixed parameter styles (positional + named) should be supported
- No breaking changes to existing APIs

### Performance Impact
- Minimal impact on parser performance
- Named parameter detection should be efficient
- Memory usage considerations for parameter storage

### Edge Cases
- Parameter name validation and uniqueness
- Error handling for invalid syntax
- Interaction with existing features (fragments, loops, etc.)

### Dependencies
- ANTLR4 parser regeneration
- No new external dependencies required
- Maintain compatibility with existing build system

## Risk Assessment

**Low Risk**:
- Grammar extension is straightforward
- Existing infrastructure supports parameter handling

**Medium Risk**:
- Rendering changes may require careful CSS updates
- Type system changes need thorough testing

**High Risk**:
- Parser performance impact needs monitoring
- Complex interaction with expression parsing

## Success Metrics
- [ ] All existing tests pass
- [ ] New functionality covered by tests
- [ ] No performance degradation
- [ ] Documentation complete
- [ ] Community feedback positive