use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::ecs::{self, ComponentManager, EntityManager, SystemManager, UnsafeComponentCell};
use super::{Component, Entity, System};

/// A Scene Handle, guaranteed to be unique per scene
#[derive(Clone, Copy, Debug)]
pub struct Scene(u32);

impl PartialEq for Scene {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Scene {}

impl Hash for Scene {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

pub struct SceneState {
    entity_manager: EntityManager,
    component_manager: ComponentManager,
    system_manager: SystemManager,
    entities_to_kill: RefCell<HashSet<Entity>>,
}

impl SceneState {
    pub fn new() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            component_manager: ComponentManager::new(),
            system_manager: SystemManager::new(),
            entities_to_kill: RefCell::new(HashSet::new()),
        }
    }

    /// Creates a new entity in the scene and returns a handle to it
    ///
    /// Note: Entity handles are unique in their scene but not across scenes
    ///
    /// Panics if it cannot get a lock on the entity manager
    pub fn create_entity(&self) -> Result<Entity, ecs::Error> {
        self.entity_manager.create_entity()
    }

    /// Marks an existing entity in the scene for destruction
    pub fn destroy_entity(&self, entity: Entity) -> Result<(), ecs::Error> {
        let entity_exists = self.entity_manager.does_entity_exist(&entity);

        if entity_exists {
            self.entities_to_kill.borrow_mut().insert(entity);
            Ok(())
        } else {
            Err(ecs::ErrorKind::EntityDoesNotExist.into())
        }
    }

    /// Retrieves handles to all currently living entities in the scene
    pub fn get_living_entities(&self) -> Vec<Entity> {
        self.entity_manager.get_living_entities()
    }

    /// Destroys all amrked entities
    pub(crate) fn cull_entities(&self) -> Result<(), ecs::Error> {
        for entity in self.entities_to_kill.take() {
            self.component_manager.remove_components(&entity)?;

            self.entity_manager.destroy_entity(entity);
        }
        Ok(())
    }

    /// Registers a component for use in the scene and returns a handle to it
    ///
    /// Note: Components cannot be unregistered once registered
    pub fn register_component<C: Send + 'static>(&self) -> Component {
        self.component_manager.register_component::<C>()
    }

    /// Adds a component to an entity in the scene
    pub fn add_component<C: Send + 'static>(
        &self,
        entity: &Entity,
        component: C,
    ) -> Result<(), ecs::Error> {
        let entity_exists = self.entity_manager.does_entity_exist(entity);

        if entity_exists {
            self.component_manager.add_component(entity, component)
        } else {
            Err(ecs::ErrorKind::EntityDoesNotExist.into())
        }
    }

    /// Removes a component from an entity that exists in the scene
    pub fn remove_component<C: Send + 'static>(&self, entity: &Entity) -> Result<(), ecs::Error> {
        let entity_exists = self.entity_manager.does_entity_exist(entity);

        if entity_exists {
            self.component_manager.remove_component::<C>(entity)
        } else {
            Err(ecs::ErrorKind::EntityDoesNotExist.into())
        }
    }

    pub fn get_component<C: Send + 'static>(
        &self,
        entity: &Entity,
    ) -> Result<UnsafeComponentCell<C>, ecs::Error> {
        self.component_manager.get_component::<C>(entity)
    }

    // Checks if an entity has all the given components
    pub fn has_components(
        &self,
        entity: &Entity,
        components: &[Component],
    ) -> Result<bool, ecs::Error> {
        self.component_manager.has_components(entity, components)
    }

    /// Registers a system to be used in the scene
    ///
    /// Note: Systems cannot be unregistered once registered
    ///
    /// # Errors
    /// Accessing any component of an entity other than the ones provided when registering the system
    /// is considered undefined behaviour and should be avoided
    pub fn register_system<S: System + 'static>(&self, signature: &[Component], system: S) {
        self.system_manager.register_system::<S>(signature, system);
    }

    /// Executes the on_entry method of ever registered system in the scene
    pub(crate) fn on_entry(&self, engine: Arc<crate::Engine>) {
        // TODO: Load Scene
        self.system_manager.on_entry(engine);
    }

    /// Executes the on_exit method of every registered system in the scene
    pub(crate) fn on_exit(&self, engine: Arc<crate::Engine>) {
        self.system_manager.on_exit(engine);
        // TODO: Destroy Scene
    }

    /// Executes the on_frame method of ever registered system in the scene
    ///
    /// if is_physics_frame is true, it will also run the on_physics_frame method
    /// before running the on_frame method
    pub(crate) fn on_frame(
        &self,
        engine: Arc<crate::Engine>,
        is_physics_frame: bool,
        dt: Duration,
    ) {
        self.system_manager.on_frame(engine, is_physics_frame, dt);
    }
}

