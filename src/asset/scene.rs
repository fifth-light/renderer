use super::node::NodeAsset;

#[derive(Debug, Clone)]
pub struct SceneAsset {
    pub name: Option<String>,
    pub nodes: Vec<NodeAsset>,
    pub skinned_nodes: Vec<NodeAsset>,
}
