use gdnative::{
    api::{AudioEffectRecord, AudioServer, AudioStreamSample},
    prelude::*,
};
use lazy_static::lazy_static;
use rand::{rngs::ThreadRng, Rng};
use std::{
    collections::{HashMap, VecDeque},
    ops::{Add, Div, Index, Mul, MulAssign},
    sync::{Arc, Mutex},
    thread,
};

use crate::algorithm::*;

const FFT_SAMPLES: usize = 1024;
const UPDATE_FRAME: usize = 5;
const DYNAMIC_RANGE: f32 = 100.0;

const LIP_SYNC_UPDATED: &str = "lip_sync_updated";
const LIP_SYNC_PANICKED: &str = "lip_sync_panicked";

#[derive(Debug, PartialEq, Clone)]
pub struct DataPoint(pub f32, pub f32);

impl DataPoint {
    pub fn exp(self) -> DataPoint {
        let e = self.0.exp();

        DataPoint(e * self.1.cos(), e * self.1.sin())
    }

    pub fn zero() -> DataPoint {
        DataPoint(0.0, 0.0)
    }
}

impl Add for DataPoint {
    type Output = DataPoint;
    fn add(self, other: DataPoint) -> DataPoint {
        DataPoint(self.0 + other.0, self.1 + other.1)
    }
}

impl Mul<DataPoint> for DataPoint {
    type Output = DataPoint;
    fn mul(self, other: DataPoint) -> DataPoint {
        let r = self.0 * other.0 - self.1 * other.1;
        let i = self.0 * other.0 + self.1 * other.1;

        DataPoint(r, i)
    }
}

impl MulAssign<f32> for DataPoint {
    fn mul_assign(&mut self, other: f32) {
        self.0 *= other;
        self.1 *= other;
    }
}

impl Div for DataPoint {
    type Output = DataPoint;
    fn div(self, other: DataPoint) -> DataPoint {
        let r = self.0 * other.0 + self.1 * other.1;
        let i = self.1 * other.0 - self.1 * other.1;
        let d = other.0 * other.0 + other.1 * other.1;

        DataPoint(r / d, i / d)
    }
}

#[derive(Debug, PartialEq)]
struct Phoneme(Vec<DataPoint>);

impl Index<usize> for Phoneme {
    type Output = DataPoint;
    fn index(&self, idx: usize) -> &DataPoint {
        &self.0[idx]
    }
}

#[derive(Debug)]
struct VowelEstimate {
    estimate: i32,
    vowel: i32,
    amount: f32,
}

impl VowelEstimate {
    fn new(estimate: i32, vowel: i32, amount: f32) -> Self {
        VowelEstimate {
            estimate: estimate,
            vowel: vowel,
            amount: amount,
        }
    }
}

impl From<VowelEstimate> for Dictionary {
    fn from(ve: VowelEstimate) -> Self {
        let dict = Dictionary::new();

        dict.insert("estimate", ve.estimate);
        dict.insert("vowel", ve.vowel);
        dict.insert("amount", ve.amount);

        dict.into_shared()
    }
}

