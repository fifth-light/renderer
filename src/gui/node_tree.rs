use std::sync::mpsc::Sender;

use egui::{Align2, CollapsingHeader, Context, Id, ScrollArea, Ui, Window};
use glam::EulerRot;

use crate::{
    asset::node::DecomposedTransform,
    renderer::{
        camera::CameraProjection,
        node::{
            camera::CameraNode,
            crosshair::CrosshairNode,
            group::GroupNode,
            joint::JointGroupNode,
            primitive::{PrimitiveNode, PrimitiveNodeContent},
            transform::TransformNode,
            RenderNode, RenderNodeItem,
        },
        Renderer,
    },
};

use super::{context::context_label, matrix::matrix_label, GuiAction};

pub fn primitive_node(ui: &mut Ui, node: &PrimitiveNode) {
    CollapsingHeader::new(format!("Primitive #{}", node.id()))
        .id_salt(node.id())
        .show(ui, |ui| {
            ui.label(match node.indices.as_ref() {
                Some(indices) => format!("Indices: {}", indices.indices()),
                None => "indices: None".to_string(),
            });
            ui.label(match &node.content {
                PrimitiveNodeContent::Color { buffer } => {
                    format!("Content: color (vertices: {})", buffer.vertices)
                }
                PrimitiveNodeContent::Texture { buffer, .. } => {
                    format!("Content: texture (vertices: {})", buffer.vertices)
                }
                PrimitiveNodeContent::ColorSkin { buffer } => {
                    format!("Content: skinned color (vertices: {})", buffer.vertices)
                }
                PrimitiveNodeContent::TextureSkin { buffer, .. } => {
                    format!("Content: skinned texture (vertices: {})", buffer.vertices)
                }
            });
        });
}

pub fn group_node(
    ui: &mut Ui,
    node: &GroupNode,
    renderer: &Renderer,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    let label = match node.label() {
        Some(label) => format!("Group \"{}\"", label),
        None => format!("Group #{}", node.id()),
    };
    CollapsingHeader::new(label)
        .id_salt(node.id())
        .show(ui, |ui| {
            for item in node.iter() {
                node_item(ui, item, renderer, gui_actions_tx)
            }
        });
}

pub fn transform_node(
    ui: &mut Ui,
    node: &TransformNode,
    renderer: &Renderer,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    CollapsingHeader::new(format!("Transform #{}", node.id()))
        .id_salt(node.id())
        .show(ui, |ui| {
            ui.label("Transform:");
            let DecomposedTransform {
                translation,
                rotation,
                scale,
            } = node.transform();
            ui.label(format!("Translation: {}", translation));
            let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
            ui.label(format!(
                "Rotation: x: {:#.2}deg, y: {:#.2}deg, z: {:#.2}deg",
                x.to_degrees(),
                y.to_degrees(),
                z.to_degrees()
            ));
            ui.label(format!("Scale: {}", scale));

            match node.context() {
                Some(context) => {
                    ui.label("Context:");
                    context_label(ui, node.id(), context);
                }
                None => {
                    ui.label("Context: None");
                }
            }
            node_item(ui, &node.node, renderer, gui_actions_tx);
        });
}

pub fn crosshair_node(ui: &mut Ui, node: &CrosshairNode) {
    CollapsingHeader::new(format!("Crosshair #{}", node.id()))
        .id_salt(node.id())
        .show(ui, |ui| primitive_node(ui, node.item()));
}

