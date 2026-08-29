# ECS Detailed Implementation Specification: Phases 1-3

**Document Type**: Technical Implementation Specification  
**Target**: `src/ecs/` module  
**Phases Covered**: 1 (Foundation), 2 (Query System), 3 (Testing & Validation)  
**Status**: Pre-Implementation  

---

## Table of Contents

1. [Rust-Specific Design Considerations](#1-rust-specific-design-considerations)
2. [Phase 1: Foundation - Detailed Implementation](#2-phase-1-foundation---detailed-implementation)
3. [Phase 2: Query System - Detailed Implementation](#3-phase-2-query-system---detailed-implementation)
4. [Phase 3: Testing & Validation - Detailed Implementation](#4-phase-3-testing--validation---detailed-implementation)
5. [File Structure & Module Organization](#5-file-structure--module-organization)
6. [Error Handling Strategy](#6-error-handling-strategy)
7. [Performance Considerations](#7-performance-considerations)

---

## 1. Rust-Specific Design Considerations

### 1.1 Entity Migration Between Archetypes

**Problem**: When an entity's component composition changes, it must move from its current archetype to a new one matching the new component set. This migration must be efficient and avoid unnecessary copying.

**Solution: Move Semantics with `Option::take()`**

```rust
// Core migration algorithm in World
fn migrate_entity(&mut self, entity: EntityId, target_components: &HashSet<TypeId>) {
    let source_archetype_key = self.entity_to_archetype[&entity].clone();
    let target_archetype_key = ArchetypeKey::from_type_ids(target_components);
    
    // Skip if no migration needed
    if source_archetype_key == target_archetype_key {
        return;
    }
    
    // Get references to source and target archetypes
    let source_archetype = self.archetypes.get_mut(&source_archetype_key)
        .expect("Source archetype must exist");
    
    let target_archetype = self.archetypes.entry(target_archetype_key.clone())
        .or_insert_with(|| Archetype::new(target_components.clone()));
    
    // Find entity's position in source archetype
    let source_index = source_archetype.entity_to_index[&entity];
    
    // MIGRATION: Move components using Option::take() - NO COPYING
    for (type_id, storage) in &mut source_archetype.component_storages {
        if target_components.contains(type_id) {
            // Component exists in both archetypes - MOVE it
            let component = storage.components[source_index].take();
            if let Some(comp) = component {
                target_archetype.add_component(*type_id, comp);
            }
        }
        // If component doesn't exist in target, it's dropped (entity loses it)
    }
    
    // Add entity to target archetype
    let target_index = target_archetype.entities.len();
    target_archetype.entities.push(entity);
    target_archetype.entity_to_index.insert(entity, target_index);
    
    // Remove entity from source archetype using swap_remove for O(1)
    source_archetype.entities.swap_remove(source_index);
    
    // Update the index of the entity that was swapped into source_index's position
    if source_index < source_archetype.entities.len() {
        let swapped_entity = source_archetype.entities[source_index];
        source_archetype.entity_to_index.insert(swapped_entity, source_index);
    }
    
    source_archetype.entity_to_index.remove(&entity);
    
    // Clean up source archetype if empty
    if source_archetype.entities.is_empty() {
        self.archetypes.remove(&source_archetype_key);
    }
    
    // Update global mapping
    self.entity_to_archetype.insert(entity, target_archetype_key);
}
```

**Key Design Decisions**:
- **`Vec<Option<T>>`**: Enables `take()` to move components out without copying
- **swap_remove**: O(1) removal from entity lists
- **Index tracking**: Each archetype maintains `entity_to_index` HashMap for O(1) lookup
- **Lazy cleanup**: Empty archetypes are removed immediately

**Complexity Analysis**:
- Time: O(C) where C is number of component types in source archetype
- Space: O(1) additional space (in-place moves)
- Cache: Good locality during migration (sequential access to component vectors)

---

### 1.2 Preventing Temporary Copies of Components

**Problem**: Component data can be large; we must avoid unnecessary copies during add/remove/migration operations.

**Solution A: `Vec<Option<T>>` Storage Pattern**

```rust
/// Storage for a single component type within an archetype
struct ComponentVec<T: Component> {
    components: Vec<Option<T>>,
}

impl<T: Component> ComponentVec<T> {
    /// Add component by moving (zero-copy)
    pub fn push(&mut self, component: T) -> usize {
        let index = self.components.len();
        self.components.push(Some(component));
        index
    }
    
    /// Remove component by taking (zero-copy, drops old value)
    pub fn take(&mut self, index: usize) -> Option<T> {
        self.components[index].take()
    }
    
    /// Get immutable reference (zero-copy)
    pub fn get(&self, index: usize) -> Option<&T> {
        self.components[index].as_ref()
    }
    
    /// Get mutable reference (zero-copy)
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.components[index].as_mut()
    }
    
    /// Check if component exists at index
    pub fn contains(&self, index: usize) -> bool {
        self.components[index].is_some()
    }
    
    /// Length of storage
    pub fn len(&self) -> usize {
        self.components.len()
    }
    
    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}
```

**Solution B: Type-Erased Storage with Downcasting**

```rust
/// Trait for type-erased component storage
trait AnyComponentStorage: DowncastStorage {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn contains(&self, index: usize) -> bool;
}

/// Blanket implementation for all ComponentVec<T>
impl<T: Component> AnyComponentStorage for ComponentVec<T> {
    fn len(&self) -> usize { self.len() }
    fn is_empty(&self) -> bool { self.is_empty() }
    fn contains(&self, index: usize) -> bool { self.contains(index) }
}

/// Trait for safe downcasting
trait DowncastStorage {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: Component> DowncastStorage for ComponentVec<T> {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Type-erased storage wrapper
struct ErasedComponentStorage {
    storage: Box<dyn AnyComponentStorage>,
    type_id: TypeId,
}

impl ErasedComponentStorage {
    /// Downcast to concrete type
    pub fn downcast<T: Component>(&self) -> Option<&ComponentVec<T>> {
        if self.type_id == TypeId::of::<T>() {
            self.storage.as_any().downcast_ref::<ComponentVec<T>>()
        } else {
            None
        }
    }
    
    /// Downcast mutable
    pub fn downcast_mut<T: Component>(&mut self) -> Option<&mut ComponentVec<T>> {
        if self.type_id == TypeId::of::<T>() {
            self.storage.as_any_mut().downcast_mut::<ComponentVec<T>>()
        } else {
            None
        }
    }
}
```

**Solution C: Direct Access API (Avoiding Downcast Overhead)**

```rust
impl Archetype {
    /// Direct access without downcasting - for known types at compile time
    pub fn get_component_direct<T: Component>(&self, index: usize) -> Option<&T> {
        let storage = self.component_storages.get(&TypeId::of::<T>())?;
        let storage = storage.storage.as_any().downcast_ref::<ComponentVec<T>>()?;
        storage.get(index)
    }
    
    /// Mutable direct access
    pub fn get_component_mut_direct<T: Component>(&mut self, index: usize) -> Option<&mut T> {
        let storage = self.component_storages.get_mut(&TypeId::of::<T>())?;
        let storage = storage.storage.as_any_mut().downcast_mut::<ComponentVec<T>>()?;
        storage.get_mut(index)
    }
}
```

**Performance Comparison**:
| Method | Time Complexity | Allocations | Best Use Case |
|--------|----------------|-------------|---------------|
| Downcast + Access | O(1) | 0 | Generic code, unknown types |
| Direct Access | O(1) | 0 | Known types, performance-critical |
| Batch Processing | O(N) | 0 | Processing many entities |

---

### 1.3 Lifetime Issues When Iterating Over Entities

**Problem**: Rust's borrow checker makes it challenging to iterate over entities while maintaining safe access to components, especially for mutable access.

**Solution A: Immutable Iterators (Simple Case)**

```rust
impl<'a> World {
    /// Simple entity iterator - borrow checker friendly
    pub fn iter_entities(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        self.archetypes.values()
            .flat_map(|archetype| archetype.entities.iter().copied())
    }
    
    /// Iterator over entities with a specific component
    pub fn iter_with<T: Component>(&'a self) -> impl Iterator<Item = (EntityId, &'a T)> + 'a {
        self.archetypes.values()
            .filter(|a| a.has_component::<T>())
            .flat_map(move |archetype| {
                let storage = archetype.get_component_storage::<T>();
                archetype.entities.iter().enumerate()
                    .filter_map(move |(idx, &entity)| {
                        storage.get(idx).map(|comp| (entity, comp))
                    })
            })
    }
}
```

**Solution B: Callback-Based Mutable Iteration**

```rust
impl World {
    /// Process entities with mutable access to a single component
    pub fn for_each_mut<T: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(EntityId, &mut T),
    {
        let type_id = TypeId::of::<T>();
        
        for archetype in self.archetypes.values_mut() {
            if !archetype.has_component_type(type_id) {
                continue;
            }
            
            // Safe: we only borrow one component storage at a time
            let storage = archetype.get_component_storage_mut::<T>(type_id);
            
            for (idx, &entity) in archetype.entities.iter().enumerate() {
                if let Some(component) = storage.get_mut(idx) {
                    f(entity, component);
                }
            }
        }
    }
    
    /// Process entities with mutable access to multiple components
    pub fn for_each_mut2<T1: Component, T2: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(EntityId, &mut T1, &mut T2),
    {
        let type_id_1 = TypeId::of::<T1>();
        let type_id_2 = TypeId::of::<T2>();
        
        for archetype in self.archetypes.values_mut() {
            if !archetype.has_component_types(&[type_id_1, type_id_2]) {
                continue;
            }
            
            // Safe: we only access one archetype at a time
            let storage_1 = archetype.get_component_storage_mut::<T1>(type_id_1);
            let storage_2 = archetype.get_component_storage_mut::<T2>(type_id_2);
            
            for (idx, &entity) in archetype.entities.iter().enumerate() {
                if let (Some(c1), Some(c2)) = (storage_1.get_mut(idx), storage_2.get_mut(idx)) {
                    f(entity, c1, c2);
                }
            }
        }
    }
}
```

**Solution C: Index-Based Access Pattern**

```rust
/// For systems that need to mutate entities after iteration
impl World {
    /// Get component by entity ID (mutable)
    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let archetype_key = self.entity_to_archetype.get(&entity)?;
        let archetype = self.archetypes.get_mut(archetype_key)?;
        let index = archetype.entity_to_index.get(&entity)?;
        archetype.get_component_mut_direct::<T>(*index)
    }
    
    /// Get multiple components by entity ID
    pub fn get_components_mut<T1: Component, T2: Component>(
        &mut self, 
        entity: EntityId
    ) -> Option<(&mut T1, &mut T2)> {
        let archetype_key = self.entity_to_archetype.get(&entity)?;
        let archetype = self.archetypes.get_mut(archetype_key)?;
        let index = archetype.entity_to_index.get(&entity)?;
        
        let c1 = archetype.get_component_mut_direct::<T1>(*index)?;
        let c2 = archetype.get_component_mut_direct::<T2>(*index)?;
        
        // Safety: Both components are from the same archetype and entity index
        // This is safe because we're only borrowing from one entity at a time
        Some((c1, c2))
    }
}
```

**Solution D: Archetype-Aware Iteration**

```rust
/// Iterator that yields entire archetypes for batch processing
impl<'a> World {
    pub fn iter_archetypes(&'a self) -> impl Iterator<Item = &'a Archetype> + 'a {
        self.archetypes.values()
    }
    
    pub fn iter_archetypes_mut(&'a mut self) -> impl Iterator<Item = &'a mut Archetype> + 'a {
        self.archetypes.values_mut()
    }
}

/// Process all entities in an archetype with multiple components
impl Archetype {
    pub fn for_each_entity<T1: Component, T2: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(EntityId, &mut T1, &mut T2),
    {
        let type_id_1 = TypeId::of::<T1>();
        let type_id_2 = TypeId::of::<T2>();
        
        let storage_1 = self.get_component_storage_mut::<T1>(type_id_1);
        let storage_2 = self.get_component_storage_mut::<T2>(type_id_2);
        
        for (idx, &entity) in self.entities.iter().enumerate() {
            if let (Some(c1), Some(c2)) = (storage_1.get_mut(idx), storage_2.get_mut(idx)) {
                f(entity, c1, c2);
            }
        }
    }
}
```

**Lifetime Strategy Summary**:

| Access Pattern | Approach | Lifetime Complexity | Use Case |
|---------------|----------|-------------------|----------|
| Immutable iteration | `flat_map` + `filter_map` | Low | Read-only queries |
| Single mutable component | Callback with `&mut T` | Medium | Simple mutations |
| Multiple mutable components | Callback with `&mut T1, &mut T2` | Medium | Complex mutations |
| Index-based access | Return indices, then access | High | Flexible access patterns |
| Archetype batch processing | Process one archetype at a time | Low | Cache-friendly processing |

---

### 1.4 Additional Rust Considerations

#### A. Type Safety and Component Trait

```rust
/// Marker trait for ECS components
/// 
/// # Safety
/// Components must be:
/// - `'static`: No non-static references
/// - `Send + Sync`: Thread-safe (for potential parallel iteration)
/// - `Sized`: Fixed size (for storage in Vec)
/// 
/// Blanket implementation provided for all valid types
pub trait Component: 'static + Send + Sync {}

impl<T: 'static + Send + Sync> Component for T {}
```

#### B. Entity ID Design

```rust
/// Simple entity identifier
/// 
/// Uses u32 for compact storage. Generational IDs can be added later.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct EntityId(u32);

impl EntityId {
    /// Null entity (invalid)
    pub const NULL: Self = Self(0);
    
    /// Maximum entity ID
    pub const MAX: Self = Self(u32::MAX);
    
    /// Create a new entity ID
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    
    /// Get the raw ID value
    pub fn as_u32(self) -> u32 {
        self.0
    }
    
    /// Check if this is a valid entity
    pub fn is_valid(self) -> bool {
        self.0 != 0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::NULL
    }
}

impl From<u32> for EntityId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}
```

#### C. Archetype Key Design

```rust
/// Unique identifier for an archetype based on its component types
/// 
/// The key is a sorted vector of TypeIds to ensure consistent hashing
/// regardless of the order in which components were added.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArchetypeKey {
    component_types: Vec<TypeId>,
}

impl ArchetypeKey {
    /// Create a new archetype key from a set of component types
    pub fn new(component_types: Vec<TypeId>) -> Self {
        let mut types = component_types;
        types.sort();
        Self { component_types: types }
    }
    
    /// Create from a HashSet (convenience)
    pub fn from_set(component_types: &HashSet<TypeId>) -> Self {
        Self::new(component_types.iter().copied().collect())
    }
    
    /// Get the component types
    pub fn component_types(&self) -> &[TypeId] {
        &self.component_types
    }
    
    /// Check if this archetype contains a specific component type
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.component_types.binary_search(&type_id).is_ok()
    }
    
    /// Check if this archetype contains all of the given component types
    pub fn contains_all(&self, type_ids: &[TypeId]) -> bool {
        type_ids.iter().all(|&tid| self.contains(tid))
    }
}

impl From<Vec<TypeId>> for ArchetypeKey {
    fn from(types: Vec<TypeId>) -> Self {
        Self::new(types)
    }
}

impl From<&HashSet<TypeId>> for ArchetypeKey {
    fn from(set: &HashSet<TypeId>) -> Self {
        Self::from_set(set)
    }
}
```

#### D. Memory Layout Optimization

```rust
/// Archetype stores entities and their components
/// 
/// Memory layout:
/// - entities: Vec<EntityId> - list of entities in this archetype
/// - entity_to_index: HashMap<EntityId, usize> - O(1) entity -> index lookup
/// - component_storages: HashMap<TypeId, ErasedComponentStorage> - type-erased component data
/// 
/// All components for an entity are stored at the same index across all ComponentVec<T>
/// This ensures cache-friendly access when iterating over entities.
pub struct Archetype {
    entities: Vec<EntityId>,
    entity_to_index: HashMap<EntityId, usize>,
    component_types: HashSet<TypeId>,
    component_storages: HashMap<TypeId, ErasedComponentStorage>,
}

impl Archetype {
    /// Create a new empty archetype
    pub fn new(component_types: HashSet<TypeId>) -> Self {
        Self {
            entities: Vec::new(),
            entity_to_index: HashMap::new(),
            component_types,
            component_storages: HashMap::new(),
        }
    }
    
    /// Add a new component storage to this archetype
    pub fn add_component_storage<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.component_types.insert(type_id);
        self.component_storages.insert(type_id, ErasedComponentStorage::new::<T>());
    }
    
    /// Check if this archetype has a specific component type
    pub fn has_component_type(&self, type_id: TypeId) -> bool {
        self.component_types.contains(&type_id)
    }
    
    /// Check if this archetype has a specific component type (generic)
    pub fn has_component<T: Component>(&self) -> bool {
        self.has_component_type(TypeId::of::<T>())
    }
    
    /// Get component storage for a specific type
    pub fn get_component_storage<T: Component>(&self) -> Option<&ComponentVec<T>> {
        let type_id = TypeId::of::<T>();
        self.component_storages.get(&type_id)
            .and_then(|s| s.downcast::<T>())
    }
    
    /// Get mutable component storage for a specific type
    pub fn get_component_storage_mut<T: Component>(&mut self, type_id: TypeId) -> &mut ComponentVec<T> {
        self.component_storages.get_mut(&type_id)
            .and_then(|s| s.downcast_mut::<T>())
            .expect("Component storage must exist for requested type")
    }
}
```

---

## 2. Phase 1: Foundation - Detailed Implementation

### 2.1 Module Structure

```
src/ecs/
├── mod.rs              # Public API exports
├── entity.rs           # EntityId and EntityManager
├── component.rs        # Component trait and storage types
├── archetype.rs        # Archetype struct and operations
├── world.rs            # World struct and core operations
└── error.rs            # Error types (optional)
```

### 2.2 Entity System (`entity.rs`)

```rust
//! Entity management for the ECS

use std::collections::HashSet;

/// Entity identifier
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct EntityId(u32);

impl EntityId {
    pub const NULL: Self = Self(0);
    pub const MAX: Self = Self(u32::MAX);
    
    pub fn new(id: u32) -> Self { Self(id) }
    pub fn as_u32(self) -> u32 { self.0 }
    pub fn is_valid(self) -> bool { self.0 != 0 }
}

impl Default for EntityId {
    fn default() -> Self { Self::NULL }
}

impl From<u32> for EntityId {
    fn from(id: u32) -> Self { Self(id) }
}

/// Manages entity ID allocation and reuse
pub struct EntityManager {
    next_id: u32,
    free_list: Vec<u32>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            next_id: 1, // Start at 1 (0 is NULL)
            free_list: Vec::new(),
        }
    }
    
    /// Allocate a new entity ID
    pub fn allocate(&mut self) -> EntityId {
        if let Some(id) = self.free_list.pop() {
            EntityId(id)
        } else {
            let id = self.next_id;
            self.next_id += 1;
            EntityId(id)
        }
    }
    
    /// Free an entity ID for reuse
    pub fn free(&mut self, id: EntityId) {
        if id.0 < self.next_id {
            self.free_list.push(id.0);
        }
    }
    
    /// Reset the manager (for testing or world reset)
    pub fn clear(&mut self) {
        self.next_id = 1;
        self.free_list.clear();
    }
}

impl Default for EntityManager {
    fn default() -> Self { Self::new() }
}
```

### 2.3 Component System (`component.rs`)

```rust
//! Component definitions and storage

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Marker trait for ECS components
pub trait Component: 'static + Send + Sync {}

impl<T: 'static + Send + Sync> Component for T {}

/// Storage for a single component type
pub struct ComponentVec<T: Component> {
    pub(crate) components: Vec<Option<T>>,
}

impl<T: Component> ComponentVec<T> {
    pub fn new() -> Self {
        Self { components: Vec::new() }
    }
    
    pub fn push(&mut self, component: T) -> usize {
        let index = self.components.len();
        self.components.push(Some(component));
        index
    }
    
    pub fn take(&mut self, index: usize) -> Option<T> {
        self.components[index].take()
    }
    
    pub fn get(&self, index: usize) -> Option<&T> {
        self.components[index].as_ref()
    }
    
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.components[index].as_mut()
    }
    
    pub fn contains(&self, index: usize) -> bool {
        self.components[index].is_some()
    }
    
    pub fn len(&self) -> usize {
        self.components.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
    
    pub fn clear(&mut self) {
        self.components.clear();
    }
}

impl<T: Component> Default for ComponentVec<T> {
    fn default() -> Self { Self::new() }
}

/// Trait for downcasting component storage
trait DowncastStorage {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> DowncastStorage for ComponentVec<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Trait for type-erased component storage operations
trait AnyComponentStorage: DowncastStorage {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn contains(&self, index: usize) -> bool;
}

impl<T: Component> AnyComponentStorage for ComponentVec<T> {
    fn len(&self) -> usize { self.len() }
    fn is_empty(&self) -> bool { self.is_empty() }
    fn contains(&self, index: usize) -> bool { self.contains(index) }
}

/// Type-erased component storage wrapper
pub struct ErasedComponentStorage {
    storage: Box<dyn AnyComponentStorage>,
    type_id: TypeId,
}

impl ErasedComponentStorage {
    pub fn new<T: Component>() -> Self {
        Self {
            storage: Box::new(ComponentVec::<T>::new()),
            type_id: TypeId::of::<T>(),
        }
    }
    
    pub fn downcast<T: Component>(&self) -> Option<&ComponentVec<T>> {
        if self.type_id == TypeId::of::<T>() {
            self.storage.as_any().downcast_ref::<ComponentVec<T>>()
        } else {
            None
        }
    }
    
    pub fn downcast_mut<T: Component>(&mut self) -> Option<&mut ComponentVec<T>> {
        if self.type_id == TypeId::of::<T>() {
            self.storage.as_any_mut().downcast_mut::<ComponentVec<T>>()
        } else {
            None
        }
    }
    
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}
```

### 2.4 Archetype System (`archetype.rs`)

```rust
//! Archetype-based entity grouping

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use super::{component::{Component, ComponentVec, ErasedComponentStorage}, entity::EntityId};

/// Unique identifier for an archetype
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArchetypeKey {
    component_types: Vec<TypeId>,
}

impl ArchetypeKey {
    pub fn new(component_types: Vec<TypeId>) -> Self {
        let mut types = component_types;
        types.sort();
        Self { component_types: types }
    }
    
    pub fn from_set(component_types: &HashSet<TypeId>) -> Self {
        Self::new(component_types.iter().copied().collect())
    }
    
    pub fn component_types(&self) -> &[TypeId] {
        &self.component_types
    }
    
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.component_types.binary_search(&type_id).is_ok()
    }
    
    pub fn contains_all(&self, type_ids: &[TypeId]) -> bool {
        type_ids.iter().all(|&tid| self.contains(tid))
    }
}

impl From<Vec<TypeId>> for ArchetypeKey {
    fn from(types: Vec<TypeId>) -> Self { Self::new(types) }
}

impl From<&HashSet<TypeId>> for ArchetypeKey {
    fn from(set: &HashSet<TypeId>) -> Self { Self::from_set(set) }
}

/// An archetype groups entities with the same component composition
pub struct Archetype {
    entities: Vec<EntityId>,
    entity_to_index: HashMap<EntityId, usize>,
    component_types: HashSet<TypeId>,
    component_storages: HashMap<TypeId, ErasedComponentStorage>,
}

impl Archetype {
    /// Create a new empty archetype with the given component types
    pub fn new(component_types: HashSet<TypeId>) -> Self {
        Self {
            entities: Vec::new(),
            entity_to_index: HashMap::new(),
            component_types,
            component_storages: HashMap::new(),
        }
    }
    
    /// Add component storage for a type
    pub fn add_component_storage<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.component_types.insert(type_id);
        self.component_storages.insert(type_id, ErasedComponentStorage::new::<T>());
    }
    
    /// Check if this archetype has a specific component type
    pub fn has_component_type(&self, type_id: TypeId) -> bool {
        self.component_types.contains(&type_id)
    }
    
    /// Check if this archetype has a specific component type (generic)
    pub fn has_component<T: Component>(&self) -> bool {
        self.has_component_type(TypeId::of::<T>())
    }
    
    /// Check if this archetype has all of the given component types
    pub fn has_components(&self, type_ids: &[TypeId]) -> bool {
        type_ids.iter().all(|&tid| self.has_component_type(tid))
    }
    
    /// Get the number of entities in this archetype
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Check if this archetype is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
    
    /// Get the component types in this archetype
    pub fn component_types(&self) -> &HashSet<TypeId> {
        &self.component_types
    }
    
    /// Add an entity to this archetype
    /// 
    /// Returns the index at which the entity was added
    pub fn add_entity(&mut self, entity: EntityId) -> usize {
        let index = self.entities.len();
        self.entities.push(entity);
        self.entity_to_index.insert(entity, index);
        index
    }
    
    /// Remove an entity from this archetype by index
    /// 
    /// Uses swap_remove for O(1) removal
    pub fn remove_entity(&mut self, index: usize) -> EntityId {
        let entity = self.entities[index];
        
        // Swap with last element
        let last_index = self.entities.len() - 1;
        if index != last_index {
            self.entities.swap(index, last_index);
            // Update the index of the entity that was moved
            let moved_entity = self.entities[index];
            self.entity_to_index.insert(moved_entity, index);
        }
        
        // Remove the entity
        self.entities.pop();
        self.entity_to_index.remove(&entity);
        
        entity
    }
    
    /// Get the index of an entity in this archetype
    pub fn entity_index(&self, entity: EntityId) -> Option<usize> {
        self.entity_to_index.get(&entity).copied()
    }
    
    /// Get component storage for a specific type (immutable)
    pub fn get_component_storage<T: Component>(&self) -> Option<&ComponentVec<T>> {
        let type_id = TypeId::of::<T>();
        self.component_storages.get(&type_id)
            .and_then(|s| s.downcast::<T>())
    }
    
    /// Get component storage for a specific type (mutable)
    pub fn get_component_storage_mut<T: Component>(&mut self, type_id: TypeId) -> Option<&mut ComponentVec<T>> {
        self.component_storages.get_mut(&type_id)
            .and_then(|s| s.downcast_mut::<T>())
    }
    
    /// Get a component for an entity (immutable)
    pub fn get_component<T: Component>(&self, entity_index: usize) -> Option<&T> {
        self.get_component_storage::<T>()?.get(entity_index)
    }
    
    /// Get a component for an entity (mutable)
    pub fn get_component_mut<T: Component>(&mut self, entity_index: usize) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.get_component_storage_mut::<T>(type_id)?.get_mut(entity_index)
    }
    
    /// Add a component to an entity in this archetype
    /// 
    /// Used during entity migration
    pub fn add_component<T: Component>(&mut self, entity_index: usize, component: T) {
        let type_id = TypeId::of::<T>();
        
        // Ensure storage exists
        if !self.component_storages.contains_key(&type_id) {
            self.add_component_storage::<T>();
        }
        
        let storage = self.get_component_storage_mut::<T>(type_id).unwrap();
        
        // Resize storage if needed
        if storage.len() <= entity_index {
            storage.components.resize(entity_index + 1, None);
        }
        
        storage.components[entity_index] = Some(component);
    }
    
    /// Remove a component from an entity in this archetype
    /// 
    /// Returns the removed component if it existed
    pub fn remove_component<T: Component>(&mut self, entity_index: usize) -> Option<T> {
        self.get_component_storage_mut::<T>(TypeId::of::<T>())?
            .take(entity_index)
    }
    
    /// Iterate over all entities in this archetype
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().copied()
    }
}
```

### 2.5 World System (`world.rs`)

```rust
//! World management - the central ECS container

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use super::{
    archetype::{Archetype, ArchetypeKey},
    component::Component,
    entity::{EntityId, EntityManager},
};

