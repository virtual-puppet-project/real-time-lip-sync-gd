use crate::algorithm::*;

#[derive(Default, Clone)]
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
    pub mfcc: Vec<f64>,
    pub phonemes: Vec<f64>,
    pub result: Vec<LipSyncJobResult>,
}

impl LipSyncJob {
    pub fn new() -> Self {
        LipSyncJob::default()
    }

    pub fn execute(&mut self) {
        let volume = get_rms_volume(self.input.as_slice());
        if volume < self.volume_thresh {
            let mut res_1 = self.result[0].clone();
            res_1.index = -1;
            res_1.volume = volume;
            res_1.distance = f64::MAX;
            self.result[0] = res_1;
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

        for i in 1..13 {
            self.mfcc[i - 1] = mel_cepstrum[i];
        }

        let mut res = LipSyncJobResult::default();
        res.volume = volume;
        self.get_vowel(&mut res);
        self.result[0] = res;
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
        let mut distance: f64 = 0.0;
        let offset = index * 12;
        for i in 0..self.mfcc.len() {
            distance += (self.mfcc[i] - self.phonemes[i + offset]).abs();
        }

        distance
    }
}