pub fn joint_node(
    ui: &mut Ui,
    node: &JointGroupNode,
    renderer: &Renderer,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    CollapsingHeader::new(format!("Joint #{}", node.id()))
        .id_salt(node.id())
        .show(ui, |ui| {
            ui.label(format!(
                "joint ids: [{}]",
                node.joint_ids()
                    .iter()
                    .map(usize::to_string)
                    .reduce(|a, b| { a + ", " + &b })
                    .unwrap_or(String::new())
            ));
            let joint_id = Id::new(node.id()).with("Joint Matrix");
            CollapsingHeader::new("joint matrixs: ")
                .id_salt(joint_id)
                .show(ui, |ui| {
                    for (index, matrix) in node.joint_matrixs().iter().enumerate() {
                        CollapsingHeader::new(format!("#{}", index))
                            .id_salt(joint_id.with(index))
                            .show(ui, |ui| {
                                matrix_label(
                                    ui,
                                    joint_id.with(index).with("Matrix Content"),
                                    matrix,
                                );
                            });
                    }
                });
            CollapsingHeader::new("items: ")
                .id_salt(Id::new(node.id()).with(node.node.id()))
                .show(ui, |ui| {
                    node_item(ui, &node.node, renderer, gui_actions_tx);
                });
            CollapsingHeader::new("joints: ")
                .id_salt(Id::new(node.id()).with(node.joint_root.id()))
                .show(ui, |ui| {
                    node_item(ui, &node.joint_root, renderer, gui_actions_tx);
                });
        });
}

pub fn camera_node(
    ui: &mut Ui,
    camera: &CameraNode,
    renderer: &Renderer,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    let label = if let Some(label) = camera.name() {
        format!("Camera \"{}\"", label)
    } else {
        format!("Camera #{}", camera.id())
    };
    CollapsingHeader::new(label)
        .id_salt(camera.id())
        .show(ui, |ui| {
            let enabled = renderer
                .state
                .enabled_camera()
                .map(|id| id == camera.id())
                .unwrap_or(false);
            if enabled {
                ui.label("Enabled");
                if ui.button("Disable").clicked() {
                    let _ = gui_actions_tx.send(GuiAction::EnableCamera(None));
                }
            } else {
                ui.label("Disabled");
                if ui.button("Enable").clicked() {
                    let _ = gui_actions_tx.send(GuiAction::EnableCamera(Some(camera.id())));
                }
            }
            match camera.projection() {
                CameraProjection::Orthographic {
                    xmag,
                    ymag,
                    zfar,
                    znear,
                } => {
                    ui.label("Orthographic");
                    ui.label(format!("xmag: {}", xmag));
                    ui.label(format!("ymag: {}", ymag));
                    ui.label(format!("zfar: {}", zfar));
                    ui.label(format!("znear: {}", znear));
                }
                CameraProjection::Perspective {
                    aspect,
                    yfov,
                    znear,
                    zfar,
                } => {
                    ui.label("Perspective");
                    ui.label(format!("aspect_radio: {:?}", aspect));
                    ui.label(format!("yfov: {}", yfov));
                    ui.label(format!("zfar: {:?}", zfar));
                    ui.label(format!("znear: {}", znear));
                }
            }
        });
}

pub fn node_item(
    ui: &mut Ui,
    item: &RenderNodeItem,
    renderer: &Renderer,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    match item {
        RenderNodeItem::Group(group) => group_node(ui, group, renderer, gui_actions_tx),
        RenderNodeItem::Primitive(primitive) => primitive_node(ui, primitive),
        RenderNodeItem::Transform(transform) => {
            transform_node(ui, transform, renderer, gui_actions_tx)
        }
        RenderNodeItem::Crosshair(crosshair) => crosshair_node(ui, crosshair),
        RenderNodeItem::Joint(joint) => joint_node(ui, joint, renderer, gui_actions_tx),
        RenderNodeItem::Camera(camera) => camera_node(ui, camera, renderer, gui_actions_tx),
    }
}

pub fn node_tree(ctx: &Context, renderer: &Renderer, gui_actions_tx: &mut Sender<GuiAction>) {
    Window::new("Node Tree")
        .pivot(Align2::LEFT_TOP)
        .resizable([false, true])
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                group_node(ui, renderer.root_node(), renderer, gui_actions_tx)
            })
        });
}