/// The central ECS container
/// 
/// Manages all entities and their components through archetypes.
pub struct World {
    entity_manager: EntityManager,
    entities: HashMap<EntityId, ArchetypeKey>,
    archetypes: HashMap<ArchetypeKey, Archetype>,
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            entities: HashMap::new(),
            archetypes: HashMap::new(),
        }
    }
    
    /// Spawn a new entity with no components
    pub fn spawn(&mut self) -> EntityId {
        let entity = self.entity_manager.allocate();
        
        // Add to empty archetype
        let empty_key = ArchetypeKey::new(Vec::new());
        let archetype = self.archetypes.entry(empty_key.clone())
            .or_insert_with(|| Archetype::new(HashSet::new()));
        
        let index = archetype.add_entity(entity);
        
        // Add component storages for all types that might be added later
        // (This is optimized in practice - we add storages on demand)
        
        self.entities.insert(entity, empty_key);
        
        entity
    }
    
    /// Despawn an entity and all its components
    pub fn despawn(&mut self, entity: EntityId) -> bool {
        if let Some(archetype_key) = self.entities.get(&entity) {
            let archetype = self.archetypes.get_mut(archetype_key).unwrap();
            
            if let Some(index) = archetype.entity_index(entity) {
                // Remove entity from archetype
                archetype.remove_entity(index);
                
                // Clean up archetype if empty
                if archetype.is_empty() {
                    self.archetypes.remove(archetype_key);
                }
                
                // Free the entity ID
                self.entity_manager.free(entity);
                self.entities.remove(&entity);
                
                return true;
            }
        }
        false
    }
    
    /// Check if an entity exists
    pub fn contains(&self, entity: EntityId) -> bool {
        self.entities.contains_key(&entity)
    }
    
    /// Add a component to an entity
    /// 
    /// This may trigger entity migration if the entity's archetype doesn't support the component
    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> bool {
        if !self.contains(entity) {
            return false;
        }
        
        let old_archetype_key = self.entities[&entity].clone();
        let old_archetype = self.archetypes.get_mut(&old_archetype_key).unwrap();
        
        let type_id = TypeId::of::<T>();
        
        // Check if the current archetype already has this component type
        if old_archetype.has_component_type(type_id) {
            // Entity is already in an archetype that supports this component
            let index = old_archetype.entity_index(entity).unwrap();
            old_archetype.add_component::<T>(index, component);
            return true;
        }
        
        // Need to migrate to a new archetype
        let mut new_component_types = old_archetype.component_types().clone();
        new_component_types.insert(type_id);
        
        let new_archetype_key = ArchetypeKey::from_set(&new_component_types);
        
        // Create new archetype if it doesn't exist
        let new_archetype = self.archetypes.entry(new_archetype_key.clone())
            .or_insert_with(|| {
                let mut archetype = Archetype::new(new_component_types.clone());
                // Add all component storages from old archetype + new type
                for &tid in old_archetype.component_types() {
                    // In practice, we'd need to create appropriate storages
                    // This is simplified - actual implementation would handle this
                }
                archetype.add_component_storage::<T>();
                archetype
            });
        
        // Get entity index in old archetype
        let old_index = old_archetype.entity_index(entity).unwrap();
        
        // Add entity to new archetype
        let new_index = new_archetype.add_entity(entity);
        
        // Move existing components from old to new archetype
        for &component_type in old_archetype.component_types() {
            if let Some(component) = old_archetype.remove_component::<T>(old_index) {
                // This is a simplification - actual implementation would handle each component type
                // using a more generic approach
            }
        }
        
        // Add the new component
        new_archetype.add_component::<T>(new_index, component);
        
        // Remove entity from old archetype
        old_archetype.remove_entity(old_index);
        
        // Clean up old archetype if empty
        if old_archetype.is_empty() {
            self.archetypes.remove(&old_archetype_key);
        }
        
        // Update entity's archetype
        self.entities.insert(entity, new_archetype_key);
        
        true
    }
    
    /// Remove a component from an entity
    pub fn remove_component<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        if !self.contains(entity) {
            return None;
        }
        
        let archetype_key = self.entities[&entity].clone();
        let archetype = self.archetypes.get_mut(&archetype_key)?;
        let index = archetype.entity_index(entity)?;
        
        archetype.remove_component::<T>(index)
    }
    
    /// Get a component from an entity (immutable)
    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        let archetype_key = self.entities.get(&entity)?;
        let archetype = self.archetypes.get(archetype_key)?;
        let index = archetype.entity_index(entity)?;
        archetype.get_component::<T>(index)
    }
    
    /// Get a component from an entity (mutable)
    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let archetype_key = self.entities.get(&entity)?;
        let archetype = self.archetypes.get_mut(archetype_key)?;
        let index = archetype.entity_index(entity)?;
        archetype.get_component_mut::<T>(index)
    }
    
    /// Check if an entity has a component
    pub fn has_component<T: Component>(&self, entity: EntityId) -> bool {
        self.get_component::<T>(entity).is_some()
    }
    
    /// Get the number of entities in the world
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Get the number of archetypes
    pub fn archetype_count(&self) -> usize {
        self.archetypes.len()
    }
    
    /// Iterate over all entities
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }
    
    /// Clear all entities and archetypes
    pub fn clear(&mut self) {
        self.entity_manager.clear();
        self.entities.clear();
        self.archetypes.clear();
    }
}

