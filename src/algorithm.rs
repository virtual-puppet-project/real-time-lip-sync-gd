use std::f64::consts::PI;

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
    let max = get_max_value(&array);
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

    let tmp = data.to_vec();

    let mut n = (3.1 / range).round();
    if (n + 1.0) % 2.0 == 0.0 {
        n += 1.0;
    }

    let mut b = Vec::<f64>::with_capacity(n as usize);

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

pub fn down_sample(
    input: &[f64],
    output: &mut Vec<f64>,
    sample_rate: i64,
    target_sample_rate: i64,
) {
    if sample_rate <= target_sample_rate {
        *output = input.to_vec();
    } else if sample_rate % target_sample_rate == 0 {
        let skip = sample_rate / target_sample_rate;
        let output_length: usize = input.len() / skip as usize;
        *output = Vec::with_capacity(output_length);
        down_sample_1(input, output, output_length, skip);
    } else {
        let df = (sample_rate / target_sample_rate) as f64;
        let output_length = (input.len() as f64 / df) as usize;
        *output = Vec::with_capacity(output_length);
        down_sample_2(input, input.len(), output, output_length, df);
    }
}

pub fn down_sample_1(input: &[f64], output: &mut [f64], output_length: usize, skip: i64) {
    for i in 0..output_length {
        output[i] = input[i * skip as usize];
    }
}

pub fn down_sample_2(
    input: &[f64],
    input_length: usize,
    output: &mut [f64],
    output_length: usize,
    df: f64,
) {
    for j in 0..output_length {
        let index = df * j as f64;
        let i0 = index.floor() as i64;
        let i1 = i0.min(input_length as i64 - 1);
        let t = index - i0 as f64;
        let x0 = input[i0 as usize];
        let x1 = input[i1 as usize];
        output[j] = lerp_float(x0, x1, t);
    }
}

pub fn pre_emphasis(data: &mut [f64], p: f64) {
    let tmp = data.to_vec();
    for i in 1..data.len() {
        data[i] = tmp[i] - p * tmp[i - 1];
    }
}

pub fn hamming_window(array: &mut [f64]) {
    let len = array.len();
    for i in 0..len {
        let x = i as f64 / (len - 1) as f64;
        array[i] *= 0.54 - 0.46 * (2.0 * PI * x).cos();
    }
}

pub struct Float2(f64, f64);

pub fn fft(data: &[f64], spectrum: &mut Vec<f64>) {
    let n = data.len();
    *spectrum = Vec::new();

    let mut spectrum_re = Vec::<f64>::with_capacity(n);
    let mut spectrum_im = Vec::<f64>::with_capacity(n);

    for i in 0..n {
        spectrum_re[i] = data[i];
    }
    _fft(spectrum_re.as_mut_slice(), spectrum_im.as_mut_slice(), n);

    for i in 0..n {
        let re = spectrum_re[i];
        let im = spectrum_im[i];
        spectrum[i] = (im - re).abs(); // TODO this uses math.Length(new float2(re, im)) in the Unity impl
    }
}

fn _fft(spectrum_re: &mut [f64], spectrum_im: &mut [f64], n: usize) {
    if n < 2 {
        return;
    }

    let mut even_re = Vec::<f64>::with_capacity((n / 2) as usize);
    let mut even_im = Vec::<f64>::with_capacity((n / 2) as usize);
    let mut odd_re = Vec::<f64>::with_capacity((n / 2) as usize);
    let mut odd_im = Vec::<f64>::with_capacity((n / 2) as usize);

    for i in 0..(n / 2) {
        let j = (i * 2) as usize;
        even_re[i] = spectrum_re[j];
        even_im[i] = spectrum_im[j];
        odd_re[i] = spectrum_re[j + 1];
        odd_im[i] = spectrum_im[j + 1];
    }

    _fft(even_re.as_mut_slice(), even_im.as_mut_slice(), n / 2);
    _fft(odd_re.as_mut_slice(), odd_im.as_mut_slice(), n / 2);

    for i in 0..(n / 2) {
        let er = even_re[i];
        let ei = even_im[i];
        let or = odd_re[i];
        let oi = odd_im[i];
        let theta = -2.0 * PI * i as f64 / n as f64;
        let cx = theta.cos();
        let cy = theta.sin();
        let c = Float2(cx * or - cy * oi, cx * oi + cy * or);
        spectrum_re[i] = er + c.0;
        spectrum_im[i] = ei + c.1;
        spectrum_re[n as usize / 2 + i] = er - c.0;
        spectrum_im[i as usize / 2 + i] = ei - c.1;
    }
}

pub fn mel_filter_bank(
    spectrum: &[f64],
    mel_spectrum: &mut Vec<f64>,
    sample_rate: f64,
    mel_div: i64,
) {
    *mel_spectrum = Vec::with_capacity(mel_div as usize);

    let f_max = sample_rate / 2.0;
    let mel_max = to_mel(f_max);
    let n_max = spectrum.len() as i64;
    let df = f_max / n_max as f64;
    let d_mel = mel_max / (mel_div + 1) as f64;

    for i in 0..mel_div {
        let n = i as f64;
        let mel_begin = d_mel * n;
        let mel_center = d_mel * (n + 1.0);
        let mel_end = d_mel * (n + 2.0);

        let f_begin = to_hz(mel_begin);
        let f_center = to_hz(mel_center);
        let f_end = to_hz(mel_end);

        let i_begin = (f_begin / df).round() as i64;
        let i_center = (f_center / df).round() as i64;
        let i_end = (f_end / df).round() as i64;

        let mut sum = 0.0 as f64;
        for i in (i_begin + 1)..i_end {
            let a: f64;
            if i < i_center {
                a = (i / i_center) as f64;
            } else {
                a = ((i - i_center) / i_center) as f64;
            }
            sum += a * spectrum[i as usize];
        }
        mel_spectrum[n as usize] = sum;
    }
}

const MEL_MAGIC: f64 = 1127.010480;

pub fn to_mel(hz: f64) -> f64 {
    MEL_MAGIC * (hz / 700.0 + 1.0).ln()
}

pub fn to_hz(mel: f64) -> f64 {
    700.0 * ((mel / MEL_MAGIC).exp() - 1.0)
}

pub fn dct(spectrum: &[f64], cepstrum: &mut Vec<f64>) {
    let len = spectrum.len();
    *cepstrum = Vec::with_capacity(len);

    let a = PI / len as f64;
    for i in 0..len {
        let mut sum = 0.0;
        for j in 0..len {
            let ang = (j as f64 + 0.5) * i as f64 * a;
            sum += spectrum[j] * ang.cos();
        }
        cepstrum[i] = sum;
    }
}

pub fn lerp_float(a: f64, b: f64, amount: f64) -> f64 {
    (a * (1.0 - amount)) + (b * amount)
}
