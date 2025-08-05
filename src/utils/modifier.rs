use crate::utils::parser::{UrdfDocument, Robot, Link, Joint, Material, UrdfParseError};
use crate::utils::processor::{UrdfProcessor, UrdfIssue, IssueSeverity};
use quick_xml::{Writer, events::Event, name::QName, events::BytesStart};
use std::io::Cursor;
use std::collections::{HashMap, HashSet};
use indexmap::IndexMap;

pub struct UrdfModifier;

#[derive(Debug, Clone)]
pub struct FixOptions {
    pub remove_duplicates: bool,
    pub fix_naming: bool,
    pub add_missing_properties: bool,
    pub clean_whitespace: bool,
    pub sort_elements: bool,
    pub remove_unused_materials: bool,
}

#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub indent: String,
    pub attribute_order: Vec<String>,
    pub element_order: Vec<String>,
    pub compact_empty_elements: bool,
    pub max_line_length: Option<usize>,
}

impl Default for FixOptions {
    fn default() -> Self {
        Self {
            remove_duplicates: true,
            fix_naming: false,
            add_missing_properties: false,
            clean_whitespace: true,
            sort_elements: false,
            remove_unused_materials: true,
        }
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            attribute_order: vec![
                "name".to_string(),
                "type".to_string(),
                "link".to_string(),
                "joint".to_string(),
                "xyz".to_string(),
                "rpy".to_string(),
            ],
            element_order: vec![
                "material".to_string(),
                "link".to_string(),
                "joint".to_string(),
                "gazebo".to_string(),
                "transmission".to_string(),
            ],
            compact_empty_elements: true,
            max_line_length: Some(120),
        }
    }
}

impl UrdfModifier {
    pub fn fix_document(&self, doc: &mut UrdfDocument, options: &FixOptions) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        
        if options.remove_duplicates {
            changes.extend(self.remove_duplicates(&mut doc.robot)?);
        }
        
        if options.remove_unused_materials {
            changes.extend(self.remove_unused_materials(&mut doc.robot)?);
        }
        
        if options.fix_naming {
            changes.extend(self.fix_naming_conventions(&mut doc.robot)?);
        }
        
        if options.add_missing_properties {
            changes.extend(self.add_missing_properties(&mut doc.robot)?);
        }
        
        if options.sort_elements {
            changes.extend(self.sort_elements(&mut doc.robot)?);
        }
        
        if options.clean_whitespace {
            self.regenerate_xml(doc)?;
            changes.push("Cleaned whitespace and formatting".to_string());
        }
        
