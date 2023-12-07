use godot::prelude::*;

pub fn print_max(sample_vec: Vec<f32>) {
    let mut max: f32 = 0.0;
    for i in sample_vec {
        max = max.max(i.abs());
    }
    godot_print!("{}", max);
}

pub fn print_min(sample_vec: Vec<f32>) {
    let mut min: f32 = 0.0;
    for i in sample_vec {
        min = min.min(i.abs());
    }
    godot_print!("{}", min);
}
