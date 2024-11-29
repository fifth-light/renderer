use std::{error::Error, fmt::Debug, rc::Rc};

use serde::{Deserialize, Serialize};

pub trait Codec<Item, SinkItem>: Debug + Clone + 'static
where
    Item: for<'a> Deserialize<'a>,
    SinkItem: Serialize,
{
    fn serialize(&self, item: &SinkItem) -> Result<Vec<u8>, Rc<dyn Error>>;
    fn deserialize(&self, bytes: &[u8]) -> Result<Item, Rc<dyn Error>>;
}

#[derive(Debug, Default, Clone)]
pub struct Json;

impl<Item, SinkItem> Codec<Item, SinkItem> for Json
where
    Item: for<'a> Deserialize<'a>,
    SinkItem: Serialize,
{
    fn serialize(&self, item: &SinkItem) -> Result<Vec<u8>, Rc<dyn Error>> {
        serde_json::to_vec(&item).map_err(|err| {
            let err = Rc::new(err);
            err as Rc<dyn Error>
        })
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Item, Rc<dyn Error>> {
        serde_json::from_slice(bytes).map_err(|err| {
            let err = Rc::new(err);
            err as Rc<dyn Error>
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Bincode;

impl<Item, SinkItem> Codec<Item, SinkItem> for Bincode
where
    Item: for<'a> Deserialize<'a>,
    SinkItem: Serialize,
{
    fn serialize(&self, item: &SinkItem) -> Result<Vec<u8>, Rc<dyn Error>> {
        bincode::serialize(item).map_err(|err| {
            let err = Rc::new(err);
            err as Rc<dyn Error>
        })
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Item, Rc<dyn Error>> {
        bincode::deserialize(bytes).map_err(|err| {
            let err = Rc::new(err);
            err as Rc<dyn Error>
        })
    }
}