impl Default for World {
    fn default() -> Self { Self::new() }
}
```

### 2.6 Module Exports (`mod.rs`)

```rust
//! Entity-Component-System implementation
//! 
//! A modern ECS with archetype-based storage for cache efficiency.

pub mod entity;
pub mod component;
pub mod archetype;
pub mod world;

pub use entity::EntityId;
pub use component::Component;
pub use archetype::ArchetypeKey;
pub use world::World;
```

---

## 3. Phase 2: Query System - Detailed Implementation

### 3.1 Query Module Structure

```
src/ecs/
├── query.rs            # Query system implementation
```

### 3.2 Query System Implementation (`query.rs`)

```rust
//! Query system for filtering and iterating over entities

use std::any::TypeId;
use std::collections::HashSet;
use std::marker::PhantomData;

use super::{archetype::ArchetypeKey, component::Component, entity::EntityId, world::World};

/// A query filter specifying which components entities must have
pub struct QueryFilter {
    required_components: HashSet<TypeId>,
}

impl QueryFilter {
    pub fn new() -> Self {
        Self {
            required_components: HashSet::new(),
        }
    }
    
    /// Require a component type
    pub fn with<T: Component>(mut self) -> Self {
        self.required_components.insert(TypeId::of::<T>());
        self
    }
    
