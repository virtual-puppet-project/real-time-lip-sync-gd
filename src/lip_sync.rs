use gdnative::{
    api::{AudioEffect, AudioServer},
    prelude::*,
};
use std::{
    sync::{Arc, Mutex},
    thread,
};

use crate::common::LipSyncInfo;
use crate::lip_sync_job::*;
use crate::profile::*;

// struct JobHandle(mpsc::Receiver<LipSyncJobHandle>);

// unsafe impl Sync for JobHandle {}
// unsafe impl Send for JobHandle {}

// struct JobTransmitter(mpsc::Sender<LipSyncJobHandle>);

// unsafe impl Sync for JobTransmitter {}
// unsafe impl Send for JobTransmitter {}

#[derive(NativeClass)]
#[inherit(Reference)]
#[user_data(user_data::RwLockData<LipSync>)]
// #[user_data(user_data::ArcData<LipSync>)]
#[register_with(Self::register_lip_sync)]
pub struct LipSync {
    // Godot-specific stuff
    effect: Option<Ref<AudioEffect, Shared>>,

    pub profile: Profile,
    pub output_sound_gain: f64,

    index: i64,

    raw_input_data: Vec<f64>,
    input_data: Vec<f64>,
    mfcc: Arc<Mutex<Vec<f64>>>,
    mfcc_for_other: Vec<f64>,
    phonemes: Vec<f64>,
    job_result: Arc<Mutex<LipSyncJobResult>>,
    requested_calibration_vowels: Vec<i64>,

    result: LipSyncInfo,

    join_handle: Option<thread::JoinHandle<()>>,
    job: Arc<Mutex<LipSyncJob>>,
}

#[methods]
impl LipSync {
    fn new(_owner: &Reference) -> Self {
        let job = Arc::new(Mutex::new(LipSyncJob::new()));
        let thread_job = job.clone();

        let builder = thread::Builder::new();
        // TODO handle thread spawn error
        let join_handle = match builder.spawn(move || loop {
            // TODO don't unwrap, handle poisoning instead
            let mut job = thread_job.lock().expect("Unable to lock job in thread");

            if !job.is_complete {
                job.execute();
                job.is_complete = true;
            }
            drop(job);
        }) {
            Ok(v) => Some(v),
            Err(_) => None,
        };

        LipSync {
            effect: None,

            profile: Profile::new(),
            output_sound_gain: 1.0,

            index: 0,

            raw_input_data: vec![],
            input_data: vec![],
            mfcc: Arc::new(Mutex::new(vec![0.0; 12])),
            mfcc_for_other: vec![],
            phonemes: vec![],
            job_result: Arc::new(Mutex::new(LipSyncJobResult::default())),
            requested_calibration_vowels: vec![],

            result: LipSyncInfo::default(),

            join_handle: join_handle,
            job: job,
        }
    }

    fn register_lip_sync(builder: &ClassBuilder<Self>) {
        builder.add_signal(Signal {
            name: "lip_sync_updated",
            args: &[SignalArgument {
                name: "result",
                default: Variant::from_dictionary(&Dictionary::default()),
                export_info: ExportInfo::new(VariantType::Dictionary),
                usage: PropertyUsage::DEFAULT,
            }],
        })
    }

    // Maps to Awake() in the Unity impl
    #[export]
    fn _init(&mut self, _owner: &Reference) {
        self.update_audio_source();
    }

    // Maps to Update() in the Unity impl
    #[export]
    pub fn update(&mut self, owner: &Reference) {
        {
            // TODO don't unwrap, handle poisoning instead
            let job = self
                .job
                .lock()
                .expect("Unable to lock job in update_result");
            if !job.is_complete {
                return;
            }
        }

        self.update_result();
        self.invoke_callback(owner);
        self.update_calibration();
        self.update_phonemes();
        self.schedule_job();

        self.update_buffers();
        self.update_audio_source();
    }

    #[export]
    pub fn shutdown(&mut self, _owner: &Reference) {
        self.join_handle
            .take()
            .expect("Unable to get join handle from option")
            .join()
            .expect("Unable to join thread");
    }

    fn awake() {
        unimplemented!("Unity-specific")
    }

    fn on_enable() {
        unimplemented!("Unity-specific")
    }

    fn on_disable() {
        unimplemented!("Unity-specific")
    }

    fn allocate_buffers(&mut self) {
        // self.raw_input_data = vec![];
        // self.input_data = vec![];
        // self.mfcc = Arc::new(Mutex::new(vec![0.0; 12]));
        // self.mfcc_for_other = vec![];
        // let mut res = self
        //     .job_result
        //     .lock()
        //     .expect("Unable to lock job_result when allocating");
        // res.index = 0;
        // res.volume = 0.0;
        // res.distance = 0.0;
        // res = LipSyncJobResult::default();
        // self.phonemes = vec![];
    }

    fn dispose_buffers(&mut self) {
        self.raw_input_data.clear();
        self.input_data.clear();
        let mut mfcc = self
            .mfcc
            .lock()
            .expect("Unable to lock mfcc when disposing of the buffer");
        mfcc.clear();
        mfcc.resize(12, 0.0);
        self.mfcc_for_other.clear();
        let mut res = self
            .job_result
            .lock()
            .expect("Unable to lock job_result when disposing of the buffer");
        res.index = 0;
        res.volume = 0.0;
        res.distance = 0.0;
        self.phonemes.clear();
    }

