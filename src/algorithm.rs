use std::f64::consts::PI;

pub struct Algorithm;

impl Algorithm {
    pub fn get_max_value(array: &[f64]) -> f64 {
        let mut max: f64 = 0.0;
        for i in array {
            let i_abs = i.abs();
            if i_abs > max {
                max = i_abs;
            }
            // max = std::cmp::max(max, i.abs());
        }

        max
    }

    pub fn get_rms_volume(array: &[f64]) -> f64 {
        let mut average: f64 = 0.0;
        for i in array {
            average += i.powi(2);
        }

        (average / array.len() as f64).sqrt()
    }

    pub fn copy_ring_buffer(input: &[f64], output: &mut [f64], start_src_index: usize) {
        let len = input.len();
        for i in 0..len {
            output[i] = input[((start_src_index + i) % len) as usize];
        }
    }

    pub fn normalize(array: &mut Vec<f64>) {
        let max = Algorithm::get_max_value(&array);
        if max < f64::EPSILON {
            return;
        }
        for i in 0..array.len() {
            array[i] /= max;
        }
    }

    pub fn low_pass_filter(data: &mut [f64], sample_rate: f64, cutoff: f64, range: f64) {
        let cutoff = cutoff / sample_rate;
        let range = range / sample_rate;

        let mut tmp = data.to_vec();

        let mut n = (3.1 / range).round();
        if (n + 1.0) % 2.0 == 0.0 {
            n += 1.0;
        }

        let mut b = Vec::<f64>::new();

        let b_len = b.len();
        for i in 0..b_len {
            let x = (i - (b_len - 1)) as f64 / 2.0;
            let ang = 2.0 * PI * cutoff * x;
            b[i] = 2.0 * cutoff * ang.sin() / ang;
        }

        for i in 0..data.len() {
            for j in 0..b_len {
                if i as i64 - j as i64 >= 0 {
                    data[i] += b[j] * tmp[i - j];
                }
            }
        }
    }
}