    /// Check if an archetype matches this filter
    pub fn matches_archetype(&self, archetype_key: &ArchetypeKey) -> bool {
        self.required_components.iter()
            .all(|&tid| archetype_key.contains(tid))
    }
    
    /// Get the required component types
    pub fn required_components(&self) -> &HashSet<TypeId> {
        &self.required_components
    }
}

impl Default for QueryFilter {
    fn default() -> Self { Self::new() }
}

/// A query result containing entities and their components
pub struct QueryResult<'a> {
    world: &'a World,
    filter: QueryFilter,
}

impl<'a> QueryResult<'a> {
    pub fn new(world: &'a World, filter: QueryFilter) -> Self {
        Self { world, filter }
    }
    
    /// Iterate over entity IDs matching the query
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + 'a {
        self.world.archetypes.values()
            .filter(|a| self.filter.matches_archetype(&a.key()))
            .flat_map(|a| a.iter_entities())
    }
    
    /// Iterate over entities with a single component
    pub fn iter_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &'a T)> + 'a {
        // Verify that T is in the filter
        let type_id = TypeId::of::<T>();
        if !self.filter.required_components.contains(&type_id) {
            return Either::Left(std::iter::empty());
        }
        
        Either::Right(self.world.archetypes.values()
            .filter(|a| self.filter.matches_archetype(&a.key()))
            .filter(|a| a.has_component::<T>())
            .flat_map(|archetype| {
                let storage = archetype.get_component_storage::<T>();
                archetype.iter_entities().enumerate()
                    .filter_map(move |(idx, entity)| {
                        storage.get(idx).map(|comp| (entity, comp))
                    })
            }))
    }
}

