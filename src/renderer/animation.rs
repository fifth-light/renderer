use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use glam::{Quat, Vec3};
use log::{trace, warn};

use crate::asset::animation::{
    AnimationKeyFrame, AnimationKeyFrames, AnimationSampler, Interpolate,
};

use super::node::{group::GroupNode, new_node_id};

#[derive(Debug, Default)]
pub enum AnimationState {
    #[default]
    Stopped,
    Once(Instant),
    Repeat(Instant),
    Loop(Instant),
}

#[derive(Debug)]
pub struct AnimationNode {
    id: usize,
    target_node: usize,
    sampler: AnimationSampler,
    length: Duration,
}

impl AnimationNode {
    pub fn new(target_node: usize, sampler: AnimationSampler, length: Duration) -> Self {
        Self {
            id: new_node_id(),
            target_node,
            sampler,
            length,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn target_node(&self) -> usize {
        self.target_node
    }

    pub fn sampler(&self) -> &AnimationSampler {
        &self.sampler
    }

    pub fn length(&self) -> &Duration {
        &self.length
    }

    pub fn update(&self, node_tree: &mut GroupNode, time: &Duration) {
        let node = if let Some(node) = node_tree.find_transform_node_mut(self.target_node) {
            node
        } else {
            #[rustfmt::skip]
            warn!("Target node to be animated not found: #{}", self.target_node);
            return;
        };

        let time = time.as_millis() as f32 / 1000.0;
        trace!("Animate time: {:#.03}s", time);

        fn find_keyframe<T: Debug + Clone>(
            time: f32,
            keyframe: &[AnimationKeyFrame<T>],
        ) -> Option<(f32, &T, &T)> {
            let mut index = 0;
            while index < keyframe.len() {
                let item = keyframe.get(index + 1);
                if let Some(item) = item {
                    if item.time > time {
                        break;
                    }
                } else {
                    break;
                }
                index += 1;
            }
            if index >= keyframe.len() - 1 {
                return None;
            }
            let current = &keyframe[index];
            let next = &keyframe[index + 1];
            let progress = (time - current.time) / (next.time - current.time);
            Some((progress.clamp(0.0, 1.0), &current.value, &next.value))
        }

        fn interpolate_frames<T: Debug + Clone, I: Interpolate>(
            time: f32,
            keyframes: &AnimationKeyFrames<T>,
            mapper: impl Fn(&T) -> I,
        ) -> Option<I> {
            match keyframes {
                AnimationKeyFrames::Linear(vec) => {
                    find_keyframe(time, vec).map(|(progress, current, next)| {
                        let (current, next) = (mapper(current), mapper(next));
                        I::linear(current, next, progress)
                    })
                }
                AnimationKeyFrames::Step(vec) => {
                    find_keyframe(time, vec).map(|(_, current, _)| mapper(current))
                }
                AnimationKeyFrames::CubicSpline(vec) => {
                    find_keyframe(time, vec).map(|(progress, current, next)| {
                        let (val_cur, out_cur) = (mapper(&current.1), mapper(&current.2));
                        let (in_next, val_next) = (mapper(&next.0), mapper(&next.1));
                        I::cubic_spline(
                            val_cur,
                            out_cur,
                            val_next,
                            in_next,
                            progress,
                            1.0 - progress,
                        )
                    })
                }
            }
        }

        match &self.sampler {
            AnimationSampler::Rotation(keyframes) => {
                if let Some(rotation) =
                    interpolate_frames(time, keyframes, |arr| Quat::from_array(*arr))
                {
                    let mut transform = node.transform().clone();
                    transform.rotation = rotation.normalize();
                    node.set_transform(transform);
                }
            }
            AnimationSampler::Translation(keyframes) => {
                if let Some(translation) =
                    interpolate_frames(time, keyframes, |arr| Vec3::from_array(*arr))
                {
                    let mut transform = node.transform().clone();
                    transform.translation = translation;
                    node.set_transform(transform);
                }
            }
            AnimationSampler::Scale(keyframes) => {
                if let Some(scale) =
                    interpolate_frames(time, keyframes, |arr| Vec3::from_array(*arr))
                {
                    let mut transform = node.transform().clone();
                    transform.scale = scale;
                    node.set_transform(transform);
                }
            }
        }
    }
}

pub struct AnimationGroupNode {
    id: usize,
    nodes: Vec<AnimationNode>,
    length: Duration,
    state: AnimationState,
    label: Option<String>,
}

impl AnimationGroupNode {
    pub fn new(nodes: Vec<AnimationNode>, length: Duration, label: Option<String>) -> Self {
        Self {
            id: new_node_id(),
            nodes,
            length,
            state: AnimationState::Stopped,
            label,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn state(&self) -> &AnimationState {
        &self.state
    }

    pub fn length(&self) -> Duration {
        self.length
    }

    pub fn set_state(&mut self, state: AnimationState) {
        self.state = state;
    }

    pub fn nodes(&self) -> &[AnimationNode] {
        &self.nodes
    }

    pub fn update(&mut self, node_tree: &mut GroupNode, time: &Instant) {
        match self.state {
            AnimationState::Stopped => (),
            AnimationState::Once(start_time) => {
                let end_time = start_time + self.length;
                if *time > end_time {
                    self.state = AnimationState::Stopped;
                    return;
                }
                let time: Duration = *time - start_time;
                self.nodes
                    .iter()
                    .for_each(|node| node.update(node_tree, &time));
            }
            AnimationState::Repeat(start_time) => {
                let time: Duration = *time - start_time;
                let time =
                    Duration::from_nanos(time.as_nanos() as u64 % self.length.as_nanos() as u64);
                self.nodes
                    .iter()
                    .for_each(|node| node.update(node_tree, &time));
            }
            AnimationState::Loop(start_time) => {
                let time: Duration = *time - start_time;
                let time = time.as_nanos() as u64;
                let length = self.length.as_nanos() as u64;
                let progress = time % (2 * length);
                let time = if progress > length {
                    2 * length - progress
                } else {
                    progress
                };
                let time = Duration::from_nanos(time);

                self.nodes
                    .iter()
                    .for_each(|node| node.update(node_tree, &time));
            }
        }
    }
}
