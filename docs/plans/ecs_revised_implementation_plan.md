# ECS Revised Implementation Plan

## Status: Phase 1 Foundation - Implementation Ready

## Architecture Decision

### Problem
Pure archetype-based ECS with entity migration is not feasible in safe Rust due to type system limitations:
- Cannot move components of unknown types between archetypes
- Cannot downcast from TypeId to concrete types without compile-time knowledge
- Cannot clone type-erased components generically

### Solution
**Implement Option A: No-Migration Archetype-Based ECS**

- Entities are created in archetypes with fixed component compositions
- Entities never change archetypes after creation
- Component access is cache-efficient via contiguous Vec<Option<T>> storage
- Query iteration is cache-efficient via archetype grouping

## Implementation Instructions

### Phase 1: Foundation

#### Step 1: Entity System (`src/ecs/entity.rs`)
```rust
// REQUIRED: Implement exactly as follows

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct EntityId(u32);

impl EntityId {
    pub const NULL: Self = Self(0);
    pub const MAX: Self = Self(u32::MAX);
    pub fn new(id: u32) -> Self { Self(id) }
    pub fn as_u32(self) -> u32 { self.0 }
    pub fn is_valid(self) -> bool { self.0 != 0 }
}

impl Default for EntityId { fn default() -> Self { Self::NULL } }
impl From<u32> for EntityId { fn from(id: u32) -> Self { Self(id) } }

pub struct EntityManager {
    next_id: u32,
    free_list: VecDeque<u32>,
}

impl EntityManager {
    pub fn new() -> Self { Self { next_id: 1, free_list: VecDeque::new() } }
    pub fn allocate(&mut self) -> EntityId { /* implement */ }
    pub fn free(&mut self, id: EntityId) { /* implement */ }
    pub fn clear(&mut self) { /* implement */ }
}
```

#### Step 2: Component System (`src/ecs/component.rs`)
```rust
// REQUIRED: Implement exactly as follows

pub trait Component: 'static + Send + Sync + Clone {}
impl<T: 'static + Send + Sync + Clone> Component for T {}

pub struct ComponentVec<T: Component> {
    pub components: Vec<Option<T>>,
}

impl<T: Component> ComponentVec<T> {
    pub fn new() -> Self { Self { components: Vec::new() } }
    pub fn push(&mut self, component: T) -> usize { /* implement */ }
    pub fn take(&mut self, index: usize) -> Option<T> { /* implement */ }
    pub fn get(&self, index: usize) -> Option<&T> { /* implement */ }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> { /* implement */ }
    pub fn contains(&self, index: usize) -> bool { /* implement */ }
    pub fn len(&self) -> usize { self.components.len() }
    pub fn is_empty(&self) -> bool { self.components.is_empty() }
    pub fn clear(&mut self) { self.components.clear() }
}
```

#### Step 3: Archetype System (`src/ecs/archetype.rs`)
```rust
// REQUIRED: Implement exactly as follows

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArchetypeKey {
    component_types: Vec<TypeId>,
}

impl ArchetypeKey {
    pub fn new(component_types: Vec<TypeId>) -> Self { /* sort and return */ }
    pub fn from_set(component_types: &HashSet<TypeId>) -> Self { /* implement */ }
    pub fn component_types(&self) -> &[TypeId] { &self.component_types }
    pub fn contains(&self, type_id: TypeId) -> bool { /* binary search */ }
    pub fn contains_all(&self, type_ids: &[TypeId]) -> bool { /* implement */ }
}

pub struct Archetype {
    entities: Vec<EntityId>,
    entity_to_index: HashMap<EntityId, usize>,
    component_types: HashSet<TypeId>,
    component_storages: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Archetype {
    pub fn new(component_types: HashSet<TypeId>) -> Self { /* implement */ }
    pub fn add_component_storage<T: Component>(&mut self) { /* implement */ }
    pub fn has_component_type(&self, type_id: TypeId) -> bool { /* implement */ }
    pub fn has_component<T: Component>(&self) -> bool { /* implement */ }
    pub fn entity_count(&self) -> usize { self.entities.len() }
    pub fn is_empty(&self) -> bool { self.entities.is_empty() }
    pub fn component_types(&self) -> &HashSet<TypeId> { &self.component_types }
    pub fn add_entity(&mut self, entity: EntityId) -> usize { /* implement */ }
    pub fn remove_entity(&mut self, index: usize) -> EntityId { /* implement with swap_remove */ }
    pub fn entity_index(&self, entity: EntityId) -> Option<usize> { /* implement */ }
    pub fn get_component_storage<T: Component>(&self) -> Option<&ComponentVec<T>> { /* downcast */ }
    pub fn get_component_storage_mut<T: Component>(&mut self, type_id: TypeId) -> Option<&mut ComponentVec<T>> { /* downcast */ }
    pub fn get_component<T: Component>(&self, entity_index: usize) -> Option<&T> { /* implement */ }
    pub fn get_component_mut<T: Component>(&mut self, entity_index: usize) -> Option<&mut T> { /* implement */ }
    pub fn add_component<T: Component>(&mut self, entity_index: usize, component: T) { /* implement */ }
    pub fn remove_component<T: Component>(&mut self, entity_index: usize) -> Option<T> { /* implement */ }
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + '_ { self.entities.iter().copied() }
}
```

