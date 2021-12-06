use gdnative::{
    api::{AudioEffect, AudioServer},
    prelude::*,
};

use crate::profile::*;

#[derive(NativeClass)]
#[inherit(Reference)]
#[user_data(user_data::RwLockData<LipSync>)]
pub struct LipSync {
    // Godot-specific stuff
    effect: Option<Ref<AudioEffect, Shared>>,

    // Unity stuff
    pub profile: Profile,

    index: i64,

    raw_input_data: Vec<f64>,
    input_data: Vec<f64>,
    mfcc: Vec<f64>,
    mfcc_for_other: Vec<f64>,
    phonemes: Vec<f64>,
    // job_result: Vec<
}

#[methods]
impl LipSync {
    fn new(_owner: &Reference) -> Self {
        LipSync {
            effect: None,

            profile: Profile::new(),

            index: 0,

            raw_input_data: vec![],
            input_data: vec![],
            mfcc: vec![],
            mfcc_for_other: vec![],
            phonemes: vec![],
        }
    }

    #[export]
    fn _init(&mut self, _owner: &Reference) {
        self.update_audio_source();
    }

    fn _process(&mut self, _owner: &Reference) {
        //

        self.update_result();
        self.invoke_callback();
        self.update_calibration();
        self.update_phonemes();
        self.schedule_job();

        self.update_buffers();
        // self.update_audio_source();
    }

    fn update_result(&mut self) {
        // wait for thread to complete
        // TODO stub

        self.mfcc_for_other.copy_from_slice(&self.mfcc);

        // get index from job result
        // let phoneme = self.p
    }

    fn invoke_callback(&mut self) {}

    fn update_calibration(&mut self) {}

    fn update_phonemes(&mut self) {}

    fn schedule_job(&mut self) {}

    fn update_buffers(&mut self) {}

    fn update_audio_source(&mut self) {
        let audio_server = AudioServer::godot_singleton();
        let record_effect_index = audio_server.get_bus_index("Record");
        self.effect = audio_server.get_bus_effect(record_effect_index, 0);
    }
}
