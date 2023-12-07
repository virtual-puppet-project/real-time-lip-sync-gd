use godot::prelude::*;

mod lip_sync;

mod algorithm;
mod debug;
mod job;
mod model;

struct LipSyncLib;

#[gdextension]
unsafe impl ExtensionLibrary for LipSyncLib {}
