#![allow(unused, clippy::new_without_default)]

use std::{
    fmt::{self, Display, Formatter},
    io::{Read, Seek},
    string::{FromUtf16Error, FromUtf8Error},
};

use binrw::{prelude::*, Endian};
use modular_bitfield::prelude::*;

#[derive(Debug, Clone)]
pub enum PmxFormatError {
    BadUtf8Text(FromUtf8Error),
    BadUtf16Text,
    BadBoolean(u8),
}

impl Display for PmxFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PmxFormatError::BadUtf8Text(err) => write!(f, "Bad UTF-8 text: {}", err),
            PmxFormatError::BadUtf16Text => write!(f, "Bad UTF-16 text"),
            PmxFormatError::BadBoolean(value) => write!(f, "Bad boolean value: {}", value),
        }
    }
}

impl From<FromUtf8Error> for PmxFormatError {
    fn from(value: FromUtf8Error) -> Self {
        PmxFormatError::BadUtf8Text(value)
    }
}

impl From<FromUtf16Error> for PmxFormatError {
    fn from(_: FromUtf16Error) -> Self {
        PmxFormatError::BadUtf16Text
    }
}

#[derive(Debug, Clone, BinRead)]
struct PmxText {
    length: u32,
    #[br(count = length)]
    bytes: Vec<u8>,
}

