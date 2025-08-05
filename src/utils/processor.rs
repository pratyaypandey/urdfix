use crate::utils::parser::{UrdfDocument, Robot, Link, Joint, Material, UrdfParseError};
use std::collections::{HashMap, HashSet};
use indexmap::IndexMap;

pub struct UrdfProcessor;

#[derive(Debug, Clone)]
pub struct UrdfStats {
    pub total_links: usize,
    pub total_joints: usize,
    pub total_materials: usize,
    pub joint_types: HashMap<String, usize>,
    pub link_properties: LinkProperties,
    pub tree_depth: usize,
    pub kinematic_chains: Vec<KinematicChain>,
}

#[derive(Debug, Clone)]
pub struct LinkProperties {
    pub with_visual: usize,
    pub with_collision: usize,
    pub with_inertial: usize,
    pub empty_links: usize,
}

#[derive(Debug, Clone)]
pub struct KinematicChain {
    pub name: String,
    pub links: Vec<String>,
    pub joints: Vec<String>,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub struct UrdfIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub element_name: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueCategory {
    Structure,
    Naming,
    Physics,
    Geometry,
    Validation,
    Style,
}

impl UrdfProcessor {
    pub fn analyze(&self, doc: &UrdfDocument) -> UrdfStats {
        let robot = &doc.robot;
        
        let joint_types = self.count_joint_types(robot);
        let link_properties = self.analyze_link_properties(robot);
        let tree_depth = self.calculate_tree_depth(robot);
        let kinematic_chains = self.find_kinematic_chains(robot);
        
        UrdfStats {
            total_links: robot.links.len(),
            total_joints: robot.joints.len(),
            total_materials: robot.materials.len(),
            joint_types,
            link_properties,
            tree_depth,
            kinematic_chains,
        }
    }

    pub fn lint(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        
        issues.extend(self.check_naming_conventions(doc));
        issues.extend(self.check_structural_issues(doc));
        issues.extend(self.check_physics_properties(doc));
        issues.extend(self.check_duplicate_elements(doc));
        issues.extend(self.check_unused_materials(doc));
        issues.extend(self.check_joint_limits(doc));
        
        issues
    }

    pub fn find_duplicates(&self, doc: &UrdfDocument) -> HashMap<String, Vec<String>> {
        let mut duplicates = HashMap::new();
        
        let link_names: Vec<&String> = doc.robot.links.keys().collect();
        let joint_names: Vec<&String> = doc.robot.joints.keys().collect();
        let material_names: Vec<&String> = doc.robot.materials.keys().collect();
        
        self.find_duplicate_names("links", &link_names, &mut duplicates);
        self.find_duplicate_names("joints", &joint_names, &mut duplicates);
        self.find_duplicate_names("materials", &material_names, &mut duplicates);
        
        duplicates
    }

    pub fn get_dependency_graph(&self, doc: &UrdfDocument) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        
        for joint in doc.robot.joints.values() {
            graph.entry(joint.parent.clone())
                .or_insert_with(Vec::new)
                .push(joint.child.clone());
        }
        
        graph
    }

    pub fn find_root_links(&self, doc: &UrdfDocument) -> Vec<String> {
        let mut child_links: HashSet<String> = HashSet::new();
        
        for joint in doc.robot.joints.values() {
            child_links.insert(joint.child.clone());
        }
        
        doc.robot.links.keys()
            .filter(|link_name| !child_links.contains(*link_name))
            .cloned()
            .collect()
    }

    pub fn find_leaf_links(&self, doc: &UrdfDocument) -> Vec<String> {
        let mut parent_links: HashSet<String> = HashSet::new();
        
        for joint in doc.robot.joints.values() {
            parent_links.insert(joint.parent.clone());
        }
        
        doc.robot.links.keys()
            .filter(|link_name| !parent_links.contains(*link_name))
            .cloned()
            .collect()
    }

    pub fn validate_kinematic_tree(&self, doc: &UrdfDocument) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        let root_links = self.find_root_links(doc);
        if root_links.len() != 1 {
            errors.push(format!("Expected exactly 1 root link, found {}: {:?}", root_links.len(), root_links));
        }
        
        if self.has_cycles(doc) {
            errors.push("Kinematic tree contains cycles".to_string());
        }
        
        let orphaned_links = self.find_orphaned_links(doc);
        if !orphaned_links.is_empty() {
            errors.push(format!("Found orphaned links: {:?}", orphaned_links));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn count_joint_types(&self, robot: &Robot) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        
        for joint in robot.joints.values() {
            *counts.entry(joint.joint_type.clone()).or_insert(0) += 1;
        }
        
        counts
    }

