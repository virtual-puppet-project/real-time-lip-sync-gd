use gdnative::prelude::*;

mod lip_sync;

mod algorithm;
mod debug;
mod job;
mod model;

fn init(handle: InitHandle) {
    handle.add_class::<lip_sync::LipSync>();
}

godot_init!(init);
