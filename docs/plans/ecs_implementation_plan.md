# ECS Implementation Plan: Archetype-Based System

## Overview

Implement a modern Rust ECS inspired by classic ID Software engines (Doom/Quake). This plan implements **Option 2 (Archetype-Based ECS)** first, then migrates to **Option 5 (Hybrid with Type Erasure)** to gain additional flexibility and structure.

**Target Location**: `src/ecs/`

**Estimated Complexity**: Medium

---

## Option Definitions

### Option 2: Archetype-Based ECS

**Core Idea**: Group entities by their component composition into "archetypes". Each archetype stores components in parallel arrays, maintaining cache efficiency.

**Characteristics**:
- Entities with identical component types share an archetype
- Components stored in `Vec<Option<T>>` per archetype
- Type erasure via `dyn Any` for storage heterogeneity
- Entity migration between archetypes when components change
- Fast iteration over entities with specific component combinations

**Pros**:
- Cache-friendly memory layout (components contiguous within archetype)
- Efficient queries (filter by archetype first, then access components)
- Flexible component combinations per entity
- Simple mental model: "entities with same components live together"

**Cons**:
- Migration overhead when adding/removing components
- Type erasure adds some runtime overhead for component access
- More complex implementation than flat arrays

### Option 5: Hybrid with Type Erasure

**Core Idea**: Enhance Option 2 with a structured type-erasure layer that provides a cleaner abstraction over component storage while maintaining the same memory layout benefits.

**Characteristics**:
- Same archetype-based grouping as Option 2
- Introduces `TypedComponentStorage` trait for abstract component access
- Uses `ConcreteComponentStorage<T>` for type-specific storage
- Wraps in `EnhancedComponentStorage` for type-erased interface
- Provides both direct index-based access and type-safe downcasting

**Pros**:
- All benefits of Option 2 (cache efficiency, archetype grouping)
- Cleaner abstraction over storage operations
- More explicit control over entity migration
- Better separation of concerns (storage vs. access)
- Easier to extend with new storage backends

**Cons**:
- Slightly more complex implementation
- Additional trait indirection may have minor performance impact
- More code to maintain

### Migration Path: Option 2 → Option 5

The migration preserves all functionality while improving the internal architecture:

1. **Storage Abstraction**: Replace ad-hoc `dyn Any` usage with structured `TypedComponentStorage` trait
2. **Access Methods**: Add direct index-based access alongside existing type-based access
3. **Migration Logic**: Make entity migration more explicit and controllable
4. **Query System**: Optimize queries to leverage the new storage structure

**Key Insight**: Option 5 is essentially a refactoring of Option 2 with better abstraction. The external API can remain largely unchanged.

### Comparison: Option 2 vs Option 5

| Aspect | Option 2 (Archetype-Based) | Option 5 (Hybrid) |
|--------|----------------------------|-------------------|
| **Storage Grouping** | Archetypes by component composition | Same |
| **Component Access** | Direct type-based downcasting | Trait-based with downcast support |
| **Storage Abstraction** | Ad-hoc `dyn Any` | Structured `TypedComponentStorage` trait |
| **Migration Control** | Implicit in add/remove | Explicit methods available |
| **Query Performance** | Good | Same or better |
| **Implementation Complexity** | Moderate | Higher |
| **Extensibility** | Limited | Better |
| **Type Safety** | Runtime checks | Runtime checks + better abstraction |

### Architecture Diagram

