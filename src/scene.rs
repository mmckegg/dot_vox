use crate::Dict;
use nom::{
    multi::count,
    number::complete::{le_i32, le_u32},
    sequence::pair,
    sequence::tuple,
    IResult,
};

use crate::parser::parse_dict;
/// Node header.
#[derive(Debug, PartialEq, Eq)]
pub struct NodeHeader {
    /// ID of this transform node.
    pub id: u32,
    /// Attributes of this transform node.
    pub attributes: Dict,
}

/// A model reference in a shape node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeModel {
    /// ID of the model.
    pub model_id: u32,
    /// Attributes of the model in this shape node.
    pub attributes: Dict,
}

impl ShapeModel {
    /// The keyframe index that this model is assigned to for the Shape node.
    pub fn frame_index(&self) -> Option<u32> {
        if let Some(input) = self.attributes.get("_f") {
            if let IResult::<&str, u32>::Ok((_, idx)) =
                nom::character::complete::u32(input.as_str())
            {
                return Some(idx);
            } else {
                debug!("Could not parse frame index of model: {}", input);
            }
        }

        None
    }
}

/// Transform node.
#[derive(Debug, PartialEq, Eq)]
pub struct SceneTransform {
    /// Header.
    pub header: NodeHeader,
    /// 1 single child (appear to be always either a group or shape node)
    pub child: u32,
    /// Layer ID.
    pub layer_id: u32,
    /// Positional frames.
    pub frames: Vec<Dict>,
}

/// Group node.
#[derive(Debug, PartialEq, Eq)]
pub struct SceneGroup {
    /// Header.
    pub header: NodeHeader,
    /// Multiple children (appear to be always transform nodes).
    pub children: Vec<u32>,
}

/// Shape node.
#[derive(Debug, PartialEq, Eq)]
pub struct SceneShape {
    /// Header.
    pub header: NodeHeader,
    /// One or more models.
    pub models: Vec<ShapeModel>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/// A color stored as sRGB.
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

/// Layer information (raw).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawLayer {
    /// ID of this layer.
    pub id: u32,
    /// Attributes of this layer.
    pub attributes: Dict,
}

/// Layer information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    /// Attributes of this layer.
    pub attributes: Dict,
}

impl Layer {
    /// Return the name for this layer, if it exists.
    pub fn name(&self) -> Option<String> {
        self.attributes.get("_name").cloned()
    }

    /// Return whether this layer is hidden (layers are visible by default).
    pub fn hidden(&self) -> bool {
        if let Some(x) = self.attributes.get("_hidden") {
            return x == "1";
        }

        false
    }

    /// Return the color associated with this layer, if one has been set.
    pub fn color(&self) -> Option<Color> {
        if let Some(x) = self.attributes.get("_color") {
            if let IResult::<&str, (u8, &str, u8, &str, u8)>::Ok((_, (r, _, g, _, b))) =
                tuple((
                    nom::character::complete::u8,
                    nom::character::complete::space1,
                    nom::character::complete::u8,
                    nom::character::complete::space1,
                    nom::character::complete::u8,
                ))(x.as_str())
            {
                return Some(Color { r, g, b });
            } else {
                debug!(
                    "Encountered _color attribute in layer that appears to be malformed: {}",
                    x
                )
            }
        }

        None
    }
}

fn parse_node_header(i: &[u8]) -> IResult<&[u8], NodeHeader> {
    let (i, (id, attributes)) = pair(le_u32, parse_dict)(i)?;
    Ok((i, NodeHeader { id, attributes }))
}

fn parse_scene_shape_model(i: &[u8]) -> IResult<&[u8], ShapeModel> {
    let (i, (model_id, attributes)) = pair(le_u32, parse_dict)(i)?;
    Ok((
        i,
        ShapeModel {
            model_id,
            attributes,
        },
    ))
}

pub fn parse_scene_transform(i: &[u8]) -> IResult<&[u8], SceneTransform> {
    let (i, header) = parse_node_header(i)?;
    let (i, child) = le_u32(i)?;
    let (i, _ignored) = le_i32(i)?;
    let (i, layer_id) = le_u32(i)?;
    let (i, frame_count) = le_u32(i)?;
    let (i, frames) = count(parse_dict, frame_count as usize)(i)?;
    Ok((
        i,
        SceneTransform {
            header,
            child,
            layer_id,
            frames,
        },
    ))
}

pub fn parse_scene_group(i: &[u8]) -> IResult<&[u8], SceneGroup> {
    let (i, header) = parse_node_header(i)?;
    let (i, child_count) = le_u32(i)?;
    let (i, children) = count(le_u32, child_count as usize)(i)?;
    Ok((i, SceneGroup { header, children }))
}

pub fn parse_scene_shape(i: &[u8]) -> IResult<&[u8], SceneShape> {
    let (i, header) = parse_node_header(i)?;
    let (i, model_count) = le_u32(i)?;
    let (i, models) = count(parse_scene_shape_model, model_count as usize)(i)?;
    Ok((i, SceneShape { header, models }))
}

pub fn parse_layer(i: &[u8]) -> IResult<&[u8], RawLayer> {
    let (i, id) = le_u32(i)?;
    let (i, attributes) = parse_dict(i)?;
    let (i, _ignored) = le_u32(i)?;
    Ok((i, RawLayer { id, attributes }))
}

/// Represents a translation. Used to position a chunk relative to other chunks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    /// The X coordinate of the translation.
    pub x: i32,
    /// The Y coordinate of the translation.
    pub y: i32,
    /// The Z coordinate of the translation.
    pub z: i32,
}

impl From<(i32, i32, i32)> for Position {
    fn from(pos: (i32, i32, i32)) -> Self {
        Position {
            x: pos.0,
            y: pos.1,
            z: pos.2,
        }
    }
}

