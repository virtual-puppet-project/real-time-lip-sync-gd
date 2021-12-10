use std::f64::consts::PI;

pub fn get_max_value(array: &[f64]) -> f64 {
    let mut max: f64 = 0.0;
    for i in array {
        let i_abs = i.abs();
        if i_abs > max {
            max = i_abs;
        }
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

// TODO this could be VecDeque
pub fn copy_ring_buffer(input: &[f64], output: &mut Vec<f64>, start_src_index: usize) {
    let len = input.len();
    for i in 0..len {
        output.push(input[((start_src_index + i) % len) as usize]);
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

// TODO seems like the data coming in is stored as i64, not f64
// Data will always be pre-populated because this is copied from a ringbuffer
pub fn low_pass_filter(data: &mut [f64], sample_rate: f64, cutoff: f64, range: f64) {
    let cutoff = cutoff / sample_rate;
    let range = range / sample_rate;

    let tmp = data.to_vec();

    let mut n = (3.1 / range).round() as i64;
    if (n + 1) % 2 == 0 {
        n += 1;
    }

    // let mut b = Vec::<f64>::with_capacity(n as usize);
    let mut b = vec![0.0; n as usize];

    for i in 0..n as i64 {
        let x = (i - (n - 1)) as f64 / 2.0;
        let ang = 2.0 * PI * cutoff * x;
        b[i as usize] = if ang != 0.0 {
            2.0 * cutoff * ang.sin() / ang
        } else {
            0.0
        };
    }

    for i in 0..data.len() {
        for j in 0..n as usize {
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
        (*output).extend(input);
    } else if sample_rate % target_sample_rate == 0 {
        let skip = sample_rate / target_sample_rate;
        let output_length: usize = input.len() / skip as usize;
        (*output).resize(output_length, 0.0);
        down_sample_1(input, output, output_length, skip);
    } else {
        let df = (sample_rate / target_sample_rate) as f64;
        let output_length = (input.len() as f64 / df) as usize;
        (*output).resize(output_length, 0.0);
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

    spectrum.resize(n, 0.0);

    let mut spectrum_re = vec![0.0; n];
    let mut spectrum_im = vec![0.0; n];

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

    let mut even_re = vec![0.0; (n / 2) as usize];
    let mut even_im = vec![0.0; (n / 2) as usize];
    let mut odd_re = vec![0.0; (n / 2) as usize];
    let mut odd_im = vec![0.0; (n / 2) as usize];

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
        spectrum_im[n as usize / 2 + i] = ei - c.1;
    }
}

pub fn mel_filter_bank(
    spectrum: &[f64],
    mel_spectrum: &mut Vec<f64>,
    sample_rate: f64,
    mel_div: i64,
) {
    mel_spectrum.resize(mel_div as usize, 0.0);

    let f_max = sample_rate / 2.0;
    let mel_max = to_mel(f_max);
    let n_max = spectrum.len() as i64 / 2;
    let df = f_max / n_max as f64;
    let d_mel = mel_max / (mel_div + 1) as f64;

    for n in 0..mel_div as usize {
        let mel_begin = d_mel * n as f64;
        let mel_center = d_mel * (n as f64 + 1.0);
        let mel_end = d_mel * (n as f64 + 2.0);

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
                a = i as f64 / i_center as f64;
            } else {
                a = (i as f64 - i_center as f64) / i_center as f64;
            }
            sum += a * spectrum[i as usize];
        }
        mel_spectrum[n] = sum;
    }
}

const MEL_MAGIC: f64 = 1127.010480;

// TODO this calculation seems a bit off from the wikipedia formula
// https://en.wikipedia.org/wiki/Mel_scale
pub fn to_mel(hz: f64) -> f64 {
    MEL_MAGIC * (hz / 700.0 + 1.0).ln()
}

pub fn to_hz(mel: f64) -> f64 {
    700.0 * ((mel / MEL_MAGIC).exp() - 1.0)
}

pub fn dct(spectrum: &[f64], cepstrum: &mut Vec<f64>) {
    let len = spectrum.len();
    cepstrum.resize(len, 0.0);

    let a = PI / len as f64;
    for i in 0..len {
        let mut sum = 0.0;
        for j in 0..len {
            let ang = (j as f64 + 0.5) * i as f64 * a;
            sum += if ang.abs() > 0.0 {
                spectrum[j] * ang.cos()
            } else {
                0.0
            };
        }
        cepstrum[i] = sum;
    }
}

pub fn lerp_float(a: f64, b: f64, amount: f64) -> f64 {
    (a * (1.0 - amount)) + (b * amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_max_value_basic() {
        assert_eq!(get_max_value(vec![1.0, 2.0].as_slice()), 2.0);
    }

    #[test]
    fn test_get_max_value_with_negative() {
        assert_eq!(get_max_value(vec![1.0, -2.0].as_slice()), 2.0);
    }

    #[test]
    fn test_get_rms_volume() {
        assert_eq!(
            get_rms_volume(vec![1.0, 2.0, 3.0, 4.0, 5.0].as_slice()).round(),
            3.0
        );
    }

    #[test]
    fn test_copy_ring_buffer() {
        let input = vec![1.0, 2.0, 3.0];
        let mut output = Vec::with_capacity(input.len());

        copy_ring_buffer(input.as_slice(), &mut output, 2 as usize);
        assert_eq!(output, vec![3.0, 1.0, 2.0]);

        output = Vec::with_capacity(input.len());
        copy_ring_buffer(input.as_slice(), &mut output, 1 as usize);
        assert_eq!(output, vec![2.0, 3.0, 1.0]);
    }

    const LESS_THAN_EPSILON: f64 = f64::EPSILON * 0.9;

    #[test]
    fn test_normalize_less_than_epsilon() {
        let even_less = LESS_THAN_EPSILON * 0.9;
        let mut input = vec![LESS_THAN_EPSILON, even_less];
        normalize(&mut input);
        assert_eq!(input, vec![LESS_THAN_EPSILON, even_less]);
    }

    #[test]
    fn test_normalize_success() {
        let mut input = vec![1.0, 2.0];
        normalize(&mut input);
        assert_eq!(input, vec![0.5, 1.0]);
    }

    // TODO add normalize test with a low epsilon value and 1.0

    const CD_SAMPLE_RATE: f64 = 44100.0;
    const RECORDING_SAMPLE_RATE: f64 = 48000.0;
    const TARGET_CUTOFF: f64 = RECORDING_SAMPLE_RATE / 2.0;
    const TARGET_RANGE: f64 = RECORDING_SAMPLE_RATE / 4.0;

    #[test]
    fn test_low_pass_filter() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut rounded_output: Vec<f64> = vec![];
        for i in input {
            rounded_output.push(i.round());
        }

        assert_eq!(rounded_output, vec![9975.0, 19324.0, 28896.0]);
    }

    #[test]
    fn test_down_sample_noop() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            RECORDING_SAMPLE_RATE as i64,
        );

        assert_eq!(input, output);
    }

    #[test]
    fn test_down_sample_1() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            (CD_SAMPLE_RATE / 2.0) as i64,
        );

        assert_eq!(9975.0, output[0].round());
    }

    #[test]
    fn test_down_sample_2() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            (RECORDING_SAMPLE_RATE * 3.0) as i64,
            CD_SAMPLE_RATE as i64,
        );

        // TODO this doesn't seem correct
        assert_eq!(9975.0, output[0].round());
    }

    #[test]
    fn test_pre_emphasis() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            RECORDING_SAMPLE_RATE as i64,
        );

        pre_emphasis(output.as_mut_slice(), 0.97);

        let rounded_output: Vec<f64> = output.into_iter().map(f64::round).collect();

        assert_eq!(rounded_output, vec![9975.0, 9648.0, 10152.0]);
    }

    #[test]
    fn test_hamming_window() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            RECORDING_SAMPLE_RATE as i64,
        );

        pre_emphasis(output.as_mut_slice(), 0.97);

        hamming_window(output.as_mut_slice());

        let rounded_output: Vec<f64> = output.into_iter().map(f64::round).collect();

        assert_eq!(rounded_output, vec![798.0, 9648.0, 812.0]);
    }

    #[test]
    fn test_fft() {
        let mut input = vec![10000.0, 20000.0, 30000.0];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            RECORDING_SAMPLE_RATE as i64,
        );

        pre_emphasis(output.as_mut_slice(), 0.97);

        hamming_window(output.as_mut_slice());

        let mut spectrum: Vec<f64> = vec![];

        fft(output.as_slice(), &mut spectrum);

        let rounded_spectrum: Vec<f64> = spectrum.into_iter().map(f64::round).collect();

        assert_eq!(rounded_spectrum, vec![10446.0, 8850.0, 812.0]);
    }

    #[test]
    fn test_mel_filter_bank() {
        let mut input = vec![
            10000.0, 20000.0, 30000.0, 5000.0, 10000.0, 30000.0, 20000.0, 10000.0, 20000.0,
            30000.0, 20000.0, 10000.0,
        ];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            RECORDING_SAMPLE_RATE as i64,
        );

        pre_emphasis(output.as_mut_slice(), 0.97);

        hamming_window(output.as_mut_slice());

        let mut spectrum: Vec<f64> = vec![];

        fft(output.as_slice(), &mut spectrum);

        let mut mel_spectrum: Vec<f64> = vec![];
        mel_filter_bank(spectrum.as_slice(), &mut mel_spectrum, CD_SAMPLE_RATE, 12);

        let rounded_non_zero_output: Vec<f64> = mel_spectrum
            .into_iter()
            .filter_map(|v| if v != 0.0 { Some(v.round()) } else { None })
            .collect();

        assert_eq!(rounded_non_zero_output, vec![14109.0, 33861.0]);
    }

    const BASE_HZ: f64 = 161.0;
    const BASE_MEL: f64 = 233.0;

    #[test]
    fn test_mel_hz_conversion() {
        assert_eq!(to_hz(BASE_MEL).round(), BASE_HZ);
    }

    #[test]
    fn test_hz_to_mel_conversion() {
        assert_eq!(to_mel(BASE_HZ).round(), BASE_MEL);
    }

    #[test]
    fn test_mel_hz_round_trip() {
        assert_eq!(to_hz(to_mel(BASE_HZ)).round(), BASE_HZ);
    }

    #[test]
    fn test_dct() {
        let mut input = vec![
            10000.0, 20000.0, 30000.0, 5000.0, 10000.0, 30000.0, 20000.0, 10000.0, 20000.0,
            30000.0, 20000.0, 10000.0, 30000.0, 20000.0, 10000.0,
        ];
        low_pass_filter(
            input.as_mut_slice(),
            CD_SAMPLE_RATE,
            TARGET_CUTOFF,
            TARGET_RANGE,
        );

        let mut output: Vec<f64> = vec![];
        down_sample(
            input.as_slice(),
            &mut output,
            CD_SAMPLE_RATE as i64,
            RECORDING_SAMPLE_RATE as i64,
        );

        pre_emphasis(output.as_mut_slice(), 0.97);

        hamming_window(output.as_mut_slice());

        let mut spectrum: Vec<f64> = vec![];

        fft(output.as_slice(), &mut spectrum);

        let mut mel_spectrum: Vec<f64> = vec![];
        mel_filter_bank(spectrum.as_slice(), &mut mel_spectrum, CD_SAMPLE_RATE, 12);

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

        let rounded_cepstrum: Vec<f64> = mel_cepstrum.into_iter().map(f64::round).collect();

        assert_eq!(
            rounded_cepstrum,
            vec![0.0, -3.0, 3.0, -3.0, 3.0, -3.0, 2.0, -2.0, 2.0, -1.0, 1.0, 0.0]
        );
    }
}
