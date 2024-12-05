use std::fmt::{self, Display, Formatter, LowerHex, UpperHex};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundleIndex(pub [u8; 32]);

impl LowerHex for BundleIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl UpperHex for BundleIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

impl AsRef<[u8; 32]> for BundleIndex {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for BundleIndex {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl Display for BundleIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self)
    }
}

impl BundleIndex {
    #[cfg(feature = "digest")]
    pub fn digest_from_reader<R: std::io::Read>(mut reader: R) -> std::io::Result<Self> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        std::io::copy(&mut reader, &mut hasher)?;

        let hash = hasher.finalize();
        Ok(Self(hash.into()))
    }

    #[cfg(feature = "digest")]
    pub fn digest_from_buffer(buffer: &[u8]) -> Self {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(buffer);
        let hash = hasher.finalize();
        Self(hash.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BundleAssetType {
    Node,
    Skin,
    Texture,
    Material,
}

impl Display for BundleAssetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BundleAssetType::Node => write!(f, "Node"),
            BundleAssetType::Texture => write!(f, "Texture"),
            BundleAssetType::Skin => write!(f, "Skin"),
            BundleAssetType::Material => write!(f, "Material"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetIndex {
    Bundle(BundleIndex),
    BundlePath(BundleIndex, String),
    BundleTypeIndex(BundleIndex, BundleAssetType, usize),
}

impl Display for AssetIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AssetIndex::Bundle(bundle_index) => Display::fmt(bundle_index, f),
            AssetIndex::BundlePath(bundle_index, path) => write!(f, "{}: {}", bundle_index, path),
            AssetIndex::BundleTypeIndex(bundle_index, asset_type, index) => {
                write!(f, "{} - {}: {}", bundle_index, asset_type, index)
            }
        }
    }
}
