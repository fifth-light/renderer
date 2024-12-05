use super::node::NodeAsset;

#[derive(Debug, Clone, Default)]
pub struct SceneAsset {
    pub name: Option<String>,
    pub nodes: Vec<NodeAsset>,
}