    fn update_buffers(&mut self) {
        if self.input_sample_count() != self.raw_input_data.len() as i64
            || self.profile.mfccs.len() * 12 != self.phonemes.len()
        {
            self.dispose_buffers();
            self.allocate_buffers();
        }
    }

    fn update_result(&mut self) {
        // TODO wait for thread to complete
        // don't want to join thread because that will block
        // TODO don't unwrap, handle poisoning instead
        let job = self
            .job
            .lock()
            .expect("Unable to lock job in update_result");

        let mfcc = job.mfcc.lock().expect("Unable to lock mfcc on job");
        self.mfcc_for_other.copy_from_slice(&mfcc);

        // TODO hopefully they're not just using lists as their main data structure
        let result = job.result.lock().expect("Unable to lock result on job");
        let index = result.index;
        let phoneme = self.profile.get_phoneme(index as usize);
        let distance = result.distance;
        let mut vol = result.volume.log10();
        let min_vol = self.profile.min_volume;
        let max_vol = self.profile.max_volume.max(min_vol + 1e-4_f64);
        vol = (vol - min_vol) / (max_vol - min_vol);
        vol = f64::clamp(vol, 0.0, 1.0);

        self.result = LipSyncInfo::new(index, phoneme, vol, result.volume, distance);
    }

    fn invoke_callback(&mut self, owner: &Reference) {
        owner.emit_signal(
            "lip_sync_updated",
            &[Variant::from_dictionary(&self.result(owner))],
        );
    }

    fn update_phonemes(&mut self) {
        let mut index: usize = 0;
        for data in self.profile.mfccs.iter() {
            for value in data.mfcc_native_array.iter() {
                if index >= self.phonemes.len() {
                    break;
                }
                index += 1;
                self.phonemes[index] = *value;
            }
        }
    }

    fn schedule_job(&mut self) {
        // The logic here doesn't make sense from the Unity impl
        // thus, it is commented out and we just use the struct value
        // let mut index: i64 = 0;

        self.input_data.clone_from(&self.raw_input_data);
        // index = self.index;

        // TODO don't unwrap, handle poisoning instead
        let mut job = self.job.lock().expect("Unable to lock job in schedule_job");
        // TODO cloning input for now, we might actually need a reference
        job.input = self.input_data.clone();
        job.start_index = self.index;
        job.output_sample_rate = AudioServer::godot_singleton().get_mix_rate() as i64;
        job.target_sample_rate = self.profile.target_sample_rate;
        job.volume_thresh = (10.0 as f64).powf(self.profile.min_volume);
        job.mel_filter_bank_channels = self.profile.mel_filter_bank_channels;
        job.mfcc = self.mfcc.clone();
        job.phonemes = self.phonemes.clone();
        job.result = self.job_result.clone();

        job.is_complete = false;
    }

    #[export]
    pub fn request_calibration(&mut self, _owner: &Reference, index: i64) {
        if index < 0 {
            return;
        }
        self.requested_calibration_vowels.push(index);
    }

    fn update_calibration(&mut self) {
        let shared_mfcc = self
            .mfcc
            .lock()
            .expect("Unable to lock mfcc in update_calibration");
        for index in self.requested_calibration_vowels.iter() {
            // We can assume index is greater than 0 because we check
            // for this in request_calibration
            self.profile
                .update_mfcc(*index as usize, shared_mfcc.clone(), true);
        }

        self.requested_calibration_vowels.clear();
    }

    fn update_audio_source(&mut self) {
        let audio_server = AudioServer::godot_singleton();
        let record_effect_index = audio_server.get_bus_index("Record");
        self.effect = audio_server.get_bus_effect(record_effect_index, 0);
    }

    // TODO connect to some audio thing
    // https://github.com/godot-rust/godot-rust/blob/0.9.3/examples/signals/src/lib.rs#L73
    fn on_data_received(&mut self, _owner: &Reference, input: &mut TypedArray<f32>, channels: i64) {
        if self.raw_input_data.len() == 0 {
            return;
        }

        let n = self.raw_input_data.len() as i64;
        self.index = self.index % n;
        let mut i = 0;
        while i < input.len() {
            self.index = (self.index + 1) % n;
            self.raw_input_data[self.index as usize] = input.get(i as i32).into();

            i += channels as i32;
        }

        if (self.output_sound_gain - 1.0).abs() > f64::EPSILON {
            let n = input.len() as i32;
            for i in 0..n {
                input.set(i, input.get(i) * self.output_sound_gain as f32);
            }
        }
    }

    fn on_audio_filter_read() {
        // TODO this might not be true
        unimplemented!("Unity-specific")
    }

    // Changed from property in the Unity impl to function
    // TODO might need to convert to Godot Array
    pub fn mfcc(&self) -> &Vec<f64> {
        &self.mfcc_for_other
    }

    // Changed from property in the Unity impl to function
    #[export]
    pub fn result(&self, _owner: &Reference) -> Dictionary {
        let dict = Dictionary::new();
        dict.insert("index", self.result.index);
        dict.insert("phoneme", self.result.phoneme.clone());
        dict.insert("volume", self.result.volume);
        dict.insert("raw_volume", self.result.raw_volume);
        dict.insert("distance", self.result.distance);

        dict.into_shared()
    }

    // Changed from property in the Unity impl to function
    fn input_sample_count(&self) -> i64 {
        let r =
            AudioServer::godot_singleton().get_mix_rate() / self.profile.target_sample_rate as f64;
        (self.profile.sample_count as f64 * r).ceil() as i64
    }
}