impl PmxText {
    fn try_into_string(self, encoding: PmxTextEncoding) -> Result<String, PmxFormatError> {
        Ok(match encoding {
            PmxTextEncoding::Utf8 => String::from_utf8(self.bytes)?,
            PmxTextEncoding::Utf16le => {
                if self.bytes.len() % 2 != 0 {
                    return Err(PmxFormatError::BadUtf16Text);
                }
                let words: Vec<u16> = self
                    .bytes
                    .chunks_exact(2)
                    .map(|num| {
                        let word: [u8; 2] = num.try_into().unwrap();
                        u16::from_le_bytes(word)
                    })
                    .collect();
                String::from_utf16(&words)?
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
pub enum PmxTextEncoding {
    #[br(magic = 0u8)]
    Utf16le,
    #[br(magic = 1u8)]
    Utf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
pub enum PmxIndexType {
    #[br(magic = 1u8)]
    Byte,
    #[br(magic = 2u8)]
    Short,
    #[br(magic = 4u8)]
    Int,
}

#[derive(Debug, Clone, Copy)]
pub struct PmxIndex(pub Option<usize>);

impl BinRead for PmxIndex {
    type Args<'a> = (PmxIndexType,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        match args.0 {
            PmxIndexType::Byte => {
                let mut buf = [0u8; 1];
                reader.read_exact(&mut buf)?;
                let index = buf[0] as i8;
                if index == -1 {
                    Ok(Self(None))
                } else if index < 0 {
                    let pos = reader.stream_position()?;
                    Err(binrw::Error::AssertFail {
                        pos,
                        message: format!("Bad byte index: {}", index),
                    })
                } else {
                    Ok(Self(Some(index as usize)))
                }
            }
            PmxIndexType::Short => {
                let mut buf = [0u8; 2];
                reader.read_exact(&mut buf)?;
                let index = match endian {
                    Endian::Big => i16::from_be_bytes(buf),
                    Endian::Little => i16::from_le_bytes(buf),
                };
                if index == -1 {
                    Ok(Self(None))
                } else if index < 0 {
                    let pos = reader.stream_position()?;
                    Err(binrw::Error::AssertFail {
                        pos,
                        message: format!("Bad short index: {}", index),
                    })
                } else {
                    Ok(Self(Some(index as usize)))
                }
            }
            PmxIndexType::Int => {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                let index = match endian {
                    Endian::Big => i32::from_be_bytes(buf),
                    Endian::Little => i32::from_le_bytes(buf),
                };
                if index == -1 {
                    Ok(Self(None))
                } else if index < 0 {
                    let pos = reader.stream_position()?;
                    Err(binrw::Error::AssertFail {
                        pos,
                        message: format!("Bad int index: {}", index),
                    })
                } else {
                    Ok(Self(Some(index as usize)))
                }
            }
        }
    }
}

#[derive(Debug, Clone, BinRead)]
pub struct PmxGlobals {
    #[br(assert(globals_count >= 8))]
    pub globals_count: i8,
    pub text_encoding: PmxTextEncoding,
    #[br(assert((0..=4).contains(&additional_vec4_count)))]
    pub additional_vec4_count: i8,
    pub vertex_index_type: PmxIndexType,
    pub texture_index_type: PmxIndexType,
    pub material_index_type: PmxIndexType,
    pub bone_index_type: PmxIndexType,
    pub morph_index_type: PmxIndexType,
    #[br(pad_after = globals_count - 8)]
    pub rigidbody_index_type: PmxIndexType,
}

#[derive(Debug, Clone, BinRead)]
pub struct PmxFileHeader {
    pub globals: PmxGlobals,
    #[br(try_map = |str: PmxText| str.try_into_string(globals.text_encoding))]
    pub model_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(globals.text_encoding))]
    pub model_name_universal: String,
    #[br(try_map = |str: PmxText| str.try_into_string(globals.text_encoding))]
    pub comments_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(globals.text_encoding))]
    pub comments_universal: String,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub enum PmxWeightDeform {
    #[br(magic = 0u8)]
    Bdef1 {
        #[br(args(header.globals.bone_index_type))]
        bone_index: PmxIndex,
    },
    #[br(magic = 1u8)]
    Bdef2 {
        #[br(args(header.globals.bone_index_type))]
        bone_index_1: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_2: PmxIndex,
        bone_weight_1: f32,
    },
    #[br(magic = 2u8)]
    Bdef4 {
        #[br(args(header.globals.bone_index_type))]
        bone_index_1: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_2: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_3: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_4: PmxIndex,
        bone_weight_1: f32,
        bone_weight_2: f32,
        bone_weight_3: f32,
        bone_weight_4: f32,
    },
    #[br(magic = 3u8)]
    Sdef {
        #[br(args(header.globals.bone_index_type))]
        bone_index_1: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_2: PmxIndex,
        bone_weight_1: f32,
        c: [f32; 3],
        r0: [f32; 3],
        r1: [f32; 3],
    },
    #[br(magic = 4u8)]
    Qdef {
        #[br(args(header.globals.bone_index_type))]
        bone_index_1: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_2: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_3: PmxIndex,
        #[br(args(header.globals.bone_index_type))]
        bone_index_4: PmxIndex,
        bone_weight_1: f32,
        bone_weight_2: f32,
        bone_weight_3: f32,
        bone_weight_4: f32,
    },
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    #[br(count = header.globals.additional_vec4_count)]
    pub additional_vec4: Vec<[f32; 4]>,
    #[br(args{ header })]
    pub weight_deform: PmxWeightDeform,
    pub edge_scale: f32,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxTexture {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub path: String,
}

#[bitfield]
#[derive(Debug, Clone, BinRead)]
#[br(map = Self::from_bytes)]
pub struct PmxDrawingFlags {
    no_cull: bool,
    ground_shadow: bool,
    draw_shadow: bool,
    receive_shadow: bool,
    has_edge: bool,
    #[skip]
    __: B3,
}

#[derive(Debug, Clone, BinRead)]
pub enum PmxEnvironmentBlendMode {
    #[br(magic = 0u8)]
    Disabled,
    #[br(magic = 1u8)]
    Multiply,
    #[br(magic = 2u8)]
    Additive,
    #[br(magic = 3u8)]
    AdditionalVec4,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub enum PmxToonReference {
    #[br(magic = 0u8)]
    Texture {
        #[br(args(header.globals.texture_index_type))]
        index: PmxIndex,
    },
    #[br(magic = 1u8)]
    Internal { index: u8 },
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxMaterial {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub material_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub material_name_universal: String,
    pub diffuse_color: [f32; 4],
    pub specular_color: [f32; 3],
    pub specular_strength: f32,
    pub ambient_color: [f32; 3],
    pub drawing_flags: PmxDrawingFlags,
    pub edge_colour: [f32; 4],
    pub edge_scale: f32,
    #[br(args(header.globals.texture_index_type))]
    pub texture_index: PmxIndex,
    #[br(args(header.globals.texture_index_type))]
    pub environment_index: PmxIndex,
    pub environment_blend_mode: PmxEnvironmentBlendMode,
    #[br(args { header: &header })]
    pub toon_reference: PmxToonReference,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub meta_data: String,
    #[br(assert(surface_count >= 0))]
    pub surface_count: i32,
}

#[bitfield]
#[derive(Debug, Clone, BinRead)]
#[br(map = Self::from_bytes)]
pub struct PmxBoneFlags {
    indexed_tail_position: bool,
    rotatable: bool,
    translatable: bool,
    is_visible: bool,
    enabled: bool,
    ik: bool,
    #[skip]
    __: B2,
    inherit_rotation: bool,
    inherit_translation: bool,
    fixed_axis: bool,
    local_coordinate: bool,
    physics_after_deform: bool,
    external_parent_deform: bool,
    #[skip]
    __: B2,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader, flags: &PmxBoneFlags } )]
pub enum PmxBoneTailPosition {
    #[br(assert(!flags.indexed_tail_position()))]
    Position([f32; 3]),
    #[br(assert(flags.indexed_tail_position()))]
    Indexed {
        #[br(args(header.globals.bone_index_type))]
        bone_index: PmxIndex,
    },
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBoneInheritParent {
    #[br(args(header.globals.bone_index_type))]
    inherit_parent_index: PmxIndex,
    inherit_parent_influence: f32,
}

#[derive(Debug, Clone, BinRead)]
pub enum PmxBoneIkLinkLimit {
    #[br(magic = 0u8)]
    None,
    #[br(magic = 1u8)]
    Some {
        limit_min: [f32; 3],
        limit_max: [f32; 3],
    },
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBoneIkLink {
    #[br(args(header.globals.bone_index_type))]
    bone_index: PmxIndex,
    limits: PmxBoneIkLinkLimit,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBoneIk {
    #[br(args(header.globals.bone_index_type))]
    target_index: PmxIndex,
    #[br(assert(loop_count >= 0))]
    loop_count: i32,
    limit_radian: f32,
    #[br(assert(link_count >= 0))]
    link_count: i32,
    #[br(args { count: link_count as usize, inner: binrw::args! { header: &header } })]
    links: Vec<PmxBoneIkLink>,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBone {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    bone_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    bone_name_universal: String,
    position: [f32; 3],
    #[br(args(header.globals.bone_index_type))]
    parent_bone_index: PmxIndex,
    layer: i32,
    flags: PmxBoneFlags,
    #[br(args { header, flags: &flags })]
    tail_position: PmxBoneTailPosition,
    #[br(if(flags.inherit_rotation() || flags.inherit_translation()), args { header })]
    inherit_parent: Option<PmxBoneInheritParent>,
    #[br(if(flags.fixed_axis()))]
    axis_direction: Option<[f32; 3]>,
    #[br(if(flags.local_coordinate()))]
    local_coordinate: Option<[[f32; 3]; 2]>,
    #[br(if(flags.external_parent_deform()), args(header.globals.bone_index_type))]
    external_parent_index: Option<PmxIndex>,
    #[br(if(flags.ik()), args { header })]
    ik: Option<PmxBoneIk>,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq, Eq)]
pub enum PmxMorphPanelType {
    #[br(magic = 0u8)]
    Hidden,
    #[br(magic = 1u8)]
    Eyebrows,
    #[br(magic = 2u8)]
    Eyes,
    #[br(magic = 3u8)]
    Mouth,
    #[br(magic = 4u8)]
    Other,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq, Eq)]
pub enum PmxMorphType {
    #[br(magic = 0u8)]
    Group,
    #[br(magic = 1u8)]
    Vertex,
    #[br(magic = 2u8)]
    Bone,
    #[br(magic = 3u8)]
    Uv,
    #[br(magic = 4u8)]
    UvExt1,
    #[br(magic = 5u8)]
    UvExt2,
    #[br(magic = 6u8)]
    UvExt3,
    #[br(magic = 7u8)]
    UvExt4,
    #[br(magic = 8u8)]
    Material,
    #[br(magic = 9u8)]
    Flip,
    #[br(magic = 10u8)]
    Impulse,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxGroupMorphData {
    #[br(args(header.globals.morph_index_type))]
    morph_index: PmxIndex,
    influence: f32,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxVertexMorphData {
    #[br(args(header.globals.vertex_index_type))]
    vertex_index: PmxIndex,
    translation: [f32; 3],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBoneMorphData {
    #[br(args(header.globals.bone_index_type))]
    bone_index: PmxIndex,
    translation: [f32; 3],
    rotation: [f32; 3],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxUvMorphData {
    #[br(args(header.globals.vertex_index_type))]
    vertex_index: PmxIndex,
    floats: [f32; 4],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxMaterialMorphData {
    #[br(args(header.globals.material_index_type))]
    material_index: PmxIndex,
    unknown: i8,
    diffuse: [f32; 4],
    specular: [f32; 3],
    specularity: f32,
    ambient: [f32; 3],
    edge_color: [f32; 4],
    edge_size: f32,
    texture_tint: [f32; 4],
    environment_tint: [f32; 4],
    toon_tint: [f32; 4],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxFlipMorphData {
    #[br(args(header.globals.morph_index_type))]
    morph_index: PmxIndex,
    influence: f32,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxImpulseMorphData {
    #[br(args(header.globals.rigidbody_index_type))]
    rigidbody_index: PmxIndex,
    local_flag: i8,
    movement_speed: [f32; 3],
    rotation_torque: [f32; 3],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader, morph_type: PmxMorphType } )]
pub enum PmxMorphOffsetData {
    #[br(assert(morph_type == PmxMorphType::Group))]
    Group {
        #[br(args { header })]
        data: PmxGroupMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::Vertex))]
    Vertex {
        #[br(args { header })]
        data: PmxVertexMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::Bone))]
    Bone {
        #[br(args { header })]
        data: PmxBoneMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::Uv))]
    Uv {
        #[br(args { header })]
        data: PmxUvMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::UvExt1))]
    UvExt1 {
        #[br(args { header })]
        data: PmxUvMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::UvExt2))]
    UvExt2 {
        #[br(args { header })]
        data: PmxUvMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::UvExt3))]
    UvExt3 {
        #[br(args { header })]
        data: PmxUvMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::UvExt4))]
    UvExt4 {
        #[br(args { header })]
        data: PmxUvMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::Material))]
    Material {
        #[br(args { header })]
        data: PmxMaterialMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::Flip))]
    Flip {
        #[br(args { header })]
        data: PmxFlipMorphData,
    },
    #[br(assert(morph_type == PmxMorphType::Impulse))]
    Impulse {
        #[br(args { header })]
        data: PmxImpulseMorphData,
    },
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxMorph {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    morph_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    morph_name_universal: String,
    panel_type: PmxMorphPanelType,
    morph_type: PmxMorphType,
    #[br(assert(offset_size > 0))]
    offset_size: i32,
    #[br(args { count: offset_size as usize, inner: binrw::args! { morph_type, header }})]
    offset_data: Vec<PmxMorphOffsetData>,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq, Eq)]
