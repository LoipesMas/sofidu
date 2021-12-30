use std::fmt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Node {
    pub path: PathBuf,
    pub size: u64,
    pub children: Vec<Node>,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_as_string_tree(0, None).0)
    }
}

impl Node {
    pub fn new(path: PathBuf, size: u64, children: Vec<Node>) -> Self {
        Self {
            path,
            size,
            children,
        }
    }
    pub fn get_as_string_line(&self) -> String {
        self.path
            .file_name()
            .map(|f| f.to_str())
            .flatten()
            .unwrap_or_else(|| self.path.to_str().unwrap_or("??"))
            .to_owned()
            + " == "
            + &file_size_to_str(self.size)
    }
    pub fn get_as_string_tree(&self, depth: usize, size_filter: Option<u64>) -> (String, bool) {
        let mut passed_filter = if let Some(size_filter) = size_filter {
            self.size >= size_filter
        } else {
            true
        };
        let mut result = "––".repeat(depth);
        result += " ";
        // Print full path of top node
        if depth == 0 {
            result += self
                .path
                .parent()
                .map(|p| p.to_str())
                .flatten()
                .unwrap_or("");
            result += "/";
        }
        result += &self.get_as_string_line();
        result += "\n";
        for child in &self.children {
            let child_res = child.get_as_string_tree(depth + 1, size_filter);
            if let Some(size_filter) = size_filter {
                if child_res.1 {
                    result += &child_res.0;
                    passed_filter = true;
                } else if child.size >= size_filter {
                    result += &("––".repeat(depth + 1) + " " + &child.get_as_string_line() + "\n");
                    passed_filter = true;
                }
            } else {
                result += &child_res.0;
            }
        }
        (result, passed_filter)
    }
}

pub fn walk_dir(path: &Path, depth: i32, follow_symlinks: bool) -> Node {
    let mut nodes: Vec<Node> = vec![];
    let mut total_size = 0;
    if let Ok(entries) = path.read_dir() {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let node = walk_dir(&entry.path(), depth - 1, follow_symlinks);
                    total_size += node.size;
                    if depth > 0 {
                        nodes.push(node);
                    }
                } else if file_type.is_file() {
                    let size = entry.metadata().map(|m| m.size()).unwrap_or(0);
                    total_size += size;
                    if depth > 0 {
                        nodes.push(Node::new(entry.path(), size, vec![]));
                    }
                }
            }
        }
    };
    Node::new(path.to_path_buf(), total_size, nodes)
}

pub fn file_size_to_str(size: u64) -> String {
    match size {
        0..=1024 => size.to_string() + "B",
        1025..=1048576 => {
            format!("{:.1}KB", size as f32 / 1024.0)
        }
        1048577..=1073741824 => {
            format!("{:.1}MB", size as f32 / 1048576.0)
        }
        1073741825.. => {
            format!("{:.1}GB", size as f32 / 1073741825.0)
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
                    "Invalid file size unit: {}.\n Supported filesizes: B, MB, KB, GB.",
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

    Ok((value * 1024u64.pow(exponent) as f32) as u64)
}
