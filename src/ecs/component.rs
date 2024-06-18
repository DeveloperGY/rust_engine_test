use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

use super::{Entity, Error, ErrorKind};

/// A Component Type Id
///
/// Explicit Component Type Ids are used when the ECS
/// cannot infer a component's Component Type Id
///
/// All Component Type Ids that refer to the same component type will
/// be equivalent
pub type Component = TypeId;

pub struct UnsafeComponentCell<'a, C> {
    data: *mut C,
    _owns: PhantomData<&'a mut C>,
}

impl<'a, C> Deref for UnsafeComponentCell<'a, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, C> DerefMut for UnsafeComponentCell<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

// See https://ianjk.com/ecs-in-rust/ for more details
trait ComponentArray {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear(&mut self);
    fn remove_component(&mut self, entity: &Entity) -> Result<(), Error>;
    fn has_entity_data(&self, entity: &Entity) -> bool;
}

impl<T: Send + 'static> ComponentArray for HashMap<Entity, T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn remove_component(&mut self, entity: &Entity) -> Result<(), Error> {
        self.remove(entity);
        Ok(())
    }

    fn has_entity_data(&self, entity: &Entity) -> bool {
        self.contains_key(entity)
    }
}

pub struct ComponentManager {
    components: Mutex<HashMap<TypeId, Mutex<Box<dyn ComponentArray + Send>>>>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            components: Mutex::new(HashMap::new()),
        }
    }

    /// Adds a component to an entity, replacing the current component
    /// if the entity already has one
    pub fn add_component<C: Send + 'static>(
        &self,
        entity: &Entity,
        component: C,
    ) -> Result<(), Error> {
        if let Some(v) = self.components.lock().unwrap().get(&TypeId::of::<C>()) {
            if let Some(v) = v
                .lock()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<HashMap<Entity, C>>()
            {
                v.insert(entity.clone(), component);
                Ok(())
            } else {
                Err(ErrorKind::ComponentArrayDowncastFailure.into())
            }
        } else {
            Err(ErrorKind::ComponentNotRegistered.into())
        }
    }

    /// Removes a component from an entity if it has one
    pub fn remove_component<C: Send + 'static>(&self, entity: &Entity) -> Result<(), Error> {
        if let Some(v) = self.components.lock().unwrap().get(&TypeId::of::<C>()) {
            if let Some(v) = v
                .lock()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<HashMap<Entity, C>>()
            {
                v.remove(entity);
                Ok(())
            } else {
                Err(ErrorKind::ComponentArrayDowncastFailure.into())
            }
        } else {
            Err(ErrorKind::ComponentNotRegistered.into())
        }
    }

    /// Removes all components from an entity
    pub fn remove_components(&self, entity: &Entity) -> Result<(), Error> {
        for comp_arr in self.components.lock().unwrap().iter() {
            comp_arr.1.lock().unwrap().remove_component(entity)?;
        }
        Ok(())
    }

    /// Checks if an entity has all the given components
    pub fn has_components(&self, entity: &Entity, components: &[Component]) -> Result<bool, Error> {
        for comp in components {
            if let Some(v) = self.components.lock().unwrap().get(comp) {
                if !v.lock().unwrap().has_entity_data(entity) {
                    return Ok(false);
                }
            } else {
                return Err(ErrorKind::ComponentNotRegistered.into());
            }
        }

        Ok(true)
    }

    /// Registers a new component for use with the component manager
    pub fn register_component<C: Send + 'static>(&self) -> Component {
        let type_id = TypeId::of::<C>();

        if self.is_component_registered(&type_id) {
            // type cast is redundant, but it makes the code intention easier to see
            type_id as Component
        } else {
            self.components
                .lock()
                .unwrap()
                .insert(type_id, Mutex::new(Box::<HashMap<Entity, C>>::default()));

            // type cast is redundant, but it makes the code intention easier to see
            type_id as Component
        }
    }

    /// Checks if a component has been registered with the component manager
    pub fn is_component_registered(&self, component: &Component) -> bool {
        self.components.lock().unwrap().contains_key(component)
    }

    /// Retrieves a mutable reference to a component
    pub fn get_component<C: Send + 'static>(
        &self,
        entity: &Entity,
    ) -> Result<UnsafeComponentCell<'_, C>, Error> {
        if let Some(v) = self.components.lock().unwrap().get(&TypeId::of::<C>()) {
            if v.lock().unwrap().has_entity_data(entity) {
                if let Some(v) = v
                    .lock()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<HashMap<Entity, C>>()
                {
                    // Expect is fine since the component array is guaranteed to have the
                    // component, yet still leaves a message in case there is flawed code logic
                    let component = v
                        .get_mut(entity)
                        .expect("Failed to get component despite entity owning component!");

                    let unsafe_cell = UnsafeComponentCell {
                        data: std::ptr::from_mut(component),
                        _owns: PhantomData,
                    };

                    Ok(unsafe_cell)
                } else {
                    Err(ErrorKind::ComponentArrayDowncastFailure.into())
                }
            } else {
                Err(ErrorKind::EntityDoesNotOwnComponent.into())
            }
        } else {
            Err(ErrorKind::ComponentNotRegistered.into())
        }
    }
}