lazy_static! {
    static ref DEFAULT_ESTIMATES: HashMap<String, HashMap<String, Phoneme>> = HashMap::from([
        (
            "peak3".to_owned(),
            HashMap::from([
                (
                    "A".to_owned(),
                    Phoneme(vec![
                        DataPoint(18.0, 1.0),
                        DataPoint(41.0, 0.9),
                        DataPoint(85.0, 0.75),
                    ]),
                ),
                (
                    "E".to_owned(),
                    Phoneme(vec![
                        DataPoint(21.0, 1.0),
                        DataPoint(60.0, 0.75),
                        DataPoint(84.0, 0.65),
                    ]),
                ),
                (
                    "I".to_owned(),
                    Phoneme(vec![
                        DataPoint(21.0, 1.0),
                        DataPoint(42.0, 1.1),
                        DataPoint(84.0, 1.0),
                    ]),
                ),
                (
                    "O".to_owned(),
                    Phoneme(vec![
                        DataPoint(20.0, 1.0),
                        DataPoint(63.0, 0.9),
                        DataPoint(85.0, 0.8),
                    ]),
                ),
                (
                    "U".to_owned(),
                    Phoneme(vec![
                        DataPoint(19.0, 1.0),
                        DataPoint(47.0, 0.65),
                        DataPoint(84.0, 0.7),
                    ]),
                ),
            ]),
        ),
        (
            "peak4".to_owned(),
            HashMap::from([
                (
                    "A".to_owned(),
                    Phoneme(vec![
                        DataPoint(18.0, 1.0),
                        DataPoint(41.0, 0.9),
                        DataPoint(68.0, 0.7),
                        DataPoint(85.0, 0.55),
                    ]),
                ),
                (
                    "E".to_owned(),
                    Phoneme(vec![
                        DataPoint(22.0, 1.0),
                        DataPoint(43.0, 0.9),
                        DataPoint(66.0, 0.7),
                        DataPoint(84.0, 0.65)
                    ])
                ),
                (
                    "I".to_owned(),
                    Phoneme(vec![
                        DataPoint(21.0, 1.0),
                        DataPoint(42.0, 1.1),
                        DataPoint(60.0, 1.0),
                        DataPoint(84.0, 1.1)
                    ])
                ),
                (
                    "O".to_owned(),
                    Phoneme(vec![
                        DataPoint(20.0, 1.0),
                        DataPoint(39.0, 0.9),
                        DataPoint(63.0, 0.75),
                        DataPoint(85.0, 0.8)
                    ])
                ),
                (
                    "U".to_owned(),
                    Phoneme(vec![
                        DataPoint(20.0, 1.0),
                        DataPoint(39.0, 0.7),
                        DataPoint(65.0, 0.6),
                        DataPoint(84.0, 0.75)
                    ])
                )
            ]),
        ),
    ]);
    pub static ref PI2: f32 = 2.0 * std::f32::consts::PI;
    pub static ref INV_255: f32 = 1.0 / 255.0;
    pub static ref INV_32767: f32 = 1.0 / 32767.0;
    pub static ref INV_LOG10: f32 = 1.0 / (10.0 as f32).ln();
    pub static ref INV_DYNAMIC_RANGE: f32 = 1.0 / DYNAMIC_RANGE;
}

const VOWELS: [&str; 5] = ["A", "E", "I", "O", "U"];

#[derive(NativeClass)]
#[inherit(Reference)]
#[user_data(user_data::RwLockData<LipSync>)]
#[register_with(Self::register_lip_sync)]
pub struct LipSync {
    join_handle: Option<thread::JoinHandle<()>>,

    before_sample_array: Vec<f32>,
    peaks3_log: VecDeque<Vec<DataPoint>>, // TODO pretty sure these are just ring buffers
    peaks4_log: VecDeque<Vec<DataPoint>>,
    vowel_log: VecDeque<i32>,
    estimate_log: VecDeque<i32>,

    is_recording: bool,
}

#[methods]
impl LipSync {
    fn new(_owner: &Reference) -> Self {
        LipSync {
            join_handle: None,

            before_sample_array: vec![],
            peaks3_log: VecDeque::new(),
            peaks4_log: VecDeque::new(),
            vowel_log: VecDeque::from(vec![-1, -1, -1]),
            estimate_log: VecDeque::from(vec![-1, -1, -1]),

            is_recording: false,
        }
    }

    fn register_lip_sync(builder: &ClassBuilder<Self>) {
        builder.add_signal(Signal {
            name: &LIP_SYNC_UPDATED,
            args: &[SignalArgument {
                name: "result",
                default: Variant::from_dictionary(&Dictionary::default()),
                export_info: ExportInfo::new(VariantType::Dictionary),
                usage: PropertyUsage::DEFAULT,
            }],
        });

        builder.add_signal(Signal {
            name: &LIP_SYNC_PANICKED,
            args: &[SignalArgument {
                name: "message",
                default: Variant::from_str("invalid error"),
                export_info: ExportInfo::new(VariantType::GodotString),
                usage: PropertyUsage::DEFAULT,
            }],
        });
    }

