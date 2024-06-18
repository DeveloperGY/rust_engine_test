use std::{
    any::TypeId,
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use super::{Component, Entity};
use crate::ThreadPool;

pub trait System: Send {
    /// runs when the scene is loaded
    fn on_entry(&mut self, _engine: Arc<crate::Engine>, _entity: Entity) {}

    /// runs when the scene is unloaded
    fn on_exit(&mut self, _engine: Arc<crate::Engine>, _entity: Entity) {}

    /// runs every frame
    fn on_frame(&mut self, _engine: Arc<crate::Engine>, _entity: Entity, _dt: Duration) {}

    /// runs every physics frame (fixed rate)
    fn on_physics_frame(&mut self, _engine: Arc<crate::Engine>, _entity: Entity) {}
}

/// The list of required components and the system itself
type SystemData = (Vec<Component>, Arc<Mutex<dyn System>>);

pub struct SystemManager {
    systems: Systems,
}

impl SystemManager {
    pub fn new() -> Self {
        Self {
            systems: Systems::new(),
        }
    }

    /// Registers a system for use in the scene
    pub fn register_system<S: System + 'static>(&self, signature: &[Component], system: S) {
        self.systems.add_system(signature, system);
    }

    pub fn on_entry(&self, engine: Arc<crate::Engine>) {
        self.systems.on_entry(engine);
    }

    pub fn on_exit(&self, engine: Arc<crate::Engine>) {
        self.systems.on_exit(engine);
    }

    pub fn on_frame(&self, engine: Arc<crate::Engine>, is_physics_frame: bool, dt: Duration) {
        self.systems.on_frame(engine, is_physics_frame, dt);
    }
}

pub struct Systems {
    system_list: Mutex<HashMap<TypeId, SystemData>>,
    system_parallels: Mutex<Vec<Vec<TypeId>>>,
    t_pool: RefCell<ThreadPool>,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            system_list: Mutex::new(HashMap::new()),
            system_parallels: Mutex::new(Vec::new()),
            t_pool: RefCell::new(ThreadPool::new(4)),
        }
    }

    pub fn add_system<S: System + 'static>(&self, signature: &[Component], system: S) {
        let system_id = TypeId::of::<S>();
        let signature = signature.to_vec();

        if self.system_list.lock().unwrap().contains_key(&system_id) {
            // this system has already been added
            return;
        }

        self.system_list
            .lock()
            .unwrap()
            .insert(system_id, (signature.clone(), Arc::new(Mutex::new(system))));

        let mut parallels = self.system_parallels.lock().unwrap();

        let mut is_inserted = false;

        for parallel in parallels.iter_mut() {
            let mut fits_in_parallel = true;

            for system in parallel.iter() {
                // check for overlapping components
                let system_list = self.system_list.lock().unwrap();

                let (components, _) = system_list.get(system).unwrap();

                if components.iter().any(|val| components.contains(val)) {
                    fits_in_parallel = false;
                    break;
                }
            }

            if fits_in_parallel {
                parallel.push(system_id);
                is_inserted = true;
                break;
            }
        }

        if !is_inserted {
            parallels.push(vec![system_id]);
        }
    }

    pub fn on_entry(&self, engine: Arc<crate::Engine>) {
        let parallels = self.system_parallels.lock().unwrap();
        let systems = self.system_list.lock().unwrap();

        for parallel in parallels.iter() {
            for system_id in parallel {
                let (reqs, system) = systems.get(system_id).unwrap();

                let entity_list = engine
                    .scenes()
                    .get_current_scene()
                    .unwrap()
                    .get_living_entities();

                let system_entities = entity_list
                    .into_iter()
                    .filter(|e| {
                        engine
                            .scenes()
                            .get_current_scene()
                            .unwrap()
                            .has_components(e, reqs)
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                let engine_handle = Arc::clone(&engine);
                let system_handle = Arc::clone(system);

                self.t_pool.borrow_mut().execute(move || {
                    let mut system = system_handle.lock().unwrap();
                    for entity in system_entities {
                        system.on_entry(Arc::clone(&engine_handle), entity.clone());
                    }
                });
            }
            self.t_pool.borrow().wait();
        }
    }

    pub fn on_exit(&self, engine: Arc<crate::Engine>) {
        let parallels = self.system_parallels.lock().unwrap();
        let systems = self.system_list.lock().unwrap();

        for parallel in parallels.iter() {
            for system_id in parallel {
                let (reqs, system) = systems.get(system_id).unwrap();

                let entity_list = engine
                    .scenes()
                    .get_current_scene()
                    .unwrap()
                    .get_living_entities();

                let system_entities = entity_list
                    .into_iter()
                    .filter(|e| {
                        engine
                            .scenes()
                            .get_current_scene()
                            .unwrap()
                            .has_components(e, reqs)
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                let engine_handle = Arc::clone(&engine);
                let system_handle = Arc::clone(system);

                self.t_pool.borrow_mut().execute(move || {
                    let mut system = system_handle.lock().unwrap();
                    for entity in system_entities {
                        system.on_exit(Arc::clone(&engine_handle), entity.clone());
                    }
                });
            }
            self.t_pool.borrow().wait();
        }
    }

    pub fn on_frame(&self, engine: Arc<crate::Engine>, is_physics_frame: bool, dt: Duration) {
        let parallels = self.system_parallels.lock().unwrap();
        let systems = self.system_list.lock().unwrap();

        if is_physics_frame {
            for parallel in parallels.iter() {
                for system_id in parallel {
                    let (reqs, system) = systems.get(system_id).unwrap();

                    let entity_list = engine
                        .scenes()
                        .get_current_scene()
                        .unwrap()
                        .get_living_entities();

                    let system_entities = entity_list
                        .into_iter()
                        .filter(|e| {
                            engine
                                .scenes()
                                .get_current_scene()
                                .unwrap()
                                .has_components(e, reqs)
                                .unwrap()
                        })
                        .collect::<Vec<_>>();

                    let engine_handle = Arc::clone(&engine);
                    let system_handle = Arc::clone(system);

                    self.t_pool.borrow_mut().execute(move || {
                        let mut system = system_handle.lock().unwrap();
                        for entity in system_entities {
                            system.on_physics_frame(Arc::clone(&engine_handle), entity.clone());
                        }
                    });
                }
            }
            self.t_pool.borrow().wait();
        }

        for parallel in parallels.iter() {
            for system_id in parallel {
                let (reqs, system) = systems.get(system_id).unwrap();

                let entity_list = engine
                    .scenes()
                    .get_current_scene()
                    .unwrap()
                    .get_living_entities();

                let system_entities = entity_list
                    .into_iter()
                    .filter(|e| {
                        engine
                            .scenes()
                            .get_current_scene()
                            .unwrap()
                            .has_components(e, reqs)
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                let engine_handle = Arc::clone(&engine);
                let system_handle = Arc::clone(system);

                self.t_pool.borrow_mut().execute(move || {
                    let mut system = system_handle.lock().unwrap();
                    for entity in system_entities {
                        system.on_frame(Arc::clone(&engine_handle), entity.clone(), dt);
                    }
                });
            }
            self.t_pool.borrow().wait();
        }
    }
}
