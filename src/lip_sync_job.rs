use std::sync::{Arc, Mutex};

use crate::algorithm::*;

#[derive(Default, Clone, Debug)]
pub struct LipSyncJobResult {
    pub index: i64,
    pub volume: f64,
    pub distance: f64,
}

#[derive(Default)]
pub struct LipSyncJob {
    pub input: Vec<f64>,
    pub start_index: i64,
    pub output_sample_rate: i64,
    pub target_sample_rate: i64,
    pub mel_filter_bank_channels: i64,
    pub volume_thresh: f64,
    pub mfcc: Arc<Mutex<Vec<f64>>>,
    pub phonemes: Vec<f64>,
    pub result: Arc<Mutex<LipSyncJobResult>>,

    pub is_complete: bool,
    pub should_stop: bool,
}

impl LipSyncJob {
    pub fn new() -> Self {
        let mut lsj = LipSyncJob::default();
        lsj.is_complete = true;

        lsj
    }

    pub fn execute(&mut self) {
        let volume = get_rms_volume(self.input.as_slice());
        if volume < self.volume_thresh {
            // TODO actually handle the error
            let mut shared_res = self.result.lock().expect(
                "Unable to lock result[0] in lip_sync_job when volume is less than volume_thresh",
            );
            let mut res_1 = shared_res.clone();
            res_1.index = -1;
            res_1.volume = volume;
            res_1.distance = f64::MAX;
            shared_res.index = res_1.index;
            shared_res.volume = res_1.volume;
            shared_res.distance = res_1.distance;
            return;
        }

        let mut buffer = Vec::<f64>::with_capacity(self.input.len());
        copy_ring_buffer(
            self.input.as_slice(),
            &mut buffer,
            self.start_index as usize,
        );

        let cutoff = self.target_sample_rate / 2;
        let range = self.target_sample_rate / 2;
        low_pass_filter(
            buffer.as_mut_slice(),
            self.output_sample_rate as f64,
            cutoff as f64,
            range as f64,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            self.input.as_slice(),
            &mut output,
            self.output_sample_rate,
            self.target_sample_rate,
        );

        pre_emphasis(output.as_mut_slice(), 0.97);

        hamming_window(output.as_mut_slice());

        let mut spectrum: Vec<f64> = vec![];

        fft(output.as_slice(), &mut spectrum);

        let mut mel_spectrum: Vec<f64> = vec![];
        mel_filter_bank(
            spectrum.as_slice(),
            &mut mel_spectrum,
            self.target_sample_rate as f64,
            self.mel_filter_bank_channels,
        );

        if mel_spectrum.len() == 0 {
            // TODO actually handle the error
            let mut shared_res = self.result.lock().expect(
                "Unable to lock result[0] in lip_sync_job when volume is less than volume_thresh",
            );
            let mut res_1 = shared_res.clone();
            res_1.index = -1;
            res_1.volume = volume;
            res_1.distance = f64::MAX;
            shared_res.index = res_1.index;
            shared_res.volume = res_1.volume;
            shared_res.distance = res_1.distance;
            return;
        }

        for i in 0..mel_spectrum.len() {
            let v = mel_spectrum[i];
            mel_spectrum[i] = if v > 0.0 {
                mel_spectrum[i].log10()
            } else {
                0.0
            };
        }

        let mut mel_cepstrum = vec![];
        dct(mel_spectrum.as_slice(), &mut mel_cepstrum);

        if mel_cepstrum.len() == 0 {
            // TODO actually handle the error
            let mut shared_res = self.result.lock().expect(
                "Unable to lock result[0] in lip_sync_job when volume is less than volume_thresh",
            );
            let mut res_1 = shared_res.clone();
            res_1.index = -1;
            res_1.volume = volume;
            res_1.distance = f64::MAX;
            shared_res.index = res_1.index;
            shared_res.volume = res_1.volume;
            shared_res.distance = res_1.distance;
            return;
        }

        let mut shared_mfcc = self.mfcc.lock().expect("Unable to lock mfcc");
        for i in 1..13 {
            shared_mfcc[i - 1] = mel_cepstrum[i];
        }
        drop(shared_mfcc);

        let mut res = LipSyncJobResult::default();
        res.volume = volume;
        self.get_vowel(&mut res);
        let mut shared_res = self
            .result
            .lock()
            .expect("Unable to lock result[0] in lip_sync_job when assigning first result");
        shared_res.index = res.index;
        shared_res.volume = res.volume;
        shared_res.distance = res.distance;
        dbg!(shared_res);
    }

    fn get_vowel(&self, result: &mut LipSyncJobResult) {
        result.distance = f64::MAX;
        let n = self.phonemes.len() / 12;
        for i in 0..n {
            let distance = self.calc_total_distance(i);
            if distance < result.distance {
                result.index = i as i64;
                result.distance = distance;
            }
        }
    }

    fn calc_total_distance(&self, index: usize) -> f64 {
        let shared_mfcc = self.mfcc.lock().expect("Unable to lock mfcc");
        let mut distance: f64 = 0.0;
        let offset = index * 12;
        for i in 0..shared_mfcc.len() {
            distance += (shared_mfcc[i] - self.phonemes[i + offset]).abs();
        }

        distance
    }
}