    fn get_peaks(&self, data: &[f32], threshold: f32) -> Vec<DataPoint> {
        let n = data.len() - 1;
        let mut i = 1;
        let mut out = vec![];
        let mut div = 1.0;
        while i < n {
            if data[i] > threshold && data[i] > data[i - 1] && data[i] > data[i + 1] {
                if out.len() > 0 {
                    out.push(DataPoint(i as f32, data[i] * div));
                } else {
                    out.push(DataPoint(i as f32, 1.0));
                    div = 1.0 / data[i];
                }
            }
            i += 1;
        }
        out
    }
    fn get_peaks_average(&mut self, size: usize) -> Vec<DataPoint> {
        let mut out = vec![];
        let mut i = 1;
        let mut j = 0;
        let mut div = 1.0;
        match size {
            3 => {
                out = self.peaks3_log[0].clone();
                while i < self.peaks3_log.len() {
                    j = 0;
                    while j < out.len() {
                        out[j].0 += self.peaks3_log[i][j].0;
                        out[j].1 += self.peaks3_log[i][j].1;
                        j += 1;
                    }
                    i += 1;
                }
                div = 1.0 / self.peaks3_log.len() as f32;
            }
            4 => {
                out = self.peaks4_log[0].clone();
                while i < self.peaks4_log.len() {
                    j = 0;
                    while j < out.len() {
                        out[j].0 += self.peaks4_log[i][j].0;
                        out[j].1 += self.peaks4_log[i][j].1;
                        j += 1;
                    }
                    i += 1;
                }
                div = 1.0 / self.peaks4_log.len() as f32;
            }
            _ => {}
        }

        for k in out.iter_mut() {
            *k *= div;
        }

        out
    }

    fn get_distance_from_db(&self, data: &[DataPoint]) -> Vec<f32> {
        let mut out = vec![];

        let mut dist = 0.0;

        let mut peak_est: &HashMap<String, Phoneme> = match data.len() {
            3 => &DEFAULT_ESTIMATES["peak3"],
            4 => &DEFAULT_ESTIMATES["peak4"],
            _ => {
                return out;
            }
        };

        for i in VOWELS {
            dist = 0.0;
            for j in 0..data.len() {
                let est = &peak_est[i][j];
                dist += (est.0 - data[j].0).abs() * *INV_255 + (est.1 - data[j].1);
            }
            out.push(dist);
        }

        out
    }

    fn push_peaks(&mut self, data: &[DataPoint]) {
        match data.len() {
            3 => {
                if self.peaks3_log.len() < 3 {
                    self.peaks3_log.push_back(data.to_owned());
                } else {
                    self.peaks3_log.push_front(data.to_owned());
                    self.peaks3_log.pop_back();
                }
            }
            4 => {
                if self.peaks4_log.len() < 3 {
                    self.peaks4_log.push_back(data.to_owned());
                } else {
                    self.peaks4_log.push_front(data.to_owned());
                    self.peaks4_log.pop_back();
                }
            }
            _ => godot_print!("push_peaks encountered invalid data"),
        }
    }

    fn estimate_vowel(&mut self, data: &[f32]) -> i32 {
        let peaks = self.get_peaks(data, 0.1);
        if peaks.len() != 3 && peaks.len() != 4 {
            return -1;
        }

        self.push_peaks(peaks.as_slice());

        let peaks_ave = self.get_peaks_average(peaks.len());
        let distance_vowel = self.get_distance_from_db(peaks_ave.as_slice());

        let mut i = 1;
        let mut min_distance = distance_vowel[0];
        let mut min_idx = 0;
        while i < UPDATE_FRAME as usize {
            let dist = distance_vowel[i];
            if dist < min_distance {
                min_distance = dist;
                min_idx = i as i32;
            }
            i += 1;
        }

        min_idx
    }