    fn analyze_link_properties(&self, robot: &Robot) -> LinkProperties {
        let mut props = LinkProperties {
            with_visual: 0,
            with_collision: 0,
            with_inertial: 0,
            empty_links: 0,
        };
        
        for link in robot.links.values() {
            if !link.visual.is_empty() {
                props.with_visual += 1;
            }
            if !link.collision.is_empty() {
                props.with_collision += 1;
            }
            if link.inertial.is_some() {
                props.with_inertial += 1;
            }
            if link.visual.is_empty() && link.collision.is_empty() && link.inertial.is_none() {
                props.empty_links += 1;
            }
        }
        
        props
    }

    fn calculate_tree_depth(&self, robot: &Robot) -> usize {
        let graph = self.build_adjacency_list(robot);
        let root_links = self.find_root_links_from_robot(robot);
        
        if root_links.is_empty() {
            return 0;
        }
        
        let mut max_depth = 0;
        for root in &root_links {
            let depth = self.dfs_depth(&graph, root, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }
        
        max_depth
    }

    fn find_kinematic_chains(&self, robot: &Robot) -> Vec<KinematicChain> {
        let mut chains = Vec::new();
        let graph = self.build_adjacency_list(robot);
        let leaf_links = self.find_leaf_links_from_robot(robot);
        let root_links = self.find_root_links_from_robot(robot);
        
        for leaf in &leaf_links {
            for root in &root_links {
                if let Some(path) = self.find_path(&graph, root, leaf, robot) {
                    chains.push(path);
                }
            }
        }
        
        chains
    }

    fn check_naming_conventions(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        
        for link_name in doc.robot.links.keys() {
            if !self.is_valid_name(link_name) {
                issues.push(UrdfIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Naming,
                    message: format!("Link name '{}' doesn't follow naming conventions", link_name),
                    element_name: Some(link_name.clone()),
                    suggestion: Some("Use snake_case with descriptive names".to_string()),
                });
            }
        }
        
        for joint_name in doc.robot.joints.keys() {
            if !self.is_valid_name(joint_name) {
                issues.push(UrdfIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Naming,
                    message: format!("Joint name '{}' doesn't follow naming conventions", joint_name),
                    element_name: Some(joint_name.clone()),
                    suggestion: Some("Use snake_case with descriptive names".to_string()),
                });
            }
        }
        
