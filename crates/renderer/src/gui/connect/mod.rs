use std::{
    fmt::{self, Display, Formatter},
    sync::mpsc::Sender,
};

use egui::{Align2, Button, ComboBox, Context, Ui, Vec2, Window};
use renderer_protocol::version::VersionData;
use serde::{Deserialize, Serialize};

use crate::{client::ConnectionState, transport::TransportParam};

use super::GuiAction;

#[cfg(feature = "tokio-transport")]
pub mod tokio;

pub trait ConnectParam: Serialize + for<'a> Deserialize<'a> + Clone + Default {
    fn name(&self) -> &str;
    fn ui(&mut self, ui: &mut Ui);
    fn param(&self) -> Option<Box<dyn TransportParam>>;
}

pub fn connect<Param: ConnectParam>(
    ctx: &Context,
    selected_index: &mut usize,
    params: &mut Vec<Param>,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    Window::new("Connect")
        .resizable([false, false])
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            if *selected_index >= params.len() {
                *selected_index = 0;
            }

            ui.horizontal(|ui| {
                ComboBox::from_id_salt("Select connection")
                    .selected_text(
                        params
                            .get(*selected_index)
                            .map(|param| param.name())
                            .unwrap_or("No connection available"),
                    )
                    .show_ui(ui, |ui| {
                        for (index, param) in params.iter().enumerate() {
                            ui.selectable_value(selected_index, index, param.name());
                        }
                    });
                if ui.button("Add").clicked() {
                    params.push(Param::default());
                }
            });

            let selected_index = *selected_index;
            if let Some(selected_param) = params.get_mut(selected_index) {
                selected_param.ui(ui);
            } else {
                ui.label("Please add a connection");
            }

            if params.get_mut(selected_index).is_some() {
                ui.horizontal(|ui| {
                    if let Some(selected_param) = params.get_mut(selected_index) {
                        if let Some(param) = selected_param.param() {
                            if ui.button("Connect").clicked() {
                                gui_actions_tx.send(GuiAction::Connect(param));
                            }
                        } else {
                            ui.add_enabled(false, Button::new("Connect"));
                        }
                    }

                    if ui.button("Remove").clicked() {
                        params.remove(selected_index);
                    }
                });
            }
        });
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Handshaking,
    SyncingWorld { server_version: VersionData },
    Connected,
    Closed,
}

pub fn connecting(ctx: &Context, state: ConnectionStatus) {
    Window::new("Connecting")
        .resizable([false, false])
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| match state {
            ConnectionStatus::Connecting => ui.label("Connecting"),
            ConnectionStatus::Handshaking => ui.label("Trying to handshake with server"),
            ConnectionStatus::SyncingWorld { server_version } => {
                ui.label(format!("Syncing world (server version {})", server_version))
            }
            ConnectionStatus::Connected => ui.label("Connected"),
            ConnectionStatus::Closed => ui.label("Connecting closed"),
        });
}