    fn get_vowel(&mut self, data: &[f32], amount: f32) -> VowelEstimate {
        let current = self.estimate_vowel(data);

        let f_vowel = self.vowel_log[0];

        if self.vowel_log[0] != -1 {
            if amount < 0.5 {
                return VowelEstimate::new(current, f_vowel, amount);
            }
        }

        if self.vowel_log.len() > 2 {
            if current == self.estimate_log[0] {
                if current != -1 {
                    return VowelEstimate::new(current, current, amount);
                }
            } else {
                if f_vowel != -1 {
                    return VowelEstimate::new(current, f_vowel, amount);
                }
            }
        }

        return VowelEstimate::new(current, rand::thread_rng().gen_range(0..5), amount);
    }

    fn push_vowel(&mut self, vowel: i32) {
        if self.vowel_log.len() < 3 {
            self.vowel_log.push_back(vowel);
        } else {
            self.vowel_log.push_front(vowel);
            self.vowel_log.pop_back();
        }
    }

    fn push_estimate(&mut self, vowel: i32) {
        if self.estimate_log.len() < 3 {
            self.estimate_log.push_back(vowel);
        } else {
            self.estimate_log.push_front(vowel);
            self.estimate_log.pop_back();
        }
    }

    #[export]
    pub fn update(&mut self, owner: &Reference, stream: TypedArray<u8>) {
        let samples = read_16_bit_samples(stream);
        let rms = rms(samples.as_slice());
        if samples.len() >= FFT_SAMPLES {
            let mut data = samples[..FFT_SAMPLES].to_vec();
            hamming(data.as_mut_slice());
            rfft(data.as_mut_slice(), false, true);
            data = data[..(FFT_SAMPLES as f32 * 0.5) as usize].to_vec();
            if self.before_sample_array.len() > 0 {
                smoothing(data.as_mut_slice(), self.before_sample_array.as_slice());
            }
            self.before_sample_array = data.clone();
            filter(data.as_mut_slice(), 10, 95);
            for i in data.iter_mut() {
                *i = i.powi(2).ln() * *INV_LOG10;
            }
            normalize(data.as_mut_slice());
            rfft(data.as_mut_slice(), true, false);
            lifter(data.as_mut_slice(), 26);
            rfft(data.as_mut_slice(), false, false);
            data = data[..(FFT_SAMPLES as f32 * 0.25) as usize].to_vec();
            normalize(data.as_mut_slice());
            for i in data.iter_mut() {
                *i = i.powi(2);
            }
            normalize(data.as_mut_slice());
            let nrm_rms = DYNAMIC_RANGE.min((rms + DYNAMIC_RANGE).max(0.0));
            for i in data.iter_mut() {
                *i = *i * nrm_rms * *INV_DYNAMIC_RANGE;
            }
            let amount = inverse_lerp(-DYNAMIC_RANGE, 0.0, rms).clamp(0.0, 1.0);
            let current_vowel = self.get_vowel(data.as_slice(), amount);
            self.push_estimate(current_vowel.estimate);
            self.push_vowel(current_vowel.vowel);

            owner.emit_signal(
                "lip_sync_updated",
                &[Variant::from_dictionary(&Dictionary::from(current_vowel))],
            );
        }
    }
}

fn read_16_bit_samples(stream: TypedArray<u8>) -> Vec<f32> {
    let mut res = vec![];

    let mut i = 0;
    while i < stream.len() {
        let b0 = stream.get(i);
        let b1 = stream.get(i + 1);

        let mut u = ((b0 as u16) << 8) | b1 as u16;

        u = (u + 32768) & 0xffff;

        let s = (u - 32768) as f32 / 32768.0;

        res.push(s);

        i += 2;
    }

    res
}
