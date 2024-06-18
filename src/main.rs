use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use engine::prelude::*;

// TODO: Wrap component access in a ref type

fn main() {
    let mut engine = Engine::new();

    let test_scene = engine.create_scene().unwrap();
    let test_state = engine.scenes().get_scene(&test_scene).unwrap();

    let position = test_state.register_component::<Position>();
    let physics = test_state.register_component::<Physics>();
    let fps_tracker = test_state.register_component::<FPSTracker>();

    // TODO: Add an idea of mutability into the component reqs,
    // that way any systems that use the same components but only read them
    // can run at the same time
    test_state.register_system(&[position, physics], PhysicsSystem);

    test_state.register_system(&[fps_tracker], FpsSystem::new());

    let fps_tracker = test_state.create_entity().unwrap();
    test_state.add_component(&fps_tracker, FPSTracker).unwrap();

    let physics_count = 10;

    for _ in 0..physics_count {
        let entity = test_state.create_entity().unwrap();
        test_state
            .add_component(&entity, Position { x: 0, y: 0 })
            .unwrap();

        test_state
            .add_component(&entity, Physics { dx: 1, dy: 1 })
            .unwrap();
    }

    engine.run(&test_scene);
}

struct Position {
    pub x: i32,
    pub y: i32,
}

struct Physics {
    pub dx: i32,
    pub dy: i32,
}

struct FPSTracker;

struct PhysicsSystem;

impl System for PhysicsSystem {
    fn on_physics_frame(&mut self, engine: Arc<engine::Engine>, entity: Entity) {
        let current_scene = engine.scenes().get_current_scene().unwrap();

        let mut pos = current_scene.get_component::<Position>(&entity).unwrap();
        let phy = current_scene.get_component::<Physics>(&entity).unwrap();

        pos.x += phy.dx;
        pos.y += phy.dy;

        if pos.x > 120 {
            engine
                .scenes()
                .get_current_scene()
                .unwrap()
                .destroy_entity(entity)
                .unwrap();
        }
    }
}

pub struct FpsSystem {
    time_of_last: Instant,
    time_of_last_phys: Instant,
}

impl FpsSystem {
    pub fn new() -> Self {
        Self {
            time_of_last: Instant::now(),
            time_of_last_phys: Instant::now(),
        }
    }
}

impl System for FpsSystem {
    fn on_frame(&mut self, _engine: Arc<engine::Engine>, _entity: Entity, dt: Duration) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.time_of_last);
        println!("FPS: {}", 1.0 / elapsed.as_secs_f32());
        self.time_of_last = now;
    }

    fn on_physics_frame(&mut self, _engine: Arc<engine::Engine>, _entity: Entity) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.time_of_last_phys);
        println!("    PFPS: {}", 1.0 / elapsed.as_secs_f32());
        self.time_of_last_phys = now;
    }
}

impl Default for FpsSystem {
    fn default() -> Self {
        Self::new()
    }
}