impl From<Position> for (i32, i32, i32) {
    fn from(pos: Position) -> Self {
        (pos.x, pos.y, pos.z)
    }
}

/// Represents a rotation.  Used to orient a chunk relative to other chunks.
/// The rotation is represented as a row-major 3×3 matrix (this is how it
/// appears in the `.vox` format).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rotation {
    /// This is a row-major representation of the rotation as an orthonormal 3×3
    /// matrix. The entries are in [-1..1].
    rot: [[i8; 3]; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
/// Represents an animation.  The chunk is oriented according to the rotation
/// (`_r`) is placed at the position (`t`) specified. The Rotation is
/// instantaneous and happens at the start of the frame. The animation is
/// interpolated across the sequence of Frames using their positions.
pub struct Frame {
    /// The raw attributes as parsed from the .vox
    attributes: Dict,
}

impl Frame {
    /// Build a new frame from a set of attributes.  Note that construction is
    /// lazy; parsing happens at query time.
    pub fn new(attributes: Dict) -> Frame {
        Frame { attributes }
    }

    /// The `_r` field in the `.vox` spec.  Represents the orientation of the
    /// model.
    pub fn orientation(&self) -> Option<Rotation> {
        if let Some(value) = self.attributes.get("_r") {
            if let IResult::<&str, u8>::Ok((_, byte_rotation)) =
                nom::character::complete::u8(value.as_str())
            {
                // .vox stores a row-major rotation in the bits of a byte.
                //
                // for example :
                // R =
                //  0  1  0
                //  0  0 -1
                // -1  0  0
                // ==>
                // unsigned char _r = (1 << 0) | (2 << 2) | (0 << 4) | (1 << 5) | (1 << 6)
                //
                // bit | value
                // 0-1 : 1 : index of the non-zero entry in the first row
                // 2-3 : 2 : index of the non-zero entry in the second row
                // 4   : 0 : the sign in the first row (0 : positive; 1 : negative)
                // 5   : 1 : the sign in the second row (0 : positive; 1 : negative)
                // 6   : 1 : the sign in the third row (0 : positive; 1 : negative)

                // First two indices
                let index_nz1 = (byte_rotation & 0b11) as usize;
                let index_nz2 = ((byte_rotation & 0b1100) >> 2) as usize;

                if index_nz1 == index_nz2 {
                    debug!("'_r' in Frame is not orthnonormal! {}", value);
                    return None;
                }

                // You get the third index out via a process of elimination here. It's the one
                // that wasn't used for the other rows.
                let possible_thirds = [
                    index_nz1 == 0 || index_nz2 == 0,
                    index_nz1 == 1 || index_nz2 == 1,
                    index_nz1 == 2 || index_nz2 == 2,
                ];

                let mut index_nz3 = 0;

                for (i, possible_third) in possible_thirds.iter().enumerate() {
                    if !possible_third {
                        index_nz3 = i;
                    }
                }

                // Values of all three columns (1 or 0)
                let val_1 = if (byte_rotation & 0b1_0000) >> 4 == 1 {
                    -1
                } else {
                    1
                };
                let val_2 = if (byte_rotation & 0b10_0000) >> 5 == 1 {
                    -1
                } else {
                    1
                };
                let val_3 = if (byte_rotation & 0b100_0000) >> 6 == 1 {
                    -1
                } else {
                    1
                };

                // Rows as read from file
                let mut initial_rows: [[i8; 3]; 3] = [[0, 0, 0], [0, 0, 0], [0, 0, 0]];

                initial_rows[0][index_nz1] = val_1;
                initial_rows[1][index_nz2] = val_2;
                initial_rows[2][index_nz3] = val_3;

                return Some(Rotation { rot: initial_rows });
            } else {
                debug!("'_r' attribute for Frame could not be parsed! {}", value);
            }
        }

        None
    }

    /// The `_t` field in the `.vox` spec.  Represents the position of this
    /// frame begins in world space.
    pub fn position(&self) -> Option<Position> {
        if let Some(value) = self.attributes.get("_t") {
            match tuple((
                nom::character::complete::i32,
                nom::character::complete::space1,
                nom::character::complete::i32,
                nom::character::complete::space1,
                nom::character::complete::i32,
            ))(value.as_str())
            {
                IResult::<&str, (i32, &str, i32, &str, i32)>::Ok((_, (x, _, y, _, z))) => {
                    return Some(Position { x, y, z });
                }
                Err(_) => {
                    debug!("'_t' attribute for Frame could not be parsed! {}", value)
                }
            }
        }

        None
    }

    /// The `_f` field in the .vox spec.  Represents the frame number that this
    /// keyframe is located at.
    pub fn frame_index(&self) -> Option<u32> {
        if let Some(value) = self.attributes.get("_f") {
            if let IResult::<&str, u32>::Ok((_, frame_idx)) =
                nom::character::complete::u32(value.as_str())
            {
                return Some(frame_idx);
            } else {
                debug!("'_f' attribute for Frame could not be parsed! {}", value);
            }
        }
        None
    }
}

/// Scene graph nodes for representing a scene in
/// [`DotVoxData`](crate::dot_vox_data::DotVoxData).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneNode {
    /// Transform Node Chunk (nTRN)
    Transform {
        /// Attributes.
        attributes: Dict,
        /// Transform frames.
        frames: Vec<Frame>,
        /// Child node of this transform node.
        child: u32,
    },
    /// Group Node Chunk (nGRP)
    Group {
        /// Attributes.
        attributes: Dict,
        /// Child nodes.
        children: Vec<u32>,
    },
    /// Shape Node Chunk (nSHP)
    Shape {
        /// Attributes.
        attributes: Dict,
        /// Models.
        models: Vec<ShapeModel>,
    },
}
