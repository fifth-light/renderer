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
    pub no_cull: bool,
    pub ground_shadow: bool,
    pub draw_shadow: bool,
    pub receive_shadow: bool,
    pub has_edge: bool,
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
    pub indexed_tail_position: bool,
    pub rotatable: bool,
    pub translatable: bool,
    pub is_visible: bool,
    pub enabled: bool,
    pub ik: bool,
    #[skip]
    __: B2,
    pub inherit_rotation: bool,
    pub inherit_translation: bool,
    pub fixed_axis: bool,
    pub local_coordinate: bool,
    pub physics_after_deform: bool,
    pub external_parent_deform: bool,
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
    pub inherit_parent_index: PmxIndex,
    pub inherit_parent_influence: f32,
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
    pub bone_index: PmxIndex,
    pub limits: PmxBoneIkLinkLimit,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBoneIk {
    #[br(args(header.globals.bone_index_type))]
    pub target_index: PmxIndex,
    #[br(assert(loop_count >= 0))]
    pub loop_count: i32,
    pub limit_radian: f32,
    #[br(assert(link_count >= 0))]
    pub link_count: i32,
    #[br(args { count: link_count as usize, inner: binrw::args! { header: &header } })]
    pub links: Vec<PmxBoneIkLink>,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBone {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub bone_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub bone_name_universal: String,
    pub position: [f32; 3],
    #[br(args(header.globals.bone_index_type))]
    pub parent_bone_index: PmxIndex,
    pub layer: i32,
    pub flags: PmxBoneFlags,
    #[br(args { header, flags: &flags })]
    pub tail_position: PmxBoneTailPosition,
    #[br(if(flags.inherit_rotation() || flags.inherit_translation()), args { header })]
    pub inherit_parent: Option<PmxBoneInheritParent>,
    #[br(if(flags.fixed_axis()))]
    pub axis_direction: Option<[f32; 3]>,
    #[br(if(flags.local_coordinate()))]
    pub local_coordinate: Option<[[f32; 3]; 2]>,
    #[br(if(flags.external_parent_deform()), args(header.globals.bone_index_type))]
    pub external_parent_index: Option<PmxIndex>,
    #[br(if(flags.ik()), args { header })]
    pub ik: Option<PmxBoneIk>,
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
    pub morph_index: PmxIndex,
    pub influence: f32,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxVertexMorphData {
    #[br(args(header.globals.vertex_index_type))]
    pub vertex_index: PmxIndex,
    pub translation: [f32; 3],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxBoneMorphData {
    #[br(args(header.globals.bone_index_type))]
    pub bone_index: PmxIndex,
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxUvMorphData {
    #[br(args(header.globals.vertex_index_type))]
    pub vertex_index: PmxIndex,
    pub floats: [f32; 4],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxMaterialMorphData {
    #[br(args(header.globals.material_index_type))]
    pub material_index: PmxIndex,
    pub unknown: i8,
    pub diffuse: [f32; 4],
    pub specular: [f32; 3],
    pub specularity: f32,
    pub ambient: [f32; 3],
    pub edge_color: [f32; 4],
    pub edge_size: f32,
    pub texture_tint: [f32; 4],
    pub environment_tint: [f32; 4],
    pub toon_tint: [f32; 4],
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxFlipMorphData {
    #[br(args(header.globals.morph_index_type))]
    pub morph_index: PmxIndex,
    pub influence: f32,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxImpulseMorphData {
    #[br(args(header.globals.rigidbody_index_type))]
    pub rigidbody_index: PmxIndex,
    pub local_flag: i8,
    pub movement_speed: [f32; 3],
    pub rotation_torque: [f32; 3],
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
    pub morph_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub morph_name_universal: String,
    pub panel_type: PmxMorphPanelType,
    pub morph_type: PmxMorphType,
    #[br(assert(offset_size > 0))]
    pub offset_size: i32,
    #[br(args { count: offset_size as usize, inner: binrw::args! { morph_type, header }})]
    pub offset_data: Vec<PmxMorphOffsetData>,
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
    pub frame_type: PmxFrameType,
    #[br(args { header, frame_type })]
    pub frame_data: PmxFrameData,
}

#[derive(Debug, Clone, BinRead)]
#[br(import { header: &PmxFileHeader } )]
pub struct PmxDisplayFrame {
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub display_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub display_name_universal: String,
    #[br(try_map = |num: u8| match num { 0 => Ok(false), 1 => Ok(true), other => Err(PmxFormatError::BadBoolean(other)) })]
    pub special_frame: bool,
    #[br(assert(frame_count >= 0))]
    pub frame_count: i32,
    #[br(args { count: frame_count as usize, inner: binrw::args! { header: &header } })]
    pub frames: Vec<PmxFrameItem>,
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
    pub rigidbody_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub rigidbody_name_universal: String,
    #[br(args(header.globals.bone_index_type))]
    pub related_bone_index: PmxIndex,
    pub group_id: i8,
    pub no_collision_group: i16,
    pub shape: PmxShapeType,
    pub shape_size: [f32; 3],
    pub shape_position: [f32; 3],
    pub shape_rotation: [f32; 3],
    pub mass: f32,
    pub move_attenuation: f32,
    pub rotation_damping: f32,
    pub repulsion: f32,
    pub friction_force: f32,
    pub physics_mode: PmxPhysicsMode,
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
    pub joint_name_local: String,
    #[br(try_map = |str: PmxText| str.try_into_string(header.globals.text_encoding))]
    pub joint_name_universal: String,
    pub joint_type: PmxJointType,
    #[br(args(header.globals.rigidbody_index_type))]
    pub xsrigidbody_index_a: PmxIndex,
    #[br(args(header.globals.rigidbody_index_type))]
    pub rigidbody_index_b: PmxIndex,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub position_minimum: [f32; 3],
    pub position_maximum: [f32; 3],
    pub rotation_minimum: [f32; 3],
    pub rotation_maximum: [f32; 3],
    pub position_spring: [f32; 3],
    pub rotation_spring: [f32; 3],
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
