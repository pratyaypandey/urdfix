use quick_xml::{Reader, Writer, events::Event, name::QName};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::fs;
use thiserror::Error;
use indexmap::IndexMap;

#[derive(Error, Debug)]
pub enum UrdfParseError {
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid URDF structure: {0}")]
    InvalidStructure(String),
    #[error("Missing required attribute: {0}")]
    MissingAttribute(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrdfDocument {
    pub robot: Robot,
    pub raw_xml: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Robot {
    pub name: String,
    pub links: IndexMap<String, Link>,
    pub joints: IndexMap<String, Joint>,
    pub materials: IndexMap<String, Material>,
    pub gazebo_elements: Vec<GazeboElement>,
    pub transmission_elements: Vec<TransmissionElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub name: String,
    pub inertial: Option<Inertial>,
    pub visual: Vec<Visual>,
    pub collision: Vec<Collision>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub name: String,
    pub joint_type: String,
    pub parent: String,
    pub child: String,
    pub origin: Option<Origin>,
    pub axis: Option<Axis>,
    pub limit: Option<Limit>,
    pub dynamics: Option<Dynamics>,
    pub mimic: Option<Mimic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub color: Option<Color>,
    pub texture: Option<Texture>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inertial {
    pub mass: f64,
    pub origin: Option<Origin>,
    pub inertia: Option<Inertia>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Visual {
    pub name: Option<String>,
    pub origin: Option<Origin>,
    pub geometry: Option<Geometry>,
    pub material: Option<MaterialRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collision {
    pub name: Option<String>,
    pub origin: Option<Origin>,
    pub geometry: Option<Geometry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    pub xyz: [f64; 3],
    pub rpy: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    pub xyz: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Limit {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dynamics {
    pub damping: Option<f64>,
    pub friction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mimic {
    pub joint: String,
    pub multiplier: Option<f64>,
    pub offset: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inertia {
    pub ixx: f64,
    pub ixy: f64,
    pub ixz: f64,
    pub iyy: f64,
    pub iyz: f64,
    pub izz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub shape: GeometryShape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeometryShape {
    Box { size: [f64; 3] },
    Cylinder { radius: f64, length: f64 },
    Sphere { radius: f64 },
    Mesh { filename: String, scale: Option<[f64; 3]> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub rgba: [f64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialRef {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GazeboElement {
    pub reference: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransmissionElement {
    pub name: String,
    pub content: String,
}

pub struct UrdfParser;

impl UrdfParser {
    pub fn parse_file(file_path: &str) -> Result<UrdfDocument, UrdfParseError> {
        let content = fs::read_to_string(file_path)?;
        Self::parse_string(&content)
    }

    pub fn parse_string(xml_content: &str) -> Result<UrdfDocument, UrdfParseError> {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut robot = None;
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) if e.name() == QName(b"robot") => {
                    robot = Some(Self::parse_robot(&mut reader, e)?);
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        let robot = robot.ok_or_else(|| UrdfParseError::InvalidStructure("No robot element found".to_string()))?;
        
        Ok(UrdfDocument {
            robot,
            raw_xml: xml_content.to_string(),
        })
    }

    fn parse_robot(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<Robot, UrdfParseError> {
        let name = Self::get_required_attribute(start_event, b"name")?;
        
        let mut robot = Robot {
            name,
            links: IndexMap::new(),
            joints: IndexMap::new(),
            materials: IndexMap::new(),
            gazebo_elements: Vec::new(),
            transmission_elements: Vec::new(),
        };

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name().as_ref() {
                        b"link" => {
                            let link = Self::parse_link(reader, e)?;
                            robot.links.insert(link.name.clone(), link);
                        }
                        b"joint" => {
                            let joint = Self::parse_joint(reader, e)?;
                            robot.joints.insert(joint.name.clone(), joint);
                        }
                        b"material" => {
                            let material = Self::parse_material(reader, e)?;
                            robot.materials.insert(material.name.clone(), material);
                        }
                        b"gazebo" => {
                            let gazebo = Self::parse_gazebo(reader, e)?;
                            robot.gazebo_elements.push(gazebo);
                        }
                        b"transmission" => {
                            let transmission = Self::parse_transmission(reader, e)?;
                            robot.transmission_elements.push(transmission);
                        }
                        _ => {
                            Self::skip_element(reader)?;
                        }
                    }
                }
                Event::End(ref e) if e.name() == QName(b"robot") => break,
                Event::Eof => return Err(UrdfParseError::InvalidStructure("Unexpected end of file".to_string())),
                _ => {}
            }
            buf.clear();
        }

        Ok(robot)
    }

    fn parse_link(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<Link, UrdfParseError> {
        let name = Self::get_required_attribute(start_event, b"name")?;
        
        let mut link = Link {
            name,
            inertial: None,
            visual: Vec::new(),
            collision: Vec::new(),
        };

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name().as_ref() {
                        b"inertial" => link.inertial = Some(Self::parse_inertial(reader, e)?),
                        b"visual" => link.visual.push(Self::parse_visual(reader, e)?),
                        b"collision" => link.collision.push(Self::parse_collision(reader, e)?),
                        _ => Self::skip_element(reader)?,
                    }
                }
                Event::End(ref e) if e.name() == QName(b"link") => break,
                Event::Eof => return Err(UrdfParseError::InvalidStructure("Unexpected end of file".to_string())),
                _ => {}
            }
            buf.clear();
        }

        Ok(link)
    }

    fn parse_joint(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<Joint, UrdfParseError> {
        let name = Self::get_required_attribute(start_event, b"name")?;
        let joint_type = Self::get_required_attribute(start_event, b"type")?;
        
        let mut joint = Joint {
            name,
            joint_type,
            parent: String::new(),
            child: String::new(),
            origin: None,
            axis: None,
            limit: None,
            dynamics: None,
            mimic: None,
        };

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Empty(ref e) | Event::Start(ref e) => {
                    match e.name().as_ref() {
                        b"parent" => joint.parent = Self::get_required_attribute(e, b"link")?,
                        b"child" => joint.child = Self::get_required_attribute(e, b"link")?,
                        b"origin" => joint.origin = Some(Self::parse_origin_from_attributes(e)?),
                        b"axis" => joint.axis = Some(Self::parse_axis_from_attributes(e)?),
                        b"limit" => joint.limit = Some(Self::parse_limit_from_attributes(e)?),
                        b"dynamics" => joint.dynamics = Some(Self::parse_dynamics_from_attributes(e)?),
                        b"mimic" => joint.mimic = Some(Self::parse_mimic_from_attributes(e)?),
                        _ => if matches!(reader.read_event_into(&mut Vec::new())?, Event::Start(_)) { Self::skip_element(reader)?; },
                    }
                }
                Event::End(ref e) if e.name() == QName(b"joint") => break,
                Event::Eof => return Err(UrdfParseError::InvalidStructure("Unexpected end of file".to_string())),
                _ => {}
            }
            buf.clear();
        }

        Ok(joint)
    }

    fn get_required_attribute(element: &quick_xml::events::BytesStart, attr_name: &[u8]) -> Result<String, UrdfParseError> {
        element.attributes()
            .find_map(|attr| {
                attr.ok().and_then(|a| {
                    if a.key.as_ref() == attr_name {
                        Some(String::from_utf8_lossy(&a.value).to_string())
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| UrdfParseError::MissingAttribute(String::from_utf8_lossy(attr_name).to_string()))
    }

    fn get_optional_attribute(element: &quick_xml::events::BytesStart, attr_name: &[u8]) -> Option<String> {
        element.attributes()
            .find_map(|attr| {
                attr.ok().and_then(|a| {
                    if a.key.as_ref() == attr_name {
                        Some(String::from_utf8_lossy(&a.value).to_string())
                    } else {
                        None
                    }
                })
            })
    }

    fn parse_origin_from_attributes(element: &quick_xml::events::BytesStart) -> Result<Origin, UrdfParseError> {
        let xyz_str = Self::get_optional_attribute(element, b"xyz").unwrap_or_else(|| "0 0 0".to_string());
        let rpy_str = Self::get_optional_attribute(element, b"rpy").unwrap_or_else(|| "0 0 0".to_string());
        
        let xyz = Self::parse_three_floats(&xyz_str)?;
        let rpy = Self::parse_three_floats(&rpy_str)?;
        
        Ok(Origin { xyz, rpy })
    }

    fn parse_axis_from_attributes(element: &quick_xml::events::BytesStart) -> Result<Axis, UrdfParseError> {
        let xyz_str = Self::get_optional_attribute(element, b"xyz").unwrap_or_else(|| "1 0 0".to_string());
        let xyz = Self::parse_three_floats(&xyz_str)?;
        Ok(Axis { xyz })
    }

    fn parse_limit_from_attributes(element: &quick_xml::events::BytesStart) -> Result<Limit, UrdfParseError> {
        Ok(Limit {
            lower: Self::get_optional_attribute(element, b"lower").and_then(|s| s.parse().ok()),
            upper: Self::get_optional_attribute(element, b"upper").and_then(|s| s.parse().ok()),
            effort: Self::get_optional_attribute(element, b"effort").and_then(|s| s.parse().ok()),
            velocity: Self::get_optional_attribute(element, b"velocity").and_then(|s| s.parse().ok()),
        })
    }

    fn parse_dynamics_from_attributes(element: &quick_xml::events::BytesStart) -> Result<Dynamics, UrdfParseError> {
        Ok(Dynamics {
            damping: Self::get_optional_attribute(element, b"damping").and_then(|s| s.parse().ok()),
            friction: Self::get_optional_attribute(element, b"friction").and_then(|s| s.parse().ok()),
        })
    }

    fn parse_mimic_from_attributes(element: &quick_xml::events::BytesStart) -> Result<Mimic, UrdfParseError> {
        let joint = Self::get_required_attribute(element, b"joint")?;
        Ok(Mimic {
            joint,
            multiplier: Self::get_optional_attribute(element, b"multiplier").and_then(|s| s.parse().ok()),
            offset: Self::get_optional_attribute(element, b"offset").and_then(|s| s.parse().ok()),
        })
    }

    fn parse_three_floats(s: &str) -> Result<[f64; 3], UrdfParseError> {
        let parts: Result<Vec<f64>, _> = s.split_whitespace()
            .map(|x| x.parse::<f64>())
            .collect();
        
        let parts = parts.map_err(|_| UrdfParseError::InvalidStructure(format!("Invalid float array: {}", s)))?;
        
        if parts.len() != 3 {
            return Err(UrdfParseError::InvalidStructure(format!("Expected 3 values, got {}", parts.len())));
        }
        
        Ok([parts[0], parts[1], parts[2]])
    }

    fn parse_inertial(reader: &mut Reader<&[u8]>, _start_event: &quick_xml::events::BytesStart) -> Result<Inertial, UrdfParseError> {
        Self::skip_element(reader)?;
        Ok(Inertial {
            mass: 1.0,
            origin: None,
            inertia: None,
        })
    }

    fn parse_visual(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<Visual, UrdfParseError> {
        Self::skip_element(reader)?;
        Ok(Visual {
            name: Self::get_optional_attribute(start_event, b"name"),
            origin: None,
            geometry: None,
            material: None,
        })
    }

    fn parse_collision(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<Collision, UrdfParseError> {
        Self::skip_element(reader)?;
        Ok(Collision {
            name: Self::get_optional_attribute(start_event, b"name"),
            origin: None,
            geometry: None,
        })
    }

    fn parse_material(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<Material, UrdfParseError> {
        let name = Self::get_required_attribute(start_event, b"name")?;
        Self::skip_element(reader)?;
        Ok(Material {
            name,
            color: None,
            texture: None,
        })
    }

    fn parse_gazebo(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<GazeboElement, UrdfParseError> {
        let reference = Self::get_optional_attribute(start_event, b"reference");
        Self::skip_element(reader)?;
        Ok(GazeboElement {
            reference,
            content: String::new(),
        })
    }

    fn parse_transmission(reader: &mut Reader<&[u8]>, start_event: &quick_xml::events::BytesStart) -> Result<TransmissionElement, UrdfParseError> {
        let name = Self::get_required_attribute(start_event, b"name")?;
        Self::skip_element(reader)?;
        Ok(TransmissionElement {
            name,
            content: String::new(),
        })
    }

    fn skip_element(reader: &mut Reader<&[u8]>) -> Result<(), UrdfParseError> {
        let mut depth = 1;
        let mut buf = Vec::new();
        
        while depth > 0 {
            match reader.read_event_into(&mut buf)? {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(())
    }
}

pub fn validate_urdf_structure(doc: &UrdfDocument) -> Vec<String> {
    let mut issues = Vec::new();
    
    if doc.robot.name.is_empty() {
        issues.push("Robot name is empty".to_string());
    }
    
    if doc.robot.links.is_empty() {
        issues.push("No links defined in robot".to_string());
    }
    
    for joint in doc.robot.joints.values() {
        if !doc.robot.links.contains_key(&joint.parent) {
            issues.push(format!("Joint '{}' references non-existent parent link '{}'", joint.name, joint.parent));
        }
        if !doc.robot.links.contains_key(&joint.child) {
            issues.push(format!("Joint '{}' references non-existent child link '{}'", joint.name, joint.child));
        }
    }
    
    issues
}