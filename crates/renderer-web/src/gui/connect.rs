use std::fmt::{self, Display, Formatter};

use renderer::{
    egui::{ComboBox, Grid, Ui},
    gui::connect::ConnectParam,
    transport::TransportParam,
};
use serde::{Deserialize, Serialize};
use web_sys::Url;

use crate::transport::{
    codec::{Bincode, Json},
    websocket::WebSocketTransportParam,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
pub enum SerializeType {
    #[default]
    Json,
    Bincode,
}

impl Display for SerializeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SerializeType::Json => write!(f, "Json"),
            SerializeType::Bincode => write!(f, "Bincode"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebSocketConnectParam {
    serialize_type: SerializeType,
    uri: String,
    name: String,
}

impl ConnectParam for WebSocketConnectParam {
    fn name(&self) -> &str {
        if self.name.is_empty() {
            if self.uri.is_empty() {
                "Empty connection"
            } else {
                &self.uri
            }
        } else {
            &self.name
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        Grid::new("Connection").num_columns(2).show(ui, |ui| {
            ui.label("Serialize type");
            ComboBox::from_id_salt("Serialize type")
                .selected_text(self.serialize_type.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.serialize_type,
                        SerializeType::Json,
                        SerializeType::Json.to_string(),
                    );
                    ui.selectable_value(
                        &mut self.serialize_type,
                        SerializeType::Bincode,
                        SerializeType::Bincode.to_string(),
                    );
                });
            ui.end_row();

            ui.label("Name");
            ui.text_edit_singleline(&mut self.name);
            ui.end_row();

            ui.label("URI");
            ui.text_edit_singleline(&mut self.uri);
            ui.end_row();
        });
    }

    fn param(&self) -> Option<Box<dyn TransportParam>> {
        let url = Url::new(&self.uri).map_err(|_| ()).ok()?;
        Some(match self.serialize_type {
            SerializeType::Json => Box::new(WebSocketTransportParam::new(url, Json)),
            SerializeType::Bincode => Box::new(WebSocketTransportParam::new(url, Bincode)),
        })
    }
}
