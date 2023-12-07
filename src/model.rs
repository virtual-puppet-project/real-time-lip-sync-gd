use godot::prelude::*;
use lazy_static::lazy_static;
use std::{
    collections::{HashMap, VecDeque},
    ops::{Add, Div, Index, Mul, MulAssign},
};

pub const FFT_SAMPLES: usize = 1024;
// pub const UPDATE_FRAME: usize = 5;
pub const DYNAMIC_RANGE: f32 = 100.0;

pub const VOWELS: [&str; 5] = ["A", "E", "I", "O", "U"];

lazy_static! {
    pub static ref DEFAULT_ESTIMATES: HashMap<String, HashMap<String, Phoneme>> = HashMap::from([
        (
            "peak3".to_owned(),
            HashMap::from([
                (
                    "A".to_owned(),
                    Phoneme(vec![
                        DataPoint(18.0, 1.0),
                        DataPoint(41.0, 0.9),
                        DataPoint(85.0, 0.75),
                    ]),
                ),
                (
                    "E".to_owned(),
                    Phoneme(vec![
                        DataPoint(21.0, 1.0),
                        DataPoint(60.0, 0.75),
                        DataPoint(84.0, 0.65),
                    ]),
                ),
                (
                    "I".to_owned(),
                    Phoneme(vec![
                        DataPoint(21.0, 1.0),
                        DataPoint(42.0, 1.1),
                        DataPoint(84.0, 1.0),
                    ]),
                ),
                (
                    "O".to_owned(),
                    Phoneme(vec![
                        DataPoint(20.0, 1.0),
                        DataPoint(63.0, 0.9),
                        DataPoint(85.0, 0.8),
                    ]),
                ),
                (
                    "U".to_owned(),
                    Phoneme(vec![
                        DataPoint(19.0, 1.0),
                        DataPoint(47.0, 0.65),
                        DataPoint(84.0, 0.7),
                    ]),
                ),
            ]),
        ),
        (
            "peak4".to_owned(),
            HashMap::from([
                (
                    "A".to_owned(),
                    Phoneme(vec![
                        DataPoint(18.0, 1.0),
                        DataPoint(41.0, 0.9),
                        DataPoint(68.0, 0.7),
                        DataPoint(85.0, 0.55),
                    ]),
                ),
                (
                    "E".to_owned(),
                    Phoneme(vec![
                        DataPoint(22.0, 1.0),
                        DataPoint(43.0, 0.9),
                        DataPoint(66.0, 0.7),
                        DataPoint(84.0, 0.65)
                    ])
                ),
                (
                    "I".to_owned(),
                    Phoneme(vec![
                        DataPoint(21.0, 1.0),
                        DataPoint(42.0, 1.1),
                        DataPoint(60.0, 1.0),
                        DataPoint(84.0, 1.1)
                    ])
                ),
                (
                    "O".to_owned(),
                    Phoneme(vec![
                        DataPoint(20.0, 1.0),
                        DataPoint(39.0, 0.9),
                        DataPoint(63.0, 0.75),
                        DataPoint(85.0, 0.8)
                    ])
                ),
                (
                    "U".to_owned(),
                    Phoneme(vec![
                        DataPoint(20.0, 1.0),
                        DataPoint(39.0, 0.7),
                        DataPoint(65.0, 0.6),
                        DataPoint(84.0, 0.75)
                    ])
                )
            ]),
        ),
    ]);
    pub static ref PI2: f32 = 2.0 * std::f32::consts::PI;
    pub static ref INV_255: f32 = 1.0 / 255.0;
    pub static ref INV_32767: f32 = 1.0 / 32767.0;
    pub static ref INV_LOG10: f32 = 1.0 / (10.0 as f32).ln();
    pub static ref INV_DYNAMIC_RANGE: f32 = 1.0 / DYNAMIC_RANGE;
}

#[derive(Debug, PartialEq, Clone)]
pub struct DataPoint(pub f32, pub f32);

impl DataPoint {
    pub fn exp(self) -> DataPoint {
        let e = self.0.exp();

        DataPoint(e * self.1.cos(), e * self.1.sin())
    }

    pub fn zero() -> DataPoint {
        DataPoint(0.0, 0.0)
    }
}

impl Add for DataPoint {
    type Output = DataPoint;
    fn add(self, other: DataPoint) -> DataPoint {
        DataPoint(self.0 + other.0, self.1 + other.1)
    }
}

impl Mul<DataPoint> for DataPoint {
    type Output = DataPoint;
    fn mul(self, other: DataPoint) -> DataPoint {
        let r = self.0 * other.0 - self.1 * other.1;
        let i = self.0 * other.0 + self.1 * other.1;

        DataPoint(r, i)
    }
}

impl MulAssign<f32> for DataPoint {
    fn mul_assign(&mut self, other: f32) {
        self.0 *= other;
        self.1 *= other;
    }
}

impl Div for DataPoint {
    type Output = DataPoint;
    fn div(self, other: DataPoint) -> DataPoint {
        let r = self.0 * other.0 + self.1 * other.1;
        let i = self.1 * other.0 - self.1 * other.1;
        let d = other.0 * other.0 + other.1 * other.1;

        DataPoint(r / d, i / d)
    }
}

#[derive(Debug, PartialEq)]
pub struct Phoneme(Vec<DataPoint>);

impl Index<usize> for Phoneme {
    type Output = DataPoint;
    fn index(&self, idx: usize) -> &DataPoint {
        &self.0[idx]
    }
}

#[derive(Debug)]
pub struct VowelEstimate {
    pub estimate: i32,
    pub vowel: i32,
    pub amount: f32,
}

impl VowelEstimate {
    pub fn new(estimate: i32, vowel: i32, amount: f32) -> Self {
        VowelEstimate {
            estimate,
            vowel,
            amount,
        }
    }
}

impl From<VowelEstimate> for Dictionary {
    fn from(ve: VowelEstimate) -> Self {
        let mut dict = Dictionary::new();

        dict.insert("estimate", ve.estimate);
        dict.insert("vowel", ve.vowel);
        dict.insert("amount", ve.amount);

        dict
    }
}
