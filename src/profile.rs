#[derive(Default, Clone)]
pub struct MfccCalibrationData(Vec<f64>);

#[derive(Default)]
pub struct MfccData {
    pub name: String,
    pub mfcc_calibration_data_list: Vec<MfccCalibrationData>,
    pub mfcc_native_array: Vec<f64>,
}

impl MfccData {
    pub fn allocate(&self) {
        unimplemented!("Allocation happens when MfccData comes into scope")
    }

    pub fn deallocate(&self) {
        unimplemented!("Deallocation should happen automatically")
    }

    fn is_allocated(&self) -> bool {
        unimplemented!("mfcc_native_array will always be allocated when in scope")
    }

    pub fn add_calibration_data(&mut self, mfcc: Vec<f64>) {
        if mfcc.len() != 12 {
            println!("The length of the MFCC array should be 12");
            return;
        }

        self.mfcc_calibration_data_list
            .push(MfccCalibrationData(mfcc));
    }

    pub fn remove_old_calibration_data(&mut self, data_count: i64) {
        while self.mfcc_calibration_data_list.len() as i64 > data_count {
            self.mfcc_calibration_data_list.remove(0);
        }
    }

    pub fn update_native_array(&mut self) {
        if self.mfcc_calibration_data_list.len() == 0 {
            return;
        }

        for i in 0..12 {
            self.mfcc_native_array[i] = 0.0;
            for mfcc in self.mfcc_calibration_data_list.iter() {
                self.mfcc_native_array[i] += mfcc.0[i];
            }

            self.mfcc_native_array[i] /= self.mfcc_calibration_data_list.len() as f64;
        }
    }

    pub fn get_average(&self, i: usize) -> MfccCalibrationData {
        self.mfcc_calibration_data_list[i].clone()
    }
}

#[derive(Default)]
pub struct Profile {
    // The number of MFCC data to calculate the average MFCC values
    pub mfcc_data_count: i64,
    // The number of Mel Filter Bank channels
    pub mel_filter_bank_channels: i64,
    // Target sampling rate to apply downsampling
    pub target_sample_rate: i64,
    // Number of audio samples after downsampling is applied
    pub sample_count: i64,
    pub min_volume: f64,
    pub max_volume: f64,

    pub mfccs: Vec<MfccData>,
}

impl Profile {
    pub fn new() -> Self {
        Profile::default()
    }

    fn on_enable(&self) {
        unimplemented!("Unity-specific")
    }

    fn on_disable(&self) {
        unimplemented!("Unity-specific")
    }

    pub fn get_phoneme(&self, index: usize) -> String {
        if index >= self.mfccs.len() {
            "".to_string();
        }

        self.mfccs[index].name.clone()
    }

    pub fn add_mfcc(&mut self, name: String) {
        let mut data = MfccData::default();
        for _ in 0..self.mfcc_data_count {
            data.mfcc_calibration_data_list
                .push(MfccCalibrationData::default());
        }
        self.mfccs.push(data);
    }

    pub fn remove_mfcc(&mut self, index: usize) {
        if index >= self.mfccs.len() {
            return;
        }

        self.mfccs.remove(index);
    }

    pub fn update_mfcc(&mut self, index: usize, mfcc: Vec<f64>, calib: bool) {
        if index >= self.mfccs.len() {
            return;
        }

        let array = mfcc.clone();
        let data = &mut self.mfccs[index];
        data.add_calibration_data(array);
        data.remove_old_calibration_data(self.mfcc_data_count);

        if calib {
            data.update_native_array();
        }
    }

    // TODO this doesn't look like it gets the average, but this is how it is in the original
    pub fn get_averages(&mut self, index: usize) -> Vec<f64> {
        self.mfccs[index].mfcc_native_array.clone()
    }

    pub fn export(&self, path: String) {
        unimplemented!("used for precompiling lip sync data")
    }

    pub fn import(&self, path: String) {
        unimplemented!("used for loading lip sync data")
    }

    // TODO superceded by the more idiomatic new()
    pub fn create(path: String) -> Self {
        Profile::default()
    }
}