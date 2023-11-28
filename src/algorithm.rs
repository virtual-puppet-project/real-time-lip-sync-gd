use std::boxed::Box;

use crate::model::{DataPoint, INV_LOG10, PI2};

pub fn rms(data: &[f32]) -> f32 {
    let mut rms: f32 = 0.0;

    for i in data.iter() {
        rms += i.powi(2);
    }

    rms = (rms / data.len() as f32).sqrt();
    rms = 20.0 * (rms.ln() * *INV_LOG10);

    rms
}

pub fn normalize(data: &mut [f32]) {
    let mut v_max: f32 = 0.0;
    let mut v_min: f32 = 0.0;

    for i in data.iter() {
        v_max = v_max.max(*i);
        v_min = v_min.min(*i);
    }

    let diff = v_max - v_min;
    let d: f32 = if diff != 0.0 { 1.0 / diff } else { 1.0 };
    for i in data.iter_mut() {
        *i = (*i - v_min) * d;
    }
}

pub fn smoothing(data: &mut [f32], before: &[f32]) {
    let n = data.len();
    for i in 0..n {
        data[i] = (data[i] + before[i]) * 0.5;
    }
}

pub fn hamming(data: &mut [f32]) {
    let n = data.len();
    for i in data.iter_mut() {
        let h = 0.54 - 0.46 * (*PI2 * *i / (n as f32 - 1.0));
        *i = *i * h;
    }
    data[0] = 0.0;
    data[n - 1] = 0.0;
}

pub fn rfft(data: &mut [f32], reverse: bool, positive: bool) {
    let n = data.len();
    let mut cmp_vec = vec![];
    for i in data.iter() {
        let dp = DataPoint(*i, 0.0);
        cmp_vec.push(Box::new(dp));
    }
    fft(cmp_vec.as_mut_slice(), reverse);
    if positive {
        for i in 0..n {
            data[i] = cmp_vec[i].0.abs();
        }
    } else {
        for i in 0..n {
            data[i] = cmp_vec[i].0;
        }
    }
    if reverse {
        let inv_n: f32 = 1.0 / n as f32;
        for i in data {
            *i *= inv_n;
        }
    }
}

pub fn fft(data: &mut [Box<DataPoint>], reverse: bool) {
    let n = data.len();
    if n == 1 {
        return;
    }

    let mut b = vec![];
    let mut c = vec![];
    for i in 0..n {
        if i % 2 == 0 {
            b.push(data[i].clone());
        } else if i % 2 == 1 {
            c.push(data[i].clone());
        }
    }
    fft(b.as_mut_slice(), reverse);
    fft(c.as_mut_slice(), reverse);
    let circle = if reverse { -*PI2 } else { *PI2 };
    for i in 0..n {
        // TODO this doesn't feel correct
        *data[i] = *b[i % (n / 2)].clone()
            + *c[i % (n / 2)].clone() * (DataPoint(0.0, circle * i as f32 / n as f32)).exp()
    }
}

pub fn lifter(data: &mut [f32], level: i32) {
    let i_min = level;
    let i_max = data.len() as i32 - 1 - level;
    for i in 0..data.len() as i32 {
        if i > i_min && i <= i_max {
            data[i as usize] = 0.0;
        }
    }
}

pub fn filter(data: &mut [f32], lowcut: i32, highcut: i32) {
    let mut minimum = data[0];
    for i in data.iter() {
        minimum = minimum.min(*i);
    }

    if minimum == 0.0 {
        minimum = 0.000001
    }

    for i in data {
        if *i <= lowcut as f32 || *i >= highcut as f32 {
            *i = minimum;
        }
    }
}

pub fn lerp(a: f32, b: f32, f: f32) -> f32 {
    // l = a + f * (b - a)
    a + f * (b - a)
}

pub fn inverse_lerp(a: f32, b: f32, l: f32) -> f32 {
    // l = a + f * (b - a)
    // l - a = f * (b - a)
    // (l - a)/(b - a) = f
    (l - a) / (b - a)
}