impl Default for SceneState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UnsafeSceneStateCell<'a> {
    data: *const SceneState,
    _owns: PhantomData<&'a SceneState>,
}

impl<'a> Deref for UnsafeSceneStateCell<'a> {
    type Target = SceneState;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

pub struct SceneManager {
    next_scene_id: Mutex<u32>,
    scenes: Mutex<HashMap<Scene, SceneState>>,

    current_scene: Mutex<Option<Scene>>,
    next_scene: Mutex<Option<Scene>>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            next_scene_id: Mutex::new(0),
            scenes: Mutex::new(HashMap::new()),
            current_scene: Mutex::new(None),
            next_scene: Mutex::new(None),
        }
    }

    /// Creates a new scene and returns a handle to it
    pub fn create_scene(&self) -> Result<Scene, ecs::Error> {
        if *self.next_scene_id.lock().unwrap() == u32::MAX {
            Err(ecs::ErrorKind::SceneMaxReached.into())
        } else {
            let scene_handle = Scene(*self.next_scene_id.lock().unwrap());
            *self.next_scene_id.lock().unwrap() += 1;

            self.scenes
                .lock()
                .unwrap()
                .insert(scene_handle, SceneState::default());

            Ok(scene_handle)
        }
    }

    /// Retrieves a mutable reference to the requested scene
    pub fn get_scene(&self, scene: &Scene) -> Result<UnsafeSceneStateCell, ecs::Error> {
        if self.does_scene_exist(scene) {
            let scenes = self.scenes.lock().unwrap();

            let scene = scenes
                .get(scene)
                .ok_or(ecs::Error::from(ecs::ErrorKind::SceneDoesNotExist))?;

            Ok(UnsafeSceneStateCell {
                data: std::ptr::from_ref(scene),
                _owns: PhantomData,
            })
        } else {
            Err(ecs::ErrorKind::SceneDoesNotExist.into())
        }
    }

    /// Retrieves a mutable reference to the current scene
    pub fn get_current_scene(&self) -> Result<UnsafeSceneStateCell, ecs::Error> {
        if let Some(scene) = *self.current_scene.lock().unwrap() {
            self.get_scene(&scene)
        } else {
            Err(ecs::ErrorKind::NoCurrentScene.into())
        }
    }

    /// Sets the current scene
    pub fn set_current_scene(&self, scene: &Scene) -> Result<(), ecs::Error> {
        if self.does_scene_exist(scene) {
            *self.next_scene.lock().unwrap() = Some(*scene);
            Ok(())
        } else {
            Err(ecs::ErrorKind::SceneDoesNotExist.into())
        }
    }

    // Swaps scenes if next scene is set
    pub fn swap_scenes(&self, engine: Arc<crate::Engine>) {
        if let Some(scene) = self.next_scene.lock().unwrap().take() {
            if let Ok(scene) = self.get_current_scene() {
                scene.on_exit(Arc::clone(&engine));
            }
            *self.current_scene.lock().unwrap() = Some(scene);
            self.get_current_scene().unwrap().on_entry(engine)
        }
    }

    /// Checks if a scene exists
    fn does_scene_exist(&self, scene: &Scene) -> bool {
        self.scenes.lock().unwrap().contains_key(scene)
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}
