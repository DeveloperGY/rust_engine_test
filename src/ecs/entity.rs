use super::{Error, ErrorKind};
use std::cell::RefCell;
use std::{collections::VecDeque, hash::Hash};

/// An Entity Id, guaranteed to be unique from all the entities
/// created by the given entity manager
#[derive(Debug)]
pub struct Entity(u32);

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Entity {}

impl Hash for Entity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Entity {
    /// Clones an Entity Id
    ///
    /// This is used by the ecs internally and should not be implemented
    /// globally, that way the ecs can take ownership of an entity id when
    /// destroying it to remove any accidental access to the destroyed entity
    /// while still allowing the ecs to copy handles to entities when it needs to
    pub(crate) fn clone(&self) -> Entity {
        Self(self.0)
    }
}

pub struct EntityManager {
    next_entity_id: RefCell<u32>,
    dead_entities: RefCell<VecDeque<u32>>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            next_entity_id: RefCell::new(u32::MIN),
            dead_entities: RefCell::new(VecDeque::new()),
        }
    }

    /// Creates a new entity, unique to this entity manager
    pub fn create_entity(&self) -> Result<Entity, Error> {
        if !self.dead_entities.borrow().is_empty() {
            return Ok(Entity(self.dead_entities.borrow_mut().pop_front().unwrap()));
        }

        if *self.next_entity_id.borrow() == u32::MAX {
            return Err(ErrorKind::EntityMaxReached.into());
        }

        *self.next_entity_id.borrow_mut() += 1;
        Ok(Entity(*self.next_entity_id.borrow() - 1))
    }

    /// Destroys an entity if it hasn't been already
    pub fn destroy_entity(&self, entity: Entity) {
        if self.does_entity_exist(&entity) {
            self.dead_entities.borrow_mut().push_back(entity.0);
        }
    }

    /// Retrieves all living entities from this entity manager
    pub fn get_living_entities(&self) -> Vec<Entity> {
        (0..*self.next_entity_id.borrow())
            .filter(|e| !self.dead_entities.borrow().contains(e))
            .map(Entity)
            .collect()
    }

    /// Checks if an entity exists in the entity manager
    pub fn does_entity_exist(&self, entity: &Entity) -> bool {
        *self.next_entity_id.borrow() > entity.0 && !self.dead_entities.borrow().contains(&entity.0)
    }
}
