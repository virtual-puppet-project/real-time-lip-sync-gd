#[derive(Clone)]
pub struct LipSyncInfo {
    pub index: i64,
    pub phoneme: String,
    pub volume: f64,
    pub raw_volume: f64,
    pub distance: f64,
}

impl Default for LipSyncInfo {
    fn default() -> Self {
        LipSyncInfo {
            index: 0,
            phoneme: "Default".to_owned(),
            volume: 0.0,
            raw_volume: 0.0,
            distance: 0.0,
        }
    }
}

impl LipSyncInfo {
    pub fn new(index: i64, phoneme: String, volume: f64, raw_volume: f64, distance: f64) -> Self {
        LipSyncInfo {
            index,
            phoneme,
            volume,
            raw_volume,
            distance,
        }
    }
}