```
Option 2 (Archetype-Based):
┌─────────────────────────────────────────────────────┐
│                      World                               │
│  ┌─────────────┐  ┌─────────────────────────────────┐ │
│  │ Entity List │  │ Archetypes (by component types)  │ │
│  └─────────────┘  └─────────────────────────────────┘ │
│                    ┌─────────────────────────────────┐ │
│                    │ Archetype [Position, Velocity]   │ │
│                    │  ┌─────────────┐  ┌─────────────┐│ │
│                    │  │ Positions   │  │ Velocities   ││ │
│                    │  │ [Vec<Option>]│  │ [Vec<Option>]││ │
│                    │  └─────────────┘  └─────────────┘│ │
│                    │  Entities: [e1, e2, e3]          │ │
│                    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘

Option 5 (Hybrid) - Same structure, but with:
┌─────────────────────────────────────────────────────┐
│  EnhancedComponentStorage                              │
│  ┌─────────────────────────────────────────────────┐ │
│  │ TypedComponentStorage trait                       │ │
│  │  ┌─────────────┐  ┌─────────────┐                │ │
│  │  │ Concrete     │  │ Concrete     │                │ │
│  │  │ Storage<Pos> │  │ Storage<Vel> │                │ │
│  │  └─────────────┘  └─────────────┘                │ │
│  └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

---

## Goals

1. **Cache Efficiency**: Group entities by component composition (archetypes) for contiguous memory access
2. **Flexibility**: Allow any combination of components per entity
3. **Type Safety**: Leverage Rust's type system where possible
4. **Performance**: Minimize runtime overhead for common operations
5. **Extensibility**: Design for future enhancements (serialization, networking, etc.)

---

## Architecture

### Core Concepts

1. **Entity**: Simple ID (u32 with generational support later)
2. **Component**: Data-only structs implementing `Component` trait
3. **Archetype**: Group of entities sharing the same component composition
4. **World**: Central container managing all entities and archetypes

### Module Structure

```
src/ecs/
├── mod.rs          # Public API exports
├── entity.rs       # EntityId and related types
├── component.rs    # Component trait and storage abstractions
├── archetype.rs    # Archetype definition and operations
├── world.rs        # World management (spawn/despawn, component access)
└── query.rs        # Query system for entity filtering
```

---

## Implementation Phases

### Phase 1: Foundation (Option 2 Core)

**Objective**: Implement basic ECS with archetype-based storage.

#### 1.1 Entity System
- [ ] Define `EntityId` as a newtype around `u32`
- [ ] Implement `Default`, `Copy`, `Eq`, `Hash` for `EntityId`
- [ ] Add special constants: `NULL` (0), `MAX` (u32::MAX)

#### 1.2 Component System
- [ ] Define `Component` trait with `'static + Send + Sync` bounds
- [ ] Provide blanket implementation for all valid types
- [ ] Create `ComponentStorage` struct for type-erased component containers
- [ ] Implement downcasting methods for type-safe access

#### 1.3 Archetype System
- [ ] Define `Archetype` struct containing:
  - Set of component `TypeId`s
  - Parallel storage vectors for each component type
  - List of entity IDs
- [ ] Create `ArchetypeKey` for consistent archetype lookup (sorted `Vec<TypeId>`)
- [ ] Implement archetype creation, entity addition/removal
- [ ] Add methods to check component composition

#### 1.4 World Management
- [ ] Define `World` struct containing:
  - Entity ID counter and free list
  - Entity to archetype mapping
  - Archetype collection
- [ ] Implement core operations:
  - `spawn()` / `despawn()`
  - `add_component()` / `remove_component()`
  - `get_component()` / `get_component_mut()`
  - `has_component()`
- [ ] Implement entity migration between archetypes when components change

#### 1.5 Basic Query Support
- [ ] Add iterator methods to `World`:
  - `iter_entities()`
  - `iter_with<T>()` for single-component iteration
  - `iter_with2<T1, T2>()` for two-component iteration

**Validation**: Unit tests for all core operations (see Phase 3)

---

### Phase 2: Query System

**Objective**: Add flexible querying capabilities.

#### 2.1 Query Builder
- [ ] Implement `Query` struct with builder pattern
- [ ] Add `with<T>()` method to require components
- [ ] Implement `run()` to execute query and return entity IDs
- [ ] Implement `run_with_components<T>()` to return entities with components

#### 2.2 Typed Queries
- [ ] Create `TypedQuery<T>` for type-safe single-component queries
- [ ] Create `TypedQuery2<T1, T2>` for two-component queries
- [ ] Add extension traits to `World` for ergonomic syntax

#### 2.3 Archetype-Aware Queries
- [ ] Add method to query by exact archetype key
- [ ] Implement efficient iteration over archetypes matching component requirements

**Validation**: Query performance tests, correctness tests

---

### Phase 3: Testing and Validation

**Objective**: Ensure correctness and performance of the implementation.

#### 3.1 Unit Tests
- [ ] Test entity creation and destruction
- [ ] Test component addition and removal
- [ ] Test component access (get/get_mut)
- [ ] Test archetype migration when adding/removing components
- [ ] Test query system with various component combinations
- [ ] Test iterator methods

#### 3.2 Integration Tests
- [ ] Create a simple game simulation with multiple entity types
- [ ] Test with realistic component combinations (Position, Velocity, Health, etc.)
- [ ] Verify memory safety with concurrent access (if applicable)

#### 3.3 Performance Tests
- [ ] Benchmark entity spawn/despawn
- [ ] Benchmark component access patterns
- [ ] Benchmark query performance with varying entity counts
- [ ] Compare cache efficiency vs. non-archetype approaches

---

## Phase 4: Migration to Option 5 (Hybrid Approach)

**Objective**: Enhance the implementation with structured type erasure for better abstraction.