/// Enum to handle different iterator types
enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, T> Iterator for Either<L, R>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = T>,
{
    type Item = T;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Either::Left(l) => l.next(),
            Either::Right(r) => r.next(),
        }
    }
}

/// Builder for creating queries
pub struct QueryBuilder<'a> {
    world: &'a World,
    filter: QueryFilter,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            filter: QueryFilter::new(),
        }
    }
    
    /// Require a component type
    pub fn with<T: Component>(mut self) -> Self {
        self.filter = self.filter.with::<T>();
        self
    }
    
    /// Execute the query
    pub fn run(self) -> QueryResult<'a> {
        QueryResult::new(self.world, self.filter)
    }
}

/// Extension trait for World to provide ergonomic query syntax
pub trait WorldQueryExt {
    /// Start building a query
    fn query(&self) -> QueryBuilder<'_>;
    
    /// Query for entities with a single component
    fn query_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)> + '_;
    
    /// Query for entities with two components
    fn query_with2<T1: Component, T2: Component>(&self) -> impl Iterator<Item = (EntityId, &T1, &T2)> + '_;
}

impl WorldQueryExt for World {
    fn query(&self) -> QueryBuilder<'_> {
        QueryBuilder::new(self)
    }
    
    fn query_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)> + '_ {
        self.query().with::<T>().run().iter_with::<T>()
    }
    
    fn query_with2<T1: Component, T2: Component>(&self) -> impl Iterator<Item = (EntityId, &T1, &T2)> + '_ {
        let type_id_1 = TypeId::of::<T1>();
        let type_id_2 = TypeId::of::<T2>();
        
        self.archetypes.values()
            .filter(|a| a.has_components(&[type_id_1, type_id_2]))
            .flat_map(|archetype| {
                let storage_1 = archetype.get_component_storage::<T1>();
                let storage_2 = archetype.get_component_storage::<T2>();
                archetype.iter_entities().enumerate()
                    .filter_map(move |(idx, entity)| {
                        let c1 = storage_1.get(idx)?;
                        let c2 = storage_2.get(idx)?;
                        Some((entity, c1, c2))
                    })
            })
    }
}

/// Mutable query extensions
pub trait WorldQueryMutExt {
    /// Process entities with mutable access to a single component
    fn query_mut<T: Component, F>(&mut self, f: F)
    where
        F: FnMut(EntityId, &mut T);
    
    /// Process entities with mutable access to two components
    fn query_mut2<T1: Component, T2: Component, F>(&mut self, f: F)
    where
        F: FnMut(EntityId, &mut T1, &mut T2);
}

impl WorldQueryMutExt for World {
    fn query_mut<T: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(EntityId, &mut T),
    {
        let type_id = TypeId::of::<T>();
        
        for archetype in self.archetypes.values_mut() {
            if !archetype.has_component_type(type_id) {
                continue;
            }
            
            let storage = archetype.get_component_storage_mut::<T>(type_id);
            
            for (idx, &entity) in archetype.entities.iter().enumerate() {
                if let Some(component) = storage.get_mut(idx) {
                    f(entity, component);
                }
            }
        }
    }
    
