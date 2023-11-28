use crate::{algorithm::*, model::*};
use godot::prelude::*;
use rand::Rng;
use std::{
    collections::{HashMap, VecDeque},
    sync::mpsc,
    thread,
};

struct Job {
    before_sample_array: Vec<f32>,
    // TODO pretty sure these are just ring buffers
    peaks3_log: VecDeque<Vec<DataPoint>>,
    peaks4_log: VecDeque<Vec<DataPoint>>,
    vowel_log: VecDeque<i32>,
    estimate_log: VecDeque<i32>,
}

impl Job {
    pub fn new() -> Self {
        Job {
            before_sample_array: vec![],
            peaks3_log: VecDeque::new(),
            peaks4_log: VecDeque::new(),
            vowel_log: VecDeque::from(vec![-1, -1, -1]),
            estimate_log: VecDeque::from(vec![-1, -1, -1]),
        }
    }

    pub fn execute(&mut self, stream: &Array<f32>) -> Option<VowelEstimate> {
        // let mut data = Job::read_16_bit_samples(stream);
        let mut data = vec![];
        for i in stream.iter_shared() {
            data.push(i);
        }

        if data.len() < FFT_SAMPLES {
            godot_print!("Audio data size is too small, skipped!");
            return None;
        }

        let rms = rms(data.as_slice());

        data = data[..FFT_SAMPLES as usize].to_vec();
        hamming(data.as_mut_slice());
        rfft(data.as_mut_slice(), false, true);
        data = data[..((FFT_SAMPLES as f32 * 0.5) as usize) + 1].to_vec();
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
        data = data[..((FFT_SAMPLES as f32 * 0.25) as usize) + 1].to_vec();
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

        Some(current_vowel)
    }

    // TODO this is returning values that are not in range -1..1
    fn read_16_bit_samples(stream: &Array<u8>) -> Vec<f32> {
        let mut res = vec![];
        let mut i = 0;
        while i < stream.len() {
            let b0 = stream.get(i);
            let b1 = stream.get(i + 1);
            let mut u = b0 as u16 | ((b1 as u16) << 8);
            u = (u + 32768) & 0xffff;
            let s = (u - 32768) as f32 / 32768.0;
            res.push(s);
            i += 2;
        }
        res
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
        let mut j: usize;
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

        let mut dist: f32;

        let peak_est: &HashMap<String, Phoneme> = match data.len() {
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
        while i < FFT_SAMPLES as usize {
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
}

pub enum JobMessage {
    InputData(Array<f32>),
    OutputData(VowelEstimate),
    Shutdown,
}

unsafe impl Send for JobMessage {}

pub fn create_job() -> Option<(
    thread::JoinHandle<()>,
    mpsc::Sender<JobMessage>,
    mpsc::Receiver<JobMessage>,
)> {
    let (s1, r2) = mpsc::channel();
    let (s2, r1) = mpsc::channel();

    let mut job = Job::new();

    let builder = thread::Builder::new();
    match builder.spawn(move || loop {
        let new_data: Array<f32>;
        if let Ok(msg) = r1.recv() {
            match msg {
                JobMessage::InputData(d) => new_data = d,
                JobMessage::Shutdown => break,
                _ => {
                    godot_print!("Error when matching job data");
                    break;
                }
            }
        } else {
            godot_print!("Error when receiving job data");
            break;
        }

        if let Some(vowel) = job.execute(&new_data) {
            match s1.send(JobMessage::OutputData(vowel)) {
                Ok(_) => {}
                Err(e) => {
                    godot_print!("Error when sending output from job: {:?}", e);
                    break;
                }
            }
        }
    }) {
        Ok(v) => return Some((v, s2, r2)),
        Err(_) => None,
    }
}