pub enum PmxFrameType {
    #[br(magic = 0u8)]
    Bone,
    #[br(magic = 1u8)]
    Morph,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader, frame_type: PmxFrameType } )]
pub enum PmxFrameData {
    #[br(assert(frame_type == PmxFrameType::Bone))]
    Bone {
        #[br(args(header.globals.bone_index_type))]
        bone_index: PmxIndex,
    },
    #[br(assert(frame_type == PmxFrameType::Morph))]
    Morph {
        #[br(args(header.globals.morph_index_type))]
        morph_index: PmxIndex,
    },
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxFrameItem {
    frame_type: PmxFrameType,
    #[br(args { header, frame_type })]
    frame_data: PmxFrameData,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxDisplayFrame {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    display_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    display_name_universal: String,
    #[br(try_map = |num: u8| match num { 0 => Ok(false), 1 => Ok(true), other => Err(PmxFormatError::BadBoolean(other)) })]
    special_frame: bool,
    #[br(assert(frame_count >= 0))]
    frame_count: i32,
    #[br(args { count: frame_count as usize, inner: binrw::args! { header: &header } })]
    frames: Vec<PmxFrameItem>,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq, Eq)]
pub enum PmxShapeType {
    #[br(magic = 0u8)]
    Sphere,
    #[br(magic = 1u8)]
    Box,
    #[br(magic = 2u8)]
    Capsule,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq, Eq)]