        Ok(changes)
    }

    pub fn format_document(&self, doc: &mut UrdfDocument, options: &FormatOptions) -> Result<(), UrdfParseError> {
        self.regenerate_xml_with_formatting(doc, options)
    }

    pub fn apply_auto_fixes(&self, doc: &mut UrdfDocument, issues: &[UrdfIssue]) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        
        for issue in issues {
            match issue.severity {
                IssueSeverity::Error => {
                    if let Some(fix) = self.try_auto_fix(doc, issue)? {
                        changes.push(fix);
                    }
                }
                IssueSeverity::Warning => {
                    if let Some(fix) = self.try_auto_fix(doc, issue)? {
                        changes.push(fix);
                    }
                }
                _ => {}
            }
        }
        
        Ok(changes)
    }

    pub fn remove_element(&self, doc: &mut UrdfDocument, element_type: &str, name: &str) -> Result<bool, UrdfParseError> {
        match element_type {
            "link" => {
                if doc.robot.links.remove(name).is_some() {
                    self.regenerate_xml(doc)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            "joint" => {
                if doc.robot.joints.remove(name).is_some() {
                    self.regenerate_xml(doc)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            "material" => {
                if doc.robot.materials.remove(name).is_some() {
                    self.regenerate_xml(doc)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    pub fn rename_element(&self, doc: &mut UrdfDocument, element_type: &str, old_name: &str, new_name: &str) -> Result<bool, UrdfParseError> {
        match element_type {
            "link" => {
                if let Some(mut link) = doc.robot.links.remove(old_name) {
                    link.name = new_name.to_string();
                    
                    for joint in doc.robot.joints.values_mut() {
                        if joint.parent == old_name {
                            joint.parent = new_name.to_string();
                        }
                        if joint.child == old_name {
                            joint.child = new_name.to_string();
                        }
                    }
                    
                    doc.robot.links.insert(new_name.to_string(), link);
                    self.regenerate_xml(doc)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            "joint" => {
                if let Some(mut joint) = doc.robot.joints.remove(old_name) {
                    joint.name = new_name.to_string();
                    doc.robot.joints.insert(new_name.to_string(), joint);
                    self.regenerate_xml(doc)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            "material" => {
                if let Some(mut material) = doc.robot.materials.remove(old_name) {
                    material.name = new_name.to_string();
                    
                    for link in doc.robot.links.values_mut() {
                        for visual in &mut link.visual {
                            if let Some(material_ref) = &mut visual.material {
                                if material_ref.name == old_name {
                                    material_ref.name = new_name.to_string();
                                }
                            }
                        }
                    }
                    
                    doc.robot.materials.insert(new_name.to_string(), material);
                    self.regenerate_xml(doc)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    fn remove_duplicates(&self, robot: &mut Robot) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        
        let duplicate_links = self.find_duplicate_links(robot);
        for link_name in duplicate_links {
            robot.links.remove(&link_name);
            changes.push(format!("Removed duplicate link: {}", link_name));
        }
        
        let duplicate_joints = self.find_duplicate_joints(robot);
        for joint_name in duplicate_joints {
            robot.joints.remove(&joint_name);
            changes.push(format!("Removed duplicate joint: {}", joint_name));
        }
        
        let duplicate_materials = self.find_duplicate_materials(robot);
        for material_name in duplicate_materials {
            robot.materials.remove(&material_name);
            changes.push(format!("Removed duplicate material: {}", material_name));
        }
        
        Ok(changes)
    }

    fn remove_unused_materials(&self, robot: &mut Robot) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        let mut used_materials = HashSet::new();
        
        for link in robot.links.values() {
            for visual in &link.visual {
                if let Some(material) = &visual.material {
                    used_materials.insert(material.name.clone());
                }
            }
        }
        
        let unused_materials: Vec<String> = robot.materials.keys()
            .filter(|name| !used_materials.contains(*name))
            .cloned()
            .collect();
        
        for material_name in unused_materials {
            robot.materials.remove(&material_name);
            changes.push(format!("Removed unused material: {}", material_name));
        }
        
        Ok(changes)
    }

    fn fix_naming_conventions(&self, robot: &mut Robot) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        
        let bad_link_names: Vec<String> = robot.links.keys()
            .filter(|name| !self.is_valid_name(name))
            .cloned()
            .collect();
        
        for old_name in bad_link_names {
            let new_name = self.fix_name(&old_name);
            if new_name != old_name {
                if let Some(mut link) = robot.links.remove(&old_name) {
                    link.name = new_name.clone();
                    
                    for joint in robot.joints.values_mut() {
                        if joint.parent == old_name {
                            joint.parent = new_name.clone();
                        }
                        if joint.child == old_name {
                            joint.child = new_name.clone();
                        }
                    }
                    
                    robot.links.insert(new_name.clone(), link);
                    changes.push(format!("Fixed link name: {} -> {}", old_name, new_name));
                }
            }
        }
        
        let bad_joint_names: Vec<String> = robot.joints.keys()
            .filter(|name| !self.is_valid_name(name))
            .cloned()
            .collect();
        
        for old_name in bad_joint_names {
            let new_name = self.fix_name(&old_name);
            if new_name != old_name {
                if let Some(mut joint) = robot.joints.remove(&old_name) {
                    joint.name = new_name.clone();
                    robot.joints.insert(new_name.clone(), joint);
                    changes.push(format!("Fixed joint name: {} -> {}", old_name, new_name));
                }
            }
        }
        
        Ok(changes)
    }

    fn add_missing_properties(&self, robot: &mut Robot) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        
        for (name, link) in &mut robot.links {
            if link.inertial.is_none() && (!link.visual.is_empty() || !link.collision.is_empty()) {
                changes.push(format!("Would add default inertial properties to link: {}", name));
            }
        }
        
        Ok(changes)
    }

    fn sort_elements(&self, robot: &mut Robot) -> Result<Vec<String>, UrdfParseError> {
        let mut changes = Vec::new();
        
        let original_links_order: Vec<String> = robot.links.keys().cloned().collect();
        let mut sorted_links = IndexMap::new();
        let mut sorted_link_names = original_links_order.clone();
        sorted_link_names.sort();
        
        for name in sorted_link_names {
            if let Some(link) = robot.links.remove(&name) {
                sorted_links.insert(name, link);
            }
        }
        robot.links = sorted_links;
        
        if original_links_order != robot.links.keys().cloned().collect::<Vec<_>>() {
            changes.push("Sorted links alphabetically".to_string());
        }
        
        let original_joints_order: Vec<String> = robot.joints.keys().cloned().collect();
        let mut sorted_joints = IndexMap::new();
        let mut sorted_joint_names = original_joints_order.clone();
        sorted_joint_names.sort();
        
        for name in sorted_joint_names {
            if let Some(joint) = robot.joints.remove(&name) {
                sorted_joints.insert(name, joint);
            }
        }
        robot.joints = sorted_joints;
        
        if original_joints_order != robot.joints.keys().cloned().collect::<Vec<_>>() {
            changes.push("Sorted joints alphabetically".to_string());
        }
        
        Ok(changes)
    }

    fn try_auto_fix(&self, _doc: &mut UrdfDocument, issue: &UrdfIssue) -> Result<Option<String>, UrdfParseError> {
        Ok(None)
    }

    fn regenerate_xml(&self, doc: &mut UrdfDocument) -> Result<(), UrdfParseError> {
        let options = FormatOptions::default();
        self.regenerate_xml_with_formatting(doc, &options)
    }

    fn regenerate_xml_with_formatting(&self, doc: &mut UrdfDocument, options: &FormatOptions) -> Result<(), UrdfParseError> {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(Cursor::new(&mut buffer), options.indent.as_bytes(), options.indent.len());
        
        let mut robot_element = BytesStart::new("robot");
        robot_element.push_attribute(("name", doc.robot.name.as_str()));
        writer.write_event(Event::Start(robot_element.to_borrowed()))?;
        
        for material in doc.robot.materials.values() {
            self.write_material(&mut writer, material, options)?;
        }
        
        for link in doc.robot.links.values() {
            self.write_link(&mut writer, link, options)?;
        }
        
        for joint in doc.robot.joints.values() {
            self.write_joint(&mut writer, joint, options)?;
        }
        
        for gazebo in &doc.robot.gazebo_elements {
            self.write_gazebo(&mut writer, gazebo, options)?;
        }
        
        for transmission in &doc.robot.transmission_elements {
            self.write_transmission(&mut writer, transmission, options)?;
        }
        
        writer.write_event(Event::End(BytesStart::new("robot").to_end()))?;
        
        doc.raw_xml = String::from_utf8(buffer)
            .map_err(|e| UrdfParseError::InvalidStructure(format!("UTF-8 error: {}", e)))?;
        
        Ok(())
    }

    fn write_material(&self, writer: &mut Writer<Cursor<&mut Vec<u8>>>, material: &Material, _options: &FormatOptions) -> Result<(), UrdfParseError> {
        let mut element = BytesStart::new("material");
        element.push_attribute(("name", material.name.as_str()));
        
        if material.color.is_some() || material.texture.is_some() {
            writer.write_event(Event::Start(element.to_borrowed()))?;
            writer.write_event(Event::End(element.to_end()))?;
        } else {
            writer.write_event(Event::Empty(element.to_borrowed()))?;
        }
        
        Ok(())
    }

    fn write_link(&self, writer: &mut Writer<Cursor<&mut Vec<u8>>>, link: &Link, _options: &FormatOptions) -> Result<(), UrdfParseError> {
        let mut element = BytesStart::new("link");
        element.push_attribute(("name", link.name.as_str()));
        
        let has_content = link.inertial.is_some() || !link.visual.is_empty() || !link.collision.is_empty();
        
        if has_content {
            writer.write_event(Event::Start(element.to_borrowed()))?;
            writer.write_event(Event::End(element.to_end()))?;
        } else {
            writer.write_event(Event::Empty(element.to_borrowed()))?;
        }
        
        Ok(())
    }

    fn write_joint(&self, writer: &mut Writer<Cursor<&mut Vec<u8>>>, joint: &Joint, _options: &FormatOptions) -> Result<(), UrdfParseError> {
        let mut element = BytesStart::new("joint");
        element.push_attribute(("name", joint.name.as_str()));
        element.push_attribute(("type", joint.joint_type.as_str()));
        
        writer.write_event(Event::Start(element.to_borrowed()))?;
        
        let mut parent_element = BytesStart::new("parent");
        parent_element.push_attribute(("link", joint.parent.as_str()));
        writer.write_event(Event::Empty(parent_element.to_borrowed()))?;
        
        let mut child_element = BytesStart::new("child");
        child_element.push_attribute(("link", joint.child.as_str()));
        writer.write_event(Event::Empty(child_element.to_borrowed()))?;
        
        if let Some(origin) = &joint.origin {
            let mut origin_element = BytesStart::new("origin");
            origin_element.push_attribute(("xyz", format!("{} {} {}", origin.xyz[0], origin.xyz[1], origin.xyz[2]).as_str()));
            origin_element.push_attribute(("rpy", format!("{} {} {}", origin.rpy[0], origin.rpy[1], origin.rpy[2]).as_str()));
            writer.write_event(Event::Empty(origin_element.to_borrowed()))?;
        }
        
        if let Some(axis) = &joint.axis {
            let mut axis_element = BytesStart::new("axis");
            axis_element.push_attribute(("xyz", format!("{} {} {}", axis.xyz[0], axis.xyz[1], axis.xyz[2]).as_str()));
            writer.write_event(Event::Empty(axis_element.to_borrowed()))?;
        }
        
        writer.write_event(Event::End(element.to_end()))?;
        
        Ok(())
    }

    fn write_gazebo(&self, writer: &mut Writer<Cursor<&mut Vec<u8>>>, gazebo: &crate::utils::parser::GazeboElement, _options: &FormatOptions) -> Result<(), UrdfParseError> {
        let mut element = BytesStart::new("gazebo");
        if let Some(reference) = &gazebo.reference {
            element.push_attribute(("reference", reference.as_str()));
        }
        writer.write_event(Event::Empty(element.to_borrowed()))?;
        Ok(())
    }

    fn write_transmission(&self, writer: &mut Writer<Cursor<&mut Vec<u8>>>, transmission: &crate::utils::parser::TransmissionElement, _options: &FormatOptions) -> Result<(), UrdfParseError> {
        let mut element = BytesStart::new("transmission");
        element.push_attribute(("name", transmission.name.as_str()));
        writer.write_event(Event::Empty(element.to_borrowed()))?;
        Ok(())
    }

    fn find_duplicate_links(&self, robot: &Robot) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();
        
        for name in robot.links.keys() {
            if !seen.insert(name.clone()) {
                duplicates.push(name.clone());
            }
        }
        
        duplicates
    }

    fn find_duplicate_joints(&self, robot: &Robot) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();
        
        for name in robot.joints.keys() {
            if !seen.insert(name.clone()) {
                duplicates.push(name.clone());
            }
        }
        
        duplicates
    }

    fn find_duplicate_materials(&self, robot: &Robot) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();
        
        for name in robot.materials.keys() {
            if !seen.insert(name.clone()) {
                duplicates.push(name.clone());
            }
        }
        
        duplicates
    }

    fn is_valid_name(&self, name: &str) -> bool {
        !name.is_empty() 
            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit())
    }

    fn fix_name(&self, name: &str) -> String {
        let mut fixed = String::new();
        let mut chars = name.chars();
        
        if let Some(first_char) = chars.next() {
            if first_char.is_ascii_digit() {
                fixed.push('_');
            }
            if first_char.is_alphanumeric() || first_char == '_' {
                fixed.push(first_char.to_ascii_lowercase());
            }
        }
        
        for ch in chars {
            if ch.is_alphanumeric() || ch == '_' {
                fixed.push(ch.to_ascii_lowercase());
            } else if ch.is_whitespace() || ch == '-' {
                fixed.push('_');
            }
        }
        
        if fixed.is_empty() {
            fixed = "unnamed".to_string();
        }
        
        fixed
    }
}

pub fn clean_xml_whitespace(xml: &str) -> Result<String, UrdfParseError> {
    let lines: Vec<&str> = xml.lines().collect();
    let cleaned_lines: Vec<String> = lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    
    Ok(cleaned_lines.join("\n"))
}

pub fn validate_xml_structure(xml: &str) -> Result<(), UrdfParseError> {
    use quick_xml::Reader;
    
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Err(e) => return Err(UrdfParseError::XmlError(e)),
            _ => {}
        }
        buf.clear();
    }
    
    Ok(())
}