### 4.1 Enhanced Component Storage
- [ ] Define `TypedComponentStorage` trait with abstract access methods
- [ ] Implement `ConcreteComponentStorage<T>` for specific types
- [ ] Create `EnhancedComponentStorage` wrapper using the trait
- [ ] Add methods for type-safe downcasting

### 4.2 Storage Refactoring
- [ ] Update `Archetype` to use `EnhancedComponentStorage`
- [ ] Refactor component access to use the new storage system
- [ ] Ensure backward compatibility with existing API

### 4.3 Direct Access Methods
- [ ] Add `get_component_direct()` using enhanced storage
- [ ] Add `get_component_mut_direct()` for mutable access
- [ ] Update `add_component_enhanced()` to use new storage

### 4.4 Query System Enhancements
- [ ] Optimize queries to leverage the new storage system
- [ ] Add support for more complex query patterns
- [ ] Improve iteration performance

**Validation**: Repeat Phase 3 tests to ensure no regressions

---

## Phase 5: Advanced Features (Optional)

**Objective**: Add features for real-world usage.

### 5.1 Generational Entity IDs
- [ ] Replace simple `u32` with generational IDs to prevent use-after-free
- [ ] Update all entity-related operations

### 5.2 Systems and Scheduling
- [ ] Define `System` trait for game logic
- [ ] Implement system execution order
- [ ] Add system scheduling

### 5.3 Hierarchical Entities
- [ ] Add `Parent` and `Children` components
- [ ] Implement hierarchy management
- [ ] Add methods for traversing entity hierarchies

### 5.4 Serialization Support
- [ ] Implement `Serialize`/`Deserialize` for `World`
- [ ] Add component registration for serialization
- [ ] Support for saving/loading game state

### 5.5 Change Detection
- [ ] Track component additions/removals
- [ ] Implement change detection for systems
- [ ] Add dirty flags for modified components

---

## Design Decisions

### Storage Strategy
- **Archetype-based**: Entities with the same components share storage
- **Type-erased**: Use `dyn Any` for flexible component storage
- **Contiguous**: Components stored in `Vec` for cache efficiency

### Migration Strategy
- **Lazy**: Migrate entities only when their component composition changes
- **Copy-on-migrate**: Copy components to new archetype during migration
- **Cleanup**: Remove empty archetypes automatically

### Query Strategy
- **Archetype-first**: Filter by archetype keys before accessing components
- **Type-safe**: Provide both dynamic and typed query interfaces
- **Iterator-based**: Use Rust iterators for ergonomic access

---

## Dependencies

### Internal
- `std::any` for type erasure
- `std::collections` for HashMap, HashSet, Vec
- `std::fmt` for debugging

### External (Optional, for Phase 5+)
- `serde` for serialization (Phase 5.4)
- `rayon` for parallel iteration (future optimization)

---

## Success Criteria

### Phase 1-2 (Core)
- [ ] All unit tests pass
- [ ] Basic game simulation runs correctly
- [ ] Performance meets expectations (cache-friendly access patterns)

### Phase 3 (Testing)
- [ ] 100% test coverage for public API
- [ ] No memory leaks or safety violations
- [ ] Performance benchmarks established

### Phase 4 (Migration)
- [ ] All existing tests still pass
- [ ] New storage system provides same or better performance
- [ ] API remains stable (no breaking changes)

### Phase 5 (Advanced)
- [ ] Each feature has its own test suite
- [ ] Performance impact of each feature is measured
- [ ] Documentation updated for new features

---

## File Checklist

- [ ] `src/ecs/mod.rs` - Module exports
- [ ] `src/ecs/entity.rs` - Entity definitions
- [ ] `src/ecs/component.rs` - Component trait and storage
- [ ] `src/ecs/archetype.rs` - Archetype implementation
- [ ] `src/ecs/world.rs` - World management
- [ ] `src/ecs/query.rs` - Query system
- [ ] `src/ecs/tests.rs` or `tests/ecs.rs` - Unit tests
- [ ] `benches/ecs.rs` - Performance benchmarks (optional)

---

## Notes

1. **Start Simple**: Implement Phase 1 completely before moving to Phase 2
2. **Test Incrementally**: Add tests for each feature as it's implemented
3. **Profile Early**: Use benchmarks to guide optimization decisions
4. **Document Assumptions**: Note any assumptions about usage patterns
5. **Consider Alternatives**: Evaluate existing crates (`hecs`, `legion`) for inspiration

---

## See Also

- [ECS Architecture Guide](../architecture/ecs.md) (if created)
- [Rust ECS Crate Comparison](https://github.com/rust-gamedev/ecs_bench_suite)
- [Legion ECS](https://github.com/amethyst/legion) - Reference implementation
- [Hecs ECS](https://github.com/Ralith/hecs) - High-performance implementation