    fn query_mut2<T1: Component, T2: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(EntityId, &mut T1, &mut T2),
    {
        let type_id_1 = TypeId::of::<T1>();
        let type_id_2 = TypeId::of::<T2>();
        
        for archetype in self.archetypes.values_mut() {
            if !archetype.has_components(&[type_id_1, type_id_2]) {
                continue;
            }
            
            let storage_1 = archetype.get_component_storage_mut::<T1>(type_id_1);
            let storage_2 = archetype.get_component_storage_mut::<T2>(type_id_2);
            
            for (idx, &entity) in archetype.entities.iter().enumerate() {
                if let (Some(c1), Some(c2)) = (storage_1.get_mut(idx), storage_2.get_mut(idx)) {
                    f(entity, c1, c2);
                }
            }
        }
    }
}
```

---

## 4. Phase 3: Testing & Validation - Detailed Implementation

### 4.1 Test Module Structure

```
src/ecs/
├── tests.rs            # Unit tests for the ECS module
```

### 4.2 Unit Tests (`tests.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test component types
    #[derive(Debug, Clone, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    struct Health {
        current: u32,
        max: u32,
    }
    
    impl Component for Position {}
    impl Component for Velocity {}
    impl Component for Health {}
    
    // ==================== Entity Tests ====================
    
    #[test]
    fn test_entity_id_creation() {
        let entity = EntityId::new(42);
        assert_eq!(entity.as_u32(), 42);
        assert!(entity.is_valid());
    }
    
    #[test]
    fn test_entity_id_null() {
        assert!(!EntityId::NULL.is_valid());
        assert_eq!(EntityId::NULL.as_u32(), 0);
    }
    
    #[test]
    fn test_entity_manager_allocation() {
        let mut manager = EntityManager::new();
        
        let e1 = manager.allocate();
        let e2 = manager.allocate();
        
        assert_eq!(e1.as_u32(), 1);
        assert_eq!(e2.as_u32(), 2);
    }
    
    #[test]
    fn test_entity_manager_free_and_reuse() {
        let mut manager = EntityManager::new();
        
        let e1 = manager.allocate();
        let e2 = manager.allocate();
        
        manager.free(e1);
        
        let e3 = manager.allocate();
        
        // e3 should reuse e1's ID
        assert_eq!(e3, e1);
    }
    
    // ==================== Component Storage Tests ====================
    
    #[test]
    fn test_component_vec_basic() {
        let mut storage = ComponentVec::<Position>::new();
        
        let pos1 = Position { x: 1.0, y: 2.0 };
        let pos2 = Position { x: 3.0, y: 4.0 };
        
        let idx1 = storage.push(pos1);
        let idx2 = storage.push(pos2);
        
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(storage.len(), 2);
    }
    
    #[test]
    fn test_component_vec_get() {
        let mut storage = ComponentVec::<Position>::new();
        
        let pos = Position { x: 1.0, y: 2.0 };
        storage.push(pos);
        
        let retrieved = storage.get(0).unwrap();
        assert_eq!(retrieved.x, 1.0);
        assert_eq!(retrieved.y, 2.0);
    }
    
    #[test]
    fn test_component_vec_take() {
        let mut storage = ComponentVec::<Position>::new();
        
        let pos = Position { x: 1.0, y: 2.0 };
        storage.push(pos);
        
        let taken = storage.take(0).unwrap();
        assert_eq!(taken.x, 1.0);
        assert!(!storage.contains(0));
    }
    
    #[test]
    fn test_component_vec_get_mut() {
        let mut storage = ComponentVec::<Position>::new();
        
        let pos = Position { x: 1.0, y: 2.0 };
        storage.push(pos);
        
        let retrieved = storage.get_mut(0).unwrap();
        retrieved.x = 10.0;
        
        assert_eq!(storage.get(0).unwrap().x, 10.0);
    }
    
    // ==================== Archetype Tests ====================
    
    #[test]
    fn test_archetype_key_creation() {
        let mut types = vec![
            TypeId::of::<Position>(),
            TypeId::of::<Velocity>(),
        ];
        types.sort();
        
        let key1 = ArchetypeKey::new(types.clone());
        let key2 = ArchetypeKey::new(types);
        
        assert_eq!(key1, key2);
    }
    
    #[test]
    fn test_archetype_key_contains() {
        let types = vec![
            TypeId::of::<Position>(),
            TypeId::of::<Velocity>(),
        ];
        
        let key = ArchetypeKey::new(types);
        
        assert!(key.contains(TypeId::of::<Position>()));
        assert!(key.contains(TypeId::of::<Velocity>()));
        assert!(!key.contains(TypeId::of::<Health>()));
    }
    
    #[test]
    fn test_archetype_entity_management() {
        let mut archetype = Archetype::new(HashSet::new());
        archetype.add_component_storage::<Position>();
        
        let e1 = EntityId::new(1);
        let e2 = EntityId::new(2);
        
        let idx1 = archetype.add_entity(e1);
        let idx2 = archetype.add_entity(e2);
        
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(archetype.entity_count(), 2);
        assert_eq!(archetype.entity_index(e1), Some(0));
        assert_eq!(archetype.entity_index(e2), Some(1));
    }
    
    #[test]
    fn test_archetype_remove_entity() {
        let mut archetype = Archetype::new(HashSet::new());
        
        let e1 = EntityId::new(1);
        let e2 = EntityId::new(2);
        let e3 = EntityId::new(3);
        
        archetype.add_entity(e1);
        archetype.add_entity(e2);
        archetype.add_entity(e3);
        
        // Remove middle entity
        let removed = archetype.remove_entity(1);
        assert_eq!(removed, e2);
        assert_eq!(archetype.entity_count(), 2);
        
        // Check that e3 was moved to index 1
        assert_eq!(archetype.entity_index(e3), Some(1));
    }
    
    #[test]
    fn test_archetype_component_access() {
        let mut archetype = Archetype::new(HashSet::new());
        archetype.add_component_storage::<Position>();
        
        let e1 = EntityId::new(1);
        let idx = archetype.add_entity(e1);
        
        let pos = Position { x: 1.0, y: 2.0 };
        archetype.add_component::<Position>(idx, pos);
        
        let retrieved = archetype.get_component::<Position>(idx).unwrap();
        assert_eq!(retrieved.x, 1.0);
        assert_eq!(retrieved.y, 2.0);
    }
    
    // ==================== World Tests ====================
    
    #[test]
    fn test_world_spawn_despawn() {
        let mut world = World::new();
        
        let e1 = world.spawn();
        let e2 = world.spawn();
        
        assert!(world.contains(e1));
        assert!(world.contains(e2));
        assert_eq!(world.entity_count(), 2);
        
        assert!(world.despawn(e1));
        assert!(!world.contains(e1));
        assert_eq!(world.entity_count(), 1);
    }
    
    #[test]
    fn test_world_add_get_component() {
        let mut world = World::new();
        
        let entity = world.spawn();
        
        let pos = Position { x: 1.0, y: 2.0 };
        assert!(world.add_component(entity, pos));
        
        let retrieved = world.get_component::<Position>(entity).unwrap();
        assert_eq!(retrieved.x, 1.0);
        assert_eq!(retrieved.y, 2.0);
    }
    
    #[test]
    fn test_world_has_component() {
        let mut world = World::new();
        
        let entity = world.spawn();
        
        assert!(!world.has_component::<Position>(entity));
        
        world.add_component(entity, Position { x: 1.0, y: 2.0 });
        
        assert!(world.has_component::<Position>(entity));
    }
    
    #[test]
    fn test_world_remove_component() {
        let mut world = World::new();
        
        let entity = world.spawn();
        
        let pos = Position { x: 1.0, y: 2.0 };
        world.add_component(entity, pos);
        
        let removed = world.remove_component::<Position>(entity).unwrap();
        assert_eq!(removed.x, 1.0);
        assert_eq!(removed.y, 2.0);
        
        assert!(!world.has_component::<Position>(entity));
    }
    
    #[test]
    fn test_world_component_migration() {
        let mut world = World::new();
        
        let entity = world.spawn();
        
        // Add first component
        world.add_component(entity, Position { x: 1.0, y: 2.0 });
        
        // Verify entity is in archetype with Position
        assert!(world.has_component::<Position>(entity));
        
        // Add second component - should trigger migration
        world.add_component(entity, Velocity { dx: 0.5, dy: 0.5 });
        
        // Verify both components exist
        assert!(world.has_component::<Position>(entity));
        assert!(world.has_component::<Velocity>(entity));
        
        let pos = world.get_component::<Position>(entity).unwrap();
        let vel = world.get_component::<Velocity>(entity).unwrap();
        
        assert_eq!(pos.x, 1.0);
        assert_eq!(vel.dx, 0.5);
    }
    
    #[test]
    fn test_world_iter_entities() {
        let mut world = World::new();
        
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();
        
        let entities: Vec<_> = world.iter_entities().collect();
        
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(entities.contains(&e3));
    }
    
    // ==================== Query Tests ====================
    
    #[test]
    fn test_query_with_single_component() {
        let mut world = World::new();
        
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();
        
        world.add_component(e1, Position { x: 1.0, y: 2.0 });
        world.add_component(e2, Position { x: 3.0, y: 4.0 });
        // e3 has no Position component
        
        let positions: Vec<_> = world.query_with::<Position>().collect();
        
        assert_eq!(positions.len(), 2);
        assert!(positions.iter().any(|(e, _)| *e == e1));
        assert!(positions.iter().any(|(e, _)| *e == e2));
    }
    
    #[test]
    fn test_query_with_multiple_components() {
        let mut world = World::new();
        
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();
        
        // e1: Position + Velocity
        world.add_component(e1, Position { x: 1.0, y: 2.0 });
        world.add_component(e1, Velocity { dx: 0.5, dy: 0.5 });
        
        // e2: Position only
        world.add_component(e2, Position { x: 3.0, y: 4.0 });
        
        // e3: Velocity only
        world.add_component(e3, Velocity { dx: 1.0, dy: 1.0 });
        
        let results: Vec<_> = world.query_with2::<Position, Velocity>().collect();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, e1);
    }
    
    #[test]
    fn test_query_mut() {
        let mut world = World::new();
        
        let e1 = world.spawn();
        let e2 = world.spawn();
        
        world.add_component(e1, Position { x: 1.0, y: 2.0 });
        world.add_component(e2, Position { x: 3.0, y: 4.0 });
        
        world.query_mut::<Position, _>(|_, pos| {
            pos.x *= 2.0;
            pos.y *= 2.0;
        });
        
        let pos1 = world.get_component::<Position>(e1).unwrap();
        let pos2 = world.get_component::<Position>(e2).unwrap();
        
        assert_eq!(pos1.x, 2.0);
        assert_eq!(pos1.y, 4.0);
        assert_eq!(pos2.x, 6.0);
        assert_eq!(pos2.y, 8.0);
    }
    
    #[test]
    fn test_query_mut2() {
        let mut world = World::new();
        
        let e1 = world.spawn();
        
        world.add_component(e1, Position { x: 1.0, y: 2.0 });
        world.add_component(e1, Velocity { dx: 0.5, dy: 0.5 });
        
        world.query_mut2::<Position, Velocity, _>(|_, pos, vel| {
            pos.x += vel.dx;
            pos.y += vel.dy;
        });
        
        let pos = world.get_component::<Position>(e1).unwrap();
        assert_eq!(pos.x, 1.5);
        assert_eq!(pos.y, 2.5);
    }
    
    // ==================== Integration Tests ====================
    
    #[test]
    fn test_game_simulation() {
        let mut world = World::new();
        
        // Create entities
        let player = world.spawn();
        let enemy1 = world.spawn();
        let enemy2 = world.spawn();
        let projectile = world.spawn();
        
        // Add components
        world.add_component(player, Position { x: 0.0, y: 0.0 });
        world.add_component(player, Velocity { dx: 0.0, dy: 0.0 });
        world.add_component(player, Health { current: 100, max: 100 });
        
        world.add_component(enemy1, Position { x: 10.0, y: 10.0 });
        world.add_component(enemy1, Velocity { dx: -0.1, dy: -0.1 });
        world.add_component(enemy1, Health { current: 50, max: 50 });
        
        world.add_component(enemy2, Position { x: -10.0, y: -10.0 });
        world.add_component(enemy2, Velocity { dx: 0.1, dy: 0.1 });
        world.add_component(enemy2, Health { current: 30, max: 30 });
        
        world.add_component(projectile, Position { x: 0.0, y: 0.0 });
        world.add_component(projectile, Velocity { dx: 1.0, dy: 0.0 });
        
        // Update positions
        world.query_mut2::<Position, Velocity, _>(|_, pos, vel| {
            pos.x += vel.dx;
            pos.y += vel.dy;
        });
        
        // Verify player position (should be unchanged - velocity is 0)
        let player_pos = world.get_component::<Position>(player).unwrap();
        assert_eq!(player_pos.x, 0.0);
        assert_eq!(player_pos.y, 0.0);
        
        // Verify enemy1 moved
        let enemy1_pos = world.get_component::<Position>(enemy1).unwrap();
        assert!((enemy1_pos.x - 9.9).abs() < 0.001);
        assert!((enemy1_pos.y - 9.9).abs() < 0.001);
        
        // Verify projectile moved
        let projectile_pos = world.get_component::<Position>(projectile).unwrap();
        assert_eq!(projectile_pos.x, 1.0);
        assert_eq!(projectile_pos.y, 0.0);
    }
    
    #[test]
    fn test_entity_migration_complex() {
        let mut world = World::new();
        
        let entity = world.spawn();
        
        // Start with no components
        assert_eq!(world.archetype_count(), 1); // Empty archetype
        
        // Add Position
        world.add_component(entity, Position { x: 1.0, y: 2.0 });
        assert_eq!(world.archetype_count(), 2); // Empty + Position
        
        // Add Velocity
        world.add_component(entity, Velocity { dx: 0.5, dy: 0.5 });
        assert_eq!(world.archetype_count(), 3); // Empty + Position + Position+Velocity
        
        // Add Health
        world.add_component(entity, Health { current: 100, max: 100 });
        assert_eq!(world.archetype_count(), 4);
        
        // Remove Velocity
        world.remove_component::<Velocity>(entity);
        assert_eq!(world.archetype_count(), 4); // Still 4 (Position+Health is new)
        
        // Verify components
        assert!(world.has_component::<Position>(entity));
        assert!(!world.has_component::<Velocity>(entity));
        assert!(world.has_component::<Health>(entity));
    }
    
    // ==================== Performance Tests ====================
    
    #[test]
    fn test_spawn_many_entities() {
        let mut world = World::new();
        
        const COUNT: usize = 10000;
        
        for _ in 0..COUNT {
            world.spawn();
        }
        
        assert_eq!(world.entity_count(), COUNT);
    }
    
    #[test]
    fn test_add_components_to_many_entities() {
        let mut world = World::new();
        
        const COUNT: usize = 10000;
        
        let entities: Vec<_> = (0..COUNT).map(|_| world.spawn()).collect();
        
        for entity in entities {
            world.add_component(entity, Position { x: 1.0, y: 2.0 });
            world.add_component(entity, Velocity { dx: 0.5, dy: 0.5 });
        }
        
        assert_eq!(world.entity_count(), COUNT);
        
        for entity in entities {
            assert!(world.has_component::<Position>(entity));
            assert!(world.has_component::<Velocity>(entity));
        }
    }
    
    #[test]
    fn test_query_performance() {
        let mut world = World::new();
        
        const COUNT: usize = 10000;
        
        // Create entities with Position
        for _ in 0..COUNT {
            let entity = world.spawn();
            world.add_component(entity, Position { x: 1.0, y: 2.0 });
        }
        
        // Query all entities with Position
        let count = world.query_with::<Position>().count();
        assert_eq!(count, COUNT);
    }
}
```

### 4.3 Test Organization

```rust
// In src/ecs/mod.rs, add:
#[cfg(test)]
mod tests;
```

### 4.4 Benchmark Tests (Optional)

Create `benches/ecs_bench.rs`:

```rust
#![feature(test)]

