use std::fmt::{self, Display, Formatter};

use egui::{ComboBox, Grid, Ui};
use serde::{Deserialize, Serialize};
use tokio_serde::formats::{Bincode, Json};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::Uri};

use crate::transport::{tokio::TokioTransportParam, TransportParam};

use super::ConnectParam;

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
pub struct TokioConnectParam {
    serialize_type: SerializeType,
    uri: String,
    name: String,
}

impl ConnectParam for TokioConnectParam {
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
        let uri = Uri::try_from(&self.uri).map_err(|_| ()).ok()?;
        let request = uri.into_client_request().map_err(|_| ()).ok()?;
        Some(match self.serialize_type {
            SerializeType::Json => Box::new(TokioTransportParam::new(request, Json::default)),
            SerializeType::Bincode => Box::new(TokioTransportParam::new(request, Bincode::default)),
        })
    }
}
