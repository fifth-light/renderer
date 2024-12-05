use std::{
    fmt::Debug,
    ops::{Add, Mul},
};

use crate::index::AssetIndex;

#[derive(Debug, Clone)]
pub struct AnimationKeyFrame<T: Debug + Clone> {
    pub time: f32,
    pub value: T,
}

#[derive(Debug, Clone)]
pub enum AnimationKeyFrames<T: Debug + Clone> {
    Linear(Vec<AnimationKeyFrame<T>>),
    Step(Vec<AnimationKeyFrame<T>>),
    // in, val, out
    CubicSpline(Vec<AnimationKeyFrame<(T, T, T)>>),
}

#[derive(Debug, Clone)]
pub enum AnimationSampler {
    Rotation(AnimationKeyFrames<[f32; 4]>),
    Translation(AnimationKeyFrames<[f32; 3]>),
    Scale(AnimationKeyFrames<[f32; 3]>),
}

#[derive(Debug, Clone)]
pub struct AnimationChannelAsset {
    pub sampler: AnimationSampler,
    pub length: f32,
    pub target_id: AssetIndex,
}

#[derive(Debug, Clone)]
pub struct AnimationAsset {
    pub name: Option<String>,
    pub channels: Vec<AnimationChannelAsset>,
}

pub trait Interpolate {
    fn linear(a: Self, b: Self, t: f32) -> Self;
    fn cubic_spline(vk: Self, bk: Self, vk_1: Self, ak_1: Self, t: f32, td: f32) -> Self;
}

impl<T> Interpolate for T
where
    T: Mul<T, Output = T> + Mul<f32, Output = T> + Add<T, Output = T>,
{
    fn linear(a: Self, b: Self, t: f32) -> Self {
        a * (1.0 - t) + b * t
    }

    fn cubic_spline(vk: Self, bk: Self, vk_1: Self, ak_1: Self, t: f32, td: f32) -> Self {
        let t3 = t.powi(3);
        let t2 = t.powi(3);
        let first = vk * (2.0 * t3 - 3.0 * t2 + 1.0);
        let second = bk * td * (t3 - 2.0 * t2 + t);
        let third = vk_1 * (-2.0 * t3 + 3.0 * t2);
        let forth = ak_1 * td * (t3 - t2);
        first + second + third + forth
    }
}
