use egui::{Align2, CollapsingHeader, Context, ScrollArea, Window};

use crate::client::{entity::Entity, world::Entities};

pub fn entities(ctx: &Context, entities: &Entities) {
    Window::new("Entities")
        .pivot(Align2::LEFT_TOP)
        .resizable([false, true])
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for (id, entity) in entities.test.iter() {
                    CollapsingHeader::new(format!("Test {:?}", id))
                        .id_salt(id)
                        .show(ui, |ui| {
                            let position = entity.position();
                            ui.label(format!(
                                "Position: [{:#.2}, {:#.2}, {:#.2}]",
                                position.x, position.y, position.z
                            ))
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
