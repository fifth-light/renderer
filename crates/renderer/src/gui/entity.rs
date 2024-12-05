use egui::{Align2, CollapsingHeader, Context, ScrollArea, Ui, Window};
use renderer_protocol::entity::EntityResourceData;

use crate::client::{entity::Entity, world::Entities};

fn resource(ui: &mut Ui, resource: &EntityResourceData) {
    match resource {
        EntityResourceData::Box => {
            ui.label("Box");
        }
        EntityResourceData::Crosshair => {
            ui.label("Crosshair");
        }
        EntityResourceData::External { bundle_index, link } => {
            ui.label(format!("Resource: {} ({})", bundle_index, link));
        }
    }
}

pub fn entities(ctx: &Context, entities: &Entities) {
    Window::new("Entities")
        .pivot(Align2::LEFT_TOP)
        .resizable([false, true])
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for (id, entity) in entities.object.iter() {
                    CollapsingHeader::new(format!("Object {:?}", id))
                        .id_salt(id)
                        .show(ui, |ui| {
                            let position = entity.position();
                            ui.label(format!(
                                "Position: [{:#.2}, {:#.2}, {:#.2}]",
                                position.x, position.y, position.z
                            ));
                            resource(ui, entity.resource());
                        });
                }
                for (id, entity) in entities.player.iter() {
                    CollapsingHeader::new(format!("Player {:?}", id))
                        .id_salt(id)
                        .show(ui, |ui| {
                            let position = entity.position();
                            ui.label(format!(
                                "Position: [{:#.2}, {:#.2}, {:#.2}]",
                                position.x, position.y, position.z
                            ))
                        });
                }
            })
        });
}
