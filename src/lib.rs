mod ecs;
mod scene;
mod thread_pool;
mod timer;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub use ecs::{Component, Entity, System};
pub use scene::Scene;
use scene::SceneManager;
use thread_pool::ThreadPool;
use timer::Timer;

use std::sync::Mutex;

pub struct Engine {
    scene_manager: SceneManager,
    physics_timer: Mutex<Timer>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            scene_manager: SceneManager::new(),
            physics_timer: Mutex::new(Timer::new(Duration::from_secs_f64(1.0 / 60.0))),
        }
    }

    pub fn scenes(&self) -> &SceneManager {
        &self.scene_manager
    }

    pub fn create_scene(&mut self) -> Result<Scene, ecs::Error> {
        self.scene_manager.create_scene()
    }

    pub fn run(self, start_scene: &Scene) {
        let this = Arc::new(self);

        this.scene_manager.set_current_scene(start_scene).unwrap();

        let mut physics_timer = this.physics_timer.lock().unwrap();

        let mut now = Instant::now();

        physics_timer.reset();
        loop {
            let dt = now.elapsed();
            now = Instant::now();
            // swap scenes
            this.scene_manager.swap_scenes(Arc::clone(&this));

            let current_scene = this.scene_manager.get_current_scene().unwrap();

            // TODO: add asset cache

            let is_physics_tick = physics_timer.tick();
            current_scene.on_frame(Arc::clone(&this), is_physics_tick, dt);
            // TODO: Do the same thing with components
            // NOTE: note that you cannot edit data of other scenes due to the fact that it gets recreated
            // when the scene loads and destroyed when it unloads, this means the user of the engine
            // will get an error if they try to edit any scene that isnt the current scene
            // this means that scene initialization will have to be a function passed to the engine
            // that will be run right before the on_entry function of any of the systems,
            // either that needs to be documented, or it needs to be impossible to edit a scene
            // anywhere but in systems or in that function
            current_scene
                .cull_entities()
                .expect("Failed to cull destroyed entities!");
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub mod prelude {
    pub use super::Component;
    pub use super::Engine;
    pub use super::Entity;
    pub use super::Scene;
    pub use super::System;
}

// Plan
// scene systems should be able to request assets from the engine
// engine should use a cache system for assets
