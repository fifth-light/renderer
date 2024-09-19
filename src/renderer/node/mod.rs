use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

use camera::CameraNode;
use crosshair::CrosshairNode;
use group::GroupNode;
use joint::JointGroupNode;
use primitive::PrimitiveNode;
use transform::TransformNode;
use wgpu::{Device, Queue};

use super::{context::Context, OngoingRenderState, RendererState};

pub mod camera;
pub mod crosshair;
pub mod group;
pub mod joint;
pub mod primitive;
pub mod skin_group;
pub mod transform;

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn new_node_id() -> usize {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// A frame: update -> prepare -> draw
// Update: update node properties (such as transform)
// Prepare: calculate the final properties and send then to uniform
// Draw: do actual drawing
pub trait RenderNode {
    fn id(&self) -> usize;
    fn update(&mut self, _context: &Context, _invalid: bool) -> bool {
        false
    }
    fn prepare(&mut self, _device: &Device, _queue: &Queue, _renderer_state: &mut RendererState) {}
    fn draw<'a>(
        &'a self,
        _renderer_state: &'a RendererState,
        _ongoing_state: &mut OngoingRenderState<'a>,
    ) {
    }
}

pub enum RenderNodeItem {
    Group(Box<GroupNode>),
    Primitive(Box<PrimitiveNode>),
    Transform(Box<TransformNode>),
    Crosshair(Box<CrosshairNode>),
    Joint(Box<JointGroupNode>),
    Camera(Box<CameraNode>),
}

// Manually implement Debug to reduce a level of elements in debug tree
impl Debug for RenderNodeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderNodeItem::Group(group) => group.fmt(f),
            RenderNodeItem::Primitive(mesh) => mesh.fmt(f),
            RenderNodeItem::Transform(transform) => transform.fmt(f),
            RenderNodeItem::Crosshair(crosshair) => crosshair.fmt(f),
            RenderNodeItem::Joint(joint) => joint.fmt(f),
            RenderNodeItem::Camera(camera) => camera.fmt(f),
        }
    }
}

impl RenderNode for RenderNodeItem {
    fn id(&self) -> usize {
        match self {
            RenderNodeItem::Group(group) => group.id(),
            RenderNodeItem::Primitive(primitive) => primitive.id(),
            RenderNodeItem::Transform(transform) => transform.id(),
            RenderNodeItem::Crosshair(crosshair) => crosshair.id(),
            RenderNodeItem::Joint(joint) => joint.id(),
            RenderNodeItem::Camera(camera) => camera.id(),
        }
    }

    fn update(&mut self, context: &Context, invalid: bool) -> bool {
        match self {
            RenderNodeItem::Group(group) => group.update(context, invalid),
            RenderNodeItem::Primitive(mesh) => mesh.update(context, invalid),
            RenderNodeItem::Transform(transform) => transform.update(context, invalid),
            RenderNodeItem::Crosshair(crosshair) => crosshair.update(context, invalid),
            RenderNodeItem::Joint(joint) => joint.update(context, invalid),
            RenderNodeItem::Camera(camera) => camera.update(context, invalid),
        }
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        match self {
            RenderNodeItem::Group(group) => group.prepare(device, queue, renderer_state),
            RenderNodeItem::Primitive(mesh) => mesh.prepare(device, queue, renderer_state),
            RenderNodeItem::Transform(transform) => {
                transform.prepare(device, queue, renderer_state)
            }
            RenderNodeItem::Crosshair(crosshair) => {
                crosshair.prepare(device, queue, renderer_state)
            }
            RenderNodeItem::Joint(joint) => joint.prepare(device, queue, renderer_state),
            RenderNodeItem::Camera(camera) => camera.prepare(device, queue, renderer_state),
        }
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        match self {
            RenderNodeItem::Group(group) => group.draw(renderer_state, ongoing_state),
            RenderNodeItem::Primitive(primitive) => primitive.draw(renderer_state, ongoing_state),
            RenderNodeItem::Transform(transform) => transform.draw(renderer_state, ongoing_state),
            RenderNodeItem::Crosshair(crosshair) => crosshair.draw(renderer_state, ongoing_state),
            RenderNodeItem::Joint(joint) => joint.draw(renderer_state, ongoing_state),
            RenderNodeItem::Camera(camera) => camera.draw(renderer_state, ongoing_state),
        }
    }
}