pub enum PmxPhysicsMode {
    #[br(magic = 0u8)]
    FollowBone,
    #[br(magic = 1u8)]
    Physics,
    #[br(magic = 2u8)]
    PhysicsAndBone,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxRigidbody {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    rigidbody_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    rigidbody_name_universal: String,
    #[br(args(header.globals.bone_index_type))]
    related_bone_index: PmxIndex,
    group_id: i8,
    no_collision_group: i16,
    shape: PmxShapeType,
    shape_size: [f32; 3],
    shape_position: [f32; 3],
    shape_rotation: [f32; 3],
    mass: f32,
    move_attenuation: f32,
    rotation_damping: f32,
    repulsion: f32,
    friction_force: f32,
    physics_mode: PmxPhysicsMode,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq, Eq)]
pub enum PmxJointType {
    #[br(magic = 0u8)]
    Spring6dof,
    #[br(magic = 1u8)]
    _6dof,
    P2p,
    #[br(magic = 2u8)]
    ConeTwist,
    #[br(magic = 3u8)]
    Slider,
    #[br(magic = 4u8)]
    Hinge,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxJoint {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    joint_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    joint_name_universal: String,
    joint_type: PmxJointType,
    #[br(args(header.globals.rigidbody_index_type))]
    rigidbody_index_a: PmxIndex,
    #[br(args(header.globals.rigidbody_index_type))]
    rigidbody_index_b: PmxIndex,
    position: [f32; 3],
    rotation: [f32; 3],
    position_minimum: [f32; 3],
    position_maximum: [f32; 3],
    rotation_minimum: [f32; 3],
    rotation_maximum: [f32; 3],
    position_spring: [f32; 3],
    rotation_spring: [f32; 3],
}

#[derive(Debug, Clone, BinRead)]
#[br(little, magic = b"PMX ")]
pub struct PmxFile {
    #[br(magic(2.0f32))]
    pub header: PmxFileHeader,
    #[br(assert(vertex_count >= 0))]
    pub vertex_count: i32,
    #[br(args { count: vertex_count as usize, inner: binrw::args! { header: &header } })]
    pub vertices: Vec<PmxVertex>,
    #[br(assert(surfaces_count >= 0 && surfaces_count % 3 == 0))]
    pub surfaces_count: i32,
    #[br(args { count: surfaces_count as usize, inner: (header.globals.vertex_index_type,) })]
    pub surfaces: Vec<PmxIndex>,
    #[br(assert(textures_count >= 0))]
    pub textures_count: i32,
    #[br(args { count: textures_count as usize, inner: binrw::args! { header: &header } })]
    pub textures: Vec<PmxTexture>,
    #[br(assert(material_count >= 0))]
    pub material_count: i32,
    #[br(args { count: material_count as usize, inner: binrw::args! { header: &header } })]
    pub materials: Vec<PmxMaterial>,
    #[br(assert(bones_count >= 0))]
    pub bones_count: i32,
    #[br(args { count: bones_count as usize, inner: binrw::args! { header: &header } })]
    pub bones: Vec<PmxBone>,
    #[br(assert(morphs_count >= 0))]
    pub morphs_count: i32,
    #[br(args { count: morphs_count as usize, inner: binrw::args! { header: &header } })]
    pub morphs: Vec<PmxMorph>,
    #[br(assert(display_frame_count >= 0))]
    pub display_frame_count: i32,
    #[br(args { count: display_frame_count as usize, inner: binrw::args! { header: &header } })]
    pub display_frames: Vec<PmxDisplayFrame>,
    #[br(assert(rigidbodies_count >= 0))]
    pub rigidbodies_count: i32,
    #[br(args { count: rigidbodies_count as usize, inner: binrw::args! { header: &header } })]
    pub rigidbodies: Vec<PmxRigidbody>,
    #[br(assert(joint_count >= 0))]
    pub joint_count: i32,
    #[br(args { count: joint_count as usize, inner: binrw::args! { header: &header } })]
    pub joints: Vec<PmxJoint>,
}

#[cfg(test)]
mod test {
    use std::fs::File;

    use binrw::BinRead;

    use super::PmxFile;

    #[test]
    fn test_file_read() {
        let mut file = File::open("models/genshin/hutao.pmx").unwrap();
        let file = PmxFile::read(&mut file).unwrap();
        eprintln!("{:?}", file);
    }
}
