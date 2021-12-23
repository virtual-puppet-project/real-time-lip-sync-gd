use gdnative::prelude::*;

mod lip_sync;

mod algorithm;
mod debug;

fn init(handle: InitHandle) {
    handle.add_class::<lip_sync::LipSync>();
}

godot_init!(init);