#### Step 4: World System (`src/ecs/world.rs`)
```rust
// REQUIRED: Implement exactly as follows

pub struct World {
    entity_manager: EntityManager,
    entities: HashMap<EntityId, ArchetypeKey>,
    archetypes: HashMap<ArchetypeKey, Archetype>,
}

impl World {
    pub fn new() -> Self { /* implement */ }
    pub fn spawn(&mut self) -> EntityId { /* create in empty archetype */ }
    pub fn spawn_in_archetype(&mut self, component_types: &HashSet<TypeId>) -> EntityId { /* implement */ }
    pub fn despawn(&mut self, entity: EntityId) -> bool { /* implement */ }
    pub fn contains(&self, entity: EntityId) -> bool { self.entities.contains_key(&entity) }
    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> bool {
        /* Only works if entity's archetype already supports T */
    }
    pub fn remove_component<T: Component>(&mut self, entity: EntityId) -> Option<T> { /* implement */ }
    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> { /* implement */ }
    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> { /* implement */ }
    pub fn has_component<T: Component>(&self, entity: EntityId) -> bool { /* implement */ }
    pub fn entity_count(&self) -> usize { self.entities.len() }
    pub fn archetype_count(&self) -> usize { self.archetypes.len() }
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + '_ { self.entities.keys().copied() }
    pub fn iter_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)> + '_ { /* implement */ }
    pub fn iter_archetype(&self, key: &ArchetypeKey) -> impl Iterator<Item = EntityId> + '_ { /* implement */ }
    pub fn clear(&mut self) { /* implement */ }
}
```

#### Step 5: Module Exports (`src/ecs/mod.rs`)
```rust
pub mod entity;
pub mod component;
pub mod archetype;
pub mod world;

pub use entity::EntityId;
pub use component::Component;
pub use archetype::ArchetypeKey;
pub use world::World;
```

### Phase 2: Query System

#### Step 1: Query Module (`src/ecs/query.rs`)
```rust
pub trait WorldQueryExt {
    fn query_with<T: Component>(&self) -> QueryIter<T>;
    fn query_with2<T1: Component, T2: Component>(&self) -> QueryIter2<T1, T2>;
    fn query_mut<T: Component, F>(&mut self, f: F) where F: FnMut(EntityId, &mut T);
}

// Implement QueryIter, QueryIter2, etc.
```

#### Step 2: Update World with Query Extensions
```rust
impl WorldQueryExt for World { /* implement */ }
```

### Phase 3: Testing

#### Step 1: Unit Tests (`src/ecs/tests.rs`)
```rust
// Test each module:
// - entity: allocation, freeing, reuse
// - component: storage, access, cloning
// - archetype: key creation, entity management
// - world: spawn, despawn, add/remove/get components, iteration
// - query: filtering, iteration, mutable access
```

#### Step 2: Integration Tests
```rust
// Test realistic scenarios:
// - Game with multiple entity types
// - Component access patterns
// - Query performance
```

### File Structure
```
src/ecs/
├── mod.rs          # Public API exports
├── entity.rs       # EntityId and EntityManager
├── component.rs    # Component trait and ComponentVec
├── archetype.rs    # ArchetypeKey and Archetype
├── world.rs        # World implementation
├── query.rs        # Query system (Phase 2)
└── tests.rs        # Unit tests

benches/
└── ecs.rs          # Performance benchmarks (Phase 3)
```

### Key Design Principles

1. **No Entity Migration**: Entities stay in their initial archetype
2. **Contiguous Storage**: Components stored in Vec<Option<T>> for cache efficiency
3. **Type Safety**: All component access is type-checked at compile time
4. **Simple API**: Minimal methods with clear semantics

### Usage Example
```rust
use renderlib::ecs::{World, Component};

#[derive(Clone)]
struct Position { x: f32, y: f32 }

#[derive(Clone)]
struct Velocity { dx: f32, dy: f32 }

let mut world = World::new();

// Create entity in Position+Velocity archetype
let mut component_types = HashSet::new();
component_types.insert(TypeId::of::<Position>());
component_types.insert(TypeId::of::<Velocity>());
let entity = world.spawn_in_archetype(&component_types);

// Add components
world.add_component(entity, Position { x: 1.0, y: 2.0 });
world.add_component(entity, Velocity { dx: 0.5, dy: 0.5 });

// Access components
let pos = world.get_component::<Position>(entity).unwrap();
let vel = world.get_component::<Velocity>(entity).unwrap();

// Query
for (entity, pos) in world.query_with::<Position>() {
    println!("Entity {:?} at ({}, {})", entity, pos.x, pos.y);
}
```

### Success Criteria

#### Phase 1
- [ ] All core types compile
- [ ] All unit tests pass
- [ ] Cache efficiency verified (contiguous memory access)

#### Phase 2
- [ ] Query system compiles
- [ ] Query tests pass
- [ ] Query performance meets targets

#### Phase 3
- [ ] Integration tests pass
- [ ] Performance benchmarks established
- [ ] Documentation complete

### Performance Targets

| Operation | Target Time | Notes |
|-----------|-------------|-------|
| spawn | < 100ns | O(1) allocation |
| despawn | < 200ns | O(1) removal |
| add_component | < 100ns | Direct Vec access |
| get_component | < 50ns | Direct Vec access |
| query (10k entities) | < 1ms | Archetype filtering |

### Notes

1. **No Migration**: Entities cannot change component composition after creation
2. **Workaround**: Despawn and respawn entities when component composition changes
3. **Clone Requirement**: All components must implement Clone for future migration support
4. **Cache Efficiency**: Component access is O(1) with perfect cache locality within archetypes