        issues
    }

    fn check_structural_issues(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        
        if let Err(validation_errors) = self.validate_kinematic_tree(doc) {
            for error in validation_errors {
                issues.push(UrdfIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Structure,
                    message: error,
                    element_name: None,
                    suggestion: Some("Fix kinematic tree structure".to_string()),
                });
            }
        }
        
        issues
    }

    fn check_physics_properties(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        
        for (name, link) in &doc.robot.links {
            if link.inertial.is_none() && (!link.visual.is_empty() || !link.collision.is_empty()) {
                issues.push(UrdfIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Physics,
                    message: format!("Link '{}' has geometry but no inertial properties", name),
                    element_name: Some(name.clone()),
                    suggestion: Some("Add inertial properties for physics simulation".to_string()),
                });
            }
        }
        
        issues
    }

    fn check_duplicate_elements(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        let duplicates = self.find_duplicates(doc);
        
        for (category, names) in duplicates {
            if names.len() > 1 {
                issues.push(UrdfIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Validation,
                    message: format!("Duplicate {} found: {:?}", category, names),
                    element_name: None,
                    suggestion: Some("Remove or rename duplicate elements".to_string()),
                });
            }
        }
        
        issues
    }

    fn check_unused_materials(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        let mut used_materials = HashSet::new();
        
        for link in doc.robot.links.values() {
            for visual in &link.visual {
                if let Some(material) = &visual.material {
                    used_materials.insert(material.name.clone());
                }
            }
        }
        
        for material_name in doc.robot.materials.keys() {
            if !used_materials.contains(material_name) {
                issues.push(UrdfIssue {
                    severity: IssueSeverity::Info,
                    category: IssueCategory::Style,
                    message: format!("Unused material: '{}'", material_name),
                    element_name: Some(material_name.clone()),
                    suggestion: Some("Remove unused material or add reference".to_string()),
                });
            }
        }
        
        issues
    }

    fn check_joint_limits(&self, doc: &UrdfDocument) -> Vec<UrdfIssue> {
        let mut issues = Vec::new();
        
        for (name, joint) in &doc.robot.joints {
            if joint.joint_type == "revolute" || joint.joint_type == "prismatic" {
                if joint.limit.is_none() {
                    issues.push(UrdfIssue {
                        severity: IssueSeverity::Warning,
                        category: IssueCategory::Physics,
                        message: format!("Joint '{}' of type '{}' is missing limit specification", name, joint.joint_type),
                        element_name: Some(name.clone()),
                        suggestion: Some("Add limit element with upper, lower, effort, and velocity".to_string()),
                    });
                }
            }
        }
        
        issues
    }

    fn find_duplicate_names(&self, category: &str, names: &[&String], duplicates: &mut HashMap<String, Vec<String>>) {
        let mut name_counts = HashMap::new();
        
        for name in names {
            *name_counts.entry((*name).clone()).or_insert(0) += 1;
        }
        
        let duplicate_names: Vec<String> = name_counts.iter()
            .filter(|(_, &count)| count > 1)
            .map(|(name, _)| name.clone())
            .collect();
        
        if !duplicate_names.is_empty() {
            duplicates.insert(category.to_string(), duplicate_names);
        }
    }

    fn has_cycles(&self, doc: &UrdfDocument) -> bool {
        let graph = self.build_adjacency_list(&doc.robot);
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for link in doc.robot.links.keys() {
            if !visited.contains(link) {
                if self.dfs_has_cycle(&graph, link, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        
        false
    }

    fn find_orphaned_links(&self, doc: &UrdfDocument) -> Vec<String> {
        let mut connected_links = HashSet::new();
        
        for joint in doc.robot.joints.values() {
            connected_links.insert(joint.parent.clone());
            connected_links.insert(joint.child.clone());
        }
        
        doc.robot.links.keys()
            .filter(|link_name| !connected_links.contains(*link_name))
            .cloned()
            .collect()
    }

    fn build_adjacency_list(&self, robot: &Robot) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        
        for joint in robot.joints.values() {
            graph.entry(joint.parent.clone())
                .or_insert_with(Vec::new)
                .push(joint.child.clone());
        }
        
        graph
    }

    fn find_root_links_from_robot(&self, robot: &Robot) -> Vec<String> {
        let mut child_links: HashSet<String> = HashSet::new();
        
        for joint in robot.joints.values() {
            child_links.insert(joint.child.clone());
        }
        
        robot.links.keys()
            .filter(|link_name| !child_links.contains(*link_name))
            .cloned()
            .collect()
    }

    fn find_leaf_links_from_robot(&self, robot: &Robot) -> Vec<String> {
        let mut parent_links: HashSet<String> = HashSet::new();
        
        for joint in robot.joints.values() {
            parent_links.insert(joint.parent.clone());
        }
        
        robot.links.keys()
            .filter(|link_name| !parent_links.contains(*link_name))
            .cloned()
            .collect()
    }

    fn dfs_depth(&self, graph: &HashMap<String, Vec<String>>, node: &str, visited: &mut HashSet<String>) -> usize {
        if visited.contains(node) {
            return 0;
        }
        
        visited.insert(node.to_string());
        let mut max_depth = 0;
        
        if let Some(children) = graph.get(node) {
            for child in children {
                let depth = self.dfs_depth(graph, child, visited);
                max_depth = max_depth.max(depth);
            }
        }
        
        max_depth + 1
    }

    fn dfs_has_cycle(&self, graph: &HashMap<String, Vec<String>>, node: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<String>) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        
        if let Some(children) = graph.get(node) {
            for child in children {
                if !visited.contains(child) {
                    if self.dfs_has_cycle(graph, child, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(child) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(node);
        false
    }

    fn find_path(&self, graph: &HashMap<String, Vec<String>>, start: &str, end: &str, robot: &Robot) -> Option<KinematicChain> {
        let mut path = Vec::new();
        let mut visited = HashSet::new();
        
        if self.dfs_path(graph, start, end, &mut path, &mut visited) {
            let joints = self.get_joints_in_path(&path, robot);
            Some(KinematicChain {
                name: format!("{}_to_{}", start, end),
                links: path,
                joints,
                length: path.len(),
            })
        } else {
            None
        }
    }

    fn dfs_path(&self, graph: &HashMap<String, Vec<String>>, current: &str, target: &str, path: &mut Vec<String>, visited: &mut HashSet<String>) -> bool {
        path.push(current.to_string());
        visited.insert(current.to_string());
        
        if current == target {
            return true;
        }
        
        if let Some(children) = graph.get(current) {
            for child in children {
                if !visited.contains(child) {
                    if self.dfs_path(graph, child, target, path, visited) {
                        return true;
                    }
                }
            }
        }
        
        path.pop();
        false
    }

    fn get_joints_in_path(&self, path: &[String], robot: &Robot) -> Vec<String> {
        let mut joints = Vec::new();
        
        for i in 0..path.len() - 1 {
            let parent = &path[i];
            let child = &path[i + 1];
            
            for (joint_name, joint) in &robot.joints {
                if joint.parent == *parent && joint.child == *child {
                    joints.push(joint_name.clone());
                    break;
                }
            }
        }
        
        joints
    }

    fn is_valid_name(&self, name: &str) -> bool {
        !name.is_empty() 
            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit())
    }
}