use renderlib::ecs::{World, Component};
use test::Bencher;

#[derive(Debug, Clone)]
struct Position { x: f32, y: f32 }

#[derive(Debug, Clone)]
struct Velocity { dx: f32, dy: f32 }

impl Component for Position {}
impl Component for Velocity {}

#[bench]
fn bench_spawn_entities(b: &mut Bencher) {
    b.iter(|| {
        let mut world = World::new();
        for _ in 0..1000 {
            world.spawn();
        }
    });
}

#[bench]
fn bench_add_components(b: &mut Bencher) {
    let mut world = World::new();
    let entities: Vec<_> = (0..1000).map(|_| world.spawn()).collect();
    
    b.iter(|| {
        for entity in entities.iter().copied() {
            world.add_component(entity, Position { x: 1.0, y: 2.0 });
        }
    });
}

#[bench]
fn bench_query_entities(b: &mut Bencher) {
    let mut world = World::new();
    
    for _ in 0..10000 {
        let entity = world.spawn();
        world.add_component(entity, Position { x: 1.0, y: 2.0 });
    }
    
    b.iter(|| {
        let count = world.query_with::<Position>().count();
        assert_eq!(count, 10000);
    });
}

#[bench]
fn bench_query_mut_entities(b: &mut Bencher) {
    let mut world = World::new();
    
    for _ in 0..10000 {
        let entity = world.spawn();
        world.add_component(entity, Position { x: 1.0, y: 2.0 });
    }
    
    b.iter(|| {
        world.query_mut::<Position, _>(|_, pos| {
            pos.x += 1.0;
            pos.y += 1.0;
        });
    });
}
```

---

## 5. File Structure & Module Organization

### 5.1 Final Module Structure

```
src/ecs/
├── mod.rs              # Public API exports
├── entity.rs           # EntityId and EntityManager
├── component.rs        # Component trait and storage types
├── archetype.rs        # Archetype struct and operations
├── world.rs            # World struct and core operations
├── query.rs            # Query system
└── tests.rs            # Unit tests

benches/
└── ecs_bench.rs        # Benchmark tests (optional)
```

### 5.2 Public API

```rust
// src/ecs/mod.rs

//! Entity-Component-System implementation
//!
//! A modern ECS with archetype-based storage for cache efficiency.
//!
//! # Example
//!
//! ```
//! use renderlib::ecs::{World, Component};
//!
//! #[derive(Debug)]
//! struct Position { x: f32, y: f32 }
//!
//! impl Component for Position {}
//!
//! let mut world = World::new();
//! let entity = world.spawn();
//! world.add_component(entity, Position { x: 1.0, y: 2.0 });
//!
//! // Query entities with Position
//! for (entity, pos) in world.query_with::<Position>() {
//!     println!("Entity {:?} at ({}, {})", entity, pos.x, pos.y);
//! }
//! ```

pub mod entity;
pub mod component;
pub mod archetype;
pub mod world;
pub mod query;

pub use entity::EntityId;
pub use component::Component;
pub use archetype::ArchetypeKey;
pub use world::World;
pub use query::{WorldQueryExt, WorldQueryMutExt};

#[cfg(test)]
mod tests;
```

---

## 6. Error Handling Strategy

### 6.1 Error Types

```rust
// src/ecs/error.rs

use std::fmt;

/// Error types for ECS operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EcsError {
    /// The specified entity does not exist
    EntityNotFound(EntityId),
    /// The entity already has a component of this type
    ComponentAlreadyExists,
    /// The entity does not have the requested component
    ComponentNotFound,
    /// The archetype does not exist
    ArchetypeNotFound,
    /// Invalid operation (e.g., adding component to wrong archetype)
    InvalidOperation(String),
}

impl fmt::Display for EcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EcsError::EntityNotFound(entity) => write!(f, "Entity {:?} not found", entity),
            EcsError::ComponentAlreadyExists => write!(f, "Component already exists"),
            EcsError::ComponentNotFound => write!(f, "Component not found"),
            EcsError::ArchetypeNotFound => write!(f, "Archetype not found"),
            EcsError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for EcsError {}

impl From<EcsError> for Option<()> {
    fn from(err: EcsError) -> Self {
        match err {
            EcsError::ComponentNotFound | EcsError::EntityNotFound | EcsError::ArchetypeNotFound => None,
            _ => Some(()),
        }
    }
}
```

### 6.2 Result-Based API (Alternative)

For a more robust API, consider using `Result` instead of `Option`:

```rust
impl World {
    /// Add a component to an entity, returning Result
    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> Result<(), EcsError> {
        if !self.contains(entity) {
            return Err(EcsError::EntityNotFound(entity));
        }
        // ... rest of implementation
        Ok(())
    }
    
    /// Get a component from an entity, returning Result
    pub fn get_component<T: Component>(&self, entity: EntityId) -> Result<&T, EcsError> {
        self.get_component_opt(entity)
            .ok_or(EcsError::ComponentNotFound)
    }
}
```

**Decision**: For Phase 1-3, use `Option` for simplicity. Can migrate to `Result` later if needed.

---

## 7. Performance Considerations

### 7.1 Cache Efficiency

**Archetype-Based Storage Benefits**:
- Components of the same type are stored contiguously in memory
- When iterating over entities with specific components, we only access relevant archetypes
- Within an archetype, all components are stored in parallel `Vec<Option<T>>` arrays

**Optimization Strategies**:
1. **Archetype-First Filtering**: Always filter by archetype before accessing components
2. **Batch Processing**: Process all entities in an archetype together for cache locality
3. **Minimize Downcasting**: Use direct access when component types are known at compile time
4. **Pre-allocate Storage**: Reserve space in vectors to avoid reallocations

### 7.2 Memory Usage

**Memory Overhead Analysis**:
| Component | Size | Notes |
|-----------|------|-------|
| EntityId | 4 bytes | u32 |
| TypeId | 8 bytes | std::any::TypeId |
| ArchetypeKey | ~24 bytes | Vec<TypeId> with typical 2-4 components |
| ComponentVec<T> | ~24 bytes + components | Vec<Option<T>> overhead |
| HashMap overhead | ~50% | Typical HashMap load factor |

**Estimated Memory per Entity**:
- Entity ID in world mapping: 4 bytes
- Entity ID in archetype: 4 bytes
- Index in archetype: 4 bytes (HashMap overhead)
- Per component: 1 byte (Option tag) + sizeof(T)

### 7.3 Optimization Opportunities

**Phase 1 Optimizations**:
1. **Inline Component Storage**: For small components, consider using `MaybeUninit` instead of `Option` (unsafe)
2. **Pre-sized Vectors**: Reserve space in component vectors based on expected entity count
3. **SmallVec for Archetype Keys**: Use `SmallVec` for archetype keys with small number of components

**Phase 2+ Optimizations**:
1. **Parallel Iteration**: Use `rayon` for parallel query processing
2. **Component Packing**: Pack multiple small components into a single storage
3. **Generational IDs**: Implement generational entity IDs to prevent use-after-free
4. **Change Detection**: Track modified components to avoid unnecessary processing

### 7.4 Benchmarking Goals

| Operation | Target Time (10k entities) | Notes |
|-----------|---------------------------|-------|
| Spawn entity | < 100ns | Should be O(1) |
| Despawn entity | < 200ns | Includes cleanup |
| Add component | < 500ns | May trigger migration |
| Remove component | < 500ns | May trigger migration |
| Get component | < 50ns | Direct access |
| Query (single component) | < 1ms | Iterate all matching |
| Query (multiple components) | < 2ms | Filter + iterate |

---

## Summary

This specification provides a detailed implementation plan for Phases 1-3 of the ECS system, with explicit Rust considerations for:

1. **Entity Migration**: Using `Option::take()` and `swap_remove` for efficient, copy-free migration
2. **Move Semantics**: `Vec<Option<T>>` storage pattern to avoid temporary copies
3. **Lifetime Management**: Callback-based APIs and archetype-aware iteration for safe mutable access
4. **Type Safety**: Component trait with automatic implementation, type-erased storage with downcasting

The implementation prioritizes:
- **Correctness**: Comprehensive test coverage
- **Performance**: Cache-friendly memory layout, minimal copying
- **Safety**: Rust's borrow checker compliance, no unsafe code in core
- **Extensibility**: Clean module organization, trait-based abstractions

**Next Steps**:
1. Implement Phase 1 (Foundation) following this specification
2. Run unit tests to verify correctness
3. Run benchmarks to establish performance baselines
4. Proceed to Phase 2 (Query System)
5. Proceed to Phase 3 (Testing & Validation)
