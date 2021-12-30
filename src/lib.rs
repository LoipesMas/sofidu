use colored::*;
use rayon::prelude::*;
use std::fmt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Node {
    pub path: PathBuf,
    pub size: u64,
    pub children: Vec<Node>,
    pub is_dir: bool,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_as_string_tree(0, None).0)
    }
}

impl Node {
    pub fn new(path: PathBuf, size: u64, children: Vec<Node>) -> Self {
        Self {
            size,
            children,
            is_dir: path.is_dir(),
            path,
        }
    }
    pub fn get_as_string_line(&self, full_path: bool) -> String {
        let string = if full_path {
            self.path.to_str().unwrap_or("??")
        } else {
            self.path
                .file_name()
                .map(|f| f.to_str())
                .flatten()
                .unwrap_or_else(|| self.path.to_str().unwrap_or("??"))
        };
        let mut string = string.to_owned();
        let string = if self.is_dir {
            string += &std::path::MAIN_SEPARATOR.to_string();
            string.bright_blue()
        } else {
            string.cyan()
        };
        string.to_string() + " " + &file_size_to_str(self.size).green().to_string()
    }
    pub fn get_as_string_tree(&self, depth: usize, size_threshold: Option<u64>) -> (String, bool) {
        let mut passed_threshold = if let Some(size_threshold) = size_threshold {
            self.size >= size_threshold
        } else {
            true
        };
        let mut result = "| ".repeat(depth);
        result += &self.get_as_string_line(depth == 0);
        result += "\n";
        let (results, passed_thresholds): (Vec<_>, Vec<_>) = self
            .children
            .par_iter()
            .map(|child| {
                let child_res = child.get_as_string_tree(depth + 1, size_threshold);
                let mut child_out = "".to_owned();
                let mut passed_threshold = false;
                if let Some(size_threshold) = size_threshold {
                    if child_res.1 {
                        child_out += &child_res.0;
                        passed_threshold = true;
                    } else if child.size >= size_threshold {
                        child_out += &("| ".repeat(depth + 1)
                            + " "
                            + &child.get_as_string_line(false)
                            + "\n");
                        passed_threshold = true;
                    }
                } else {
                    child_out += &child_res.0;
                }
                (child_out, passed_threshold)
            })
            .unzip();
        result = results.iter().fold(result, |fold, r| fold + r);
        passed_threshold |= passed_thresholds.par_iter().any(|&p| p);
        (result, passed_threshold)
    }
    pub fn flatten(&self) -> Vec<Node> {
        let mut nodes = vec![self.clone_childless()];
        for child in &self.children {
            nodes.append(&mut child.flatten());
        }
        nodes
    }
    pub fn clone_childless(&self) -> Self {
        Self {
            path: self.path.clone(),
            is_dir: self.is_dir,
            size: self.size,
            children: vec![],
        }
    }

    pub fn sort(&mut self) {
        self.children.sort_unstable_by_key(|c| c.size);
        self.children.reverse();
        for child in self.children.iter_mut() {
            child.sort();
        }
    }
}

pub fn walk_dir(path: &Path, depth: i32, follow_symlinks: bool) -> Node {
    let mut nodes: Vec<Node> = vec![];
    let mut total_size = path.metadata().map(|m| m.size()).unwrap_or(0);
    if let Ok(entries) = path.read_dir() {
        let (children, sizes): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .par_bridge()
            .filter_map(|entry| {
                let mut node = None;
                let mut size = None;
                if let Ok(ref entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let node_temp = walk_dir(&entry.path(), depth - 1, follow_symlinks);
                            size = Some(node_temp.size);
                            if depth > 0 {
                                node = Some(node_temp);
                            }
                        } else if file_type.is_file() {
                            let size_temp = entry.metadata().map(|m| m.size()).unwrap_or(0);
                            size = Some(size_temp);
                            if depth > 0 {
                                node = Some(Node::new(entry.path(), size_temp, vec![]));
                            }
                        }
                    }
                };
                if node.is_some() || size.is_some() {
                    Some((node, size))
                } else {
                    None
                }
            })
            .unzip();
        children.into_iter().flatten().for_each(|child| {
            nodes.push(child);
        });
        total_size += sizes.into_par_iter().flatten().sum::<u64>();
    };
    Node::new(path.to_path_buf(), total_size, nodes)
}

pub fn file_size_to_str(size: u64) -> String {
    let exp = (size as f32).log10() as u32;
    match exp {
        0..=2 => size.to_string() + "B",
        3..=5 => {
            format!("{:.1}KB", size as f32 / 1000.0)
        }
        6..=8 => {
            format!("{:.1}MB", size as f32 / 1000000.0)
        }
        9.. => {
            format!("{:.1}GB", size as f32 / 1000000000.0)
        }
    }
}

pub fn str_to_file_size(input: &str) -> Result<u64, String> {
    let value;
    let mut exponent = 0;
    let pos = input.find(|c: char| c.is_ascii_alphabetic());
    if let Some(pos) = pos {
        let (value_s, unit) = input.split_at(pos);
        exponent = match unit.to_uppercase().as_str() {
            "G" | "GB" => 3,
            "M" | "MB" => 2,
            "K" | "KB" => 1,
            "" | "B" => 0,
            u => {
                return Err(format!(
                    "Invalid file size unit: {}.\n Supported file size units: B, MB, KB, GB.",
                    u
                ))
            }
        };
        if let Ok(v) = value_s.parse::<f32>() {
            value = v;
        } else {
            return Err(format!("Failed to parse value: {}", value_s));
        }
    } else if let Ok(v) = input.parse::<f32>() {
        value = v;
    } else {
        return Err(format!("Failed to parse value: {}", input));
    }

    Ok((value * 1000u64.pow(exponent) as f32) as u64)
}
