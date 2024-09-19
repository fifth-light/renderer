use wgpu::{Device, Queue};

use crate::{
    asset::primitive::{PrimitiveAsset, PrimitiveAssetMode},
    renderer::{context::Context, loader::RendererAssetLoader},
};

use super::{new_node_id, primitive::PrimitiveNode, OngoingRenderState, RenderNode, RendererState};

const CROSSHAIR_POSITIONS: &[[f32; 3]] = &[
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
];

const CROSSHAIR_VERTEX_COLORS: &[[f32; 4]] = &[
    [1.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
];

#[derive(Debug)]
pub struct CrosshairNode {
    id: usize,
    node: PrimitiveNode,
}

impl CrosshairNode {
    pub fn new(
        device: &Device,
        queue: &Queue,
        asset_node_loader: &mut RendererAssetLoader,
    ) -> CrosshairNode {
        let asset = PrimitiveAsset {
            name: None,
            positions: CROSSHAIR_POSITIONS.to_vec(),
            vertex_color: vec![CROSSHAIR_VERTEX_COLORS.to_vec()],
            tex_coords: vec![],
            skin: vec![],
            material: None,
            indices: None,
            mode: PrimitiveAssetMode::LineList,
        };
        let node = asset_node_loader.load_primitive(device, queue, asset);
        CrosshairNode {
            id: new_node_id(),
            node,
        }
    }

    pub fn item(&self) -> &PrimitiveNode {
        &self.node
    }
}

impl RenderNode for CrosshairNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(&mut self, context: &Context, invalid: bool) -> bool {
        self.node.update(context, invalid)
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        self.node.prepare(device, queue, renderer_state);
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        self.node.draw(renderer_state, ongoing_state);
    }
}
