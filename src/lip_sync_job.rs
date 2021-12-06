#[derive(Default)]
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

    pub fn execute(&mut self) {}
}
