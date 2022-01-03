use colored::*;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

use clap::Arg;
use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version};

/// Represents a file or a directory
/// `size` for directories is computed at creation
/// `children` is a vec of nodes which are inside this directory (empty for non-dirs)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub path: PathBuf,
    pub size: u64,
    pub children: Vec<Node>,
    pub is_dir: bool,
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

    /// Gets a single line display for this node.
    /// Includes filename or full path, and size
    pub fn get_as_string_line(
        &self,
        full_path: bool,
        machine_readable: bool,
        parent_size: Option<u64>,
    ) -> String {
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
        let file_size_str = if machine_readable {
            self.size.to_string()
        } else {
            file_size_to_str(self.size)
        }
        .green();

        let percentage_string = match parent_size {
            None => "".to_string(),
            Some(parent_size) => {
                let percentage = match parent_size {
                    0 => 100.0, // If parent size is zero, just display 💯
                    v => (self.size as f32 / v as f32) * 100.0,
                };
                let string = format!(" {:.1}%", percentage);
                if percentage > 30.0 {
                    string.red().bold()
                } else if percentage > 16.0 {
                    string.bright_red()
                } else {
                    string.white()
                }
                .to_string()
            }
        };
        format!("{} {}{}", string, file_size_str, percentage_string)
    }

    /// Gets a recursive tree display for this node
    /// Returns the output string and bool representing whether this should pass the filter
    /// (threshold), so all the parent nodes should pass the filter too.
    pub fn get_as_string_tree(
        &self,
        depth: usize,
        size_threshold: Option<u64>,
        machine_readable: bool,
        parent_size: Option<u64>,
    ) -> (String, bool) {
        let mut passed_threshold = if let Some(size_threshold) = size_threshold {
            self.size >= size_threshold
        } else {
            true
        };

        // This is display indentation, could be replaced with something prettier
        let mut result = format!(
            "{}{}\n",
            "| ".repeat(depth),
            &self.get_as_string_line(depth == 0, machine_readable, parent_size)
        );

        // This part is kinda wacky, but it had to be for parallelism
        let (results, passed_thresholds): (Vec<_>, Vec<_>) = self
            .children
            .par_iter()
            .map(|child| {
                let child_res = child.get_as_string_tree(
                    depth + 1,
                    size_threshold,
                    machine_readable,
                    Some(self.size),
                );
                let mut child_out = "".to_owned();
                let mut passed_threshold = false;
                if let Some(size_threshold) = size_threshold {
                    // Something deeper passed threshold so this node does too
                    if child_res.1 {
                        child_out += &child_res.0;
                        passed_threshold = true;
                    }
                    // This node passes the threshold by itself
                    else if child.size >= size_threshold {
                        child_out += &format!(
                            "{} {}\n",
                            "| ".repeat(depth + 1),
                            child.get_as_string_line(false, machine_readable, Some(self.size))
                        );
                        passed_threshold = true;
                    }
                } else {
                    child_out += &child_res.0;
                }
                (child_out, passed_threshold)
            })
            .unzip(); // Vec of tuples to tuple of vecs

        // Concat all results
        result = results.iter().fold(result, |fold, r| fold + r);
        passed_threshold |= passed_thresholds.par_iter().any(|&p| p);
        (result, passed_threshold)
    }

    /// Returns a string that lists all of the nodes,
    /// that are subnodes of self
    pub fn get_as_string_list(
        &self,
        only_files: bool,
        size_threshold: Option<u64>,
        machine_readable: bool,
    ) -> String {
        let mut output = "".to_owned();
        let nodes = self.flatten();
        for node in nodes {
            if only_files && node.is_dir {
                continue;
            }
            if let Some(size_threshold) = size_threshold {
                if node.size < size_threshold {
                    continue;
                }
            }
            output += &node.get_as_string_line(true, machine_readable, None);
            output += "\n";
        }
        output
    }

    /// Turns a tree of nodes into a flat vec of nodes
    pub fn flatten(&self) -> Vec<Node> {
        let mut nodes = vec![self.clone_childless()];
        for child in &self.children {
            nodes.append(&mut child.flatten());
        }
        nodes
    }
    /// Returns a clone of this node but without children
    pub fn clone_childless(&self) -> Self {
        Self {
            path: self.path.clone(),
            is_dir: self.is_dir,
            size: self.size,
            children: vec![],
        }
    }

    /// Sort all nodes in the tree by size descending
    pub fn sort(&mut self) {
        self.children.sort_unstable_by_key(|c| c.size);
        self.children.reverse();
        for child in self.children.iter_mut() {
            child.sort();
        }
    }
}

/// Walks a directory recursively, creating nodes along the way
pub fn walk_dir(path: &Path, depth: i32, follow_symlinks: bool) -> Node {
    let mut nodes: Vec<Node> = vec![];

    let mut total_size = path.metadata().map(|m| m.len()).unwrap_or(0);

    if let Ok(entries) = path.read_dir() {
        // Walk over children
        let (children, sizes): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .par_bridge()
            .filter_map(|entry| {
                let mut node = None;
                let mut size = None;
                if let Ok(ref entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            // Walk this dir recursively
                            let node_temp = walk_dir(&entry.path(), depth - 1, follow_symlinks);
                            size = Some(node_temp.size);
                            if depth > 0 {
                                // If not too deep, store it
                                node = Some(node_temp);
                            }
                        } else if file_type.is_file() {
                            // Get size for this file
                            let size_temp = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            size = Some(size_temp);
                            if depth > 0 {
                                // If not too deep, store it
                                node = Some(Node::new(entry.path(), size_temp, vec![]));
                            }
                        }
                    }
                };
                if node.is_some() || size.is_some() {
                    // Store the results
                    Some((node, size))
                } else {
                    // Filter out if both are none
                    None
                }
            })
            .unzip(); // Vec of tuples to tuple of vecs

        // Append all new children
        children.into_iter().flatten().for_each(|child| {
            nodes.push(child);
        });
        // Add up all sizes of children
        total_size += sizes.into_par_iter().flatten().sum::<u64>();
    };
    Node::new(path.to_path_buf(), total_size, nodes)
}

pub struct AppSettings {
    pub path: PathBuf,
    pub depth: i32,
    pub sort: bool,
    pub reverse: bool,
    pub list: bool,
    pub machine: bool,
    pub only_files: bool,
    pub threshold: Option<u64>,
}

impl AppSettings {
    /// Parses arguments using clap to AppSettings
    pub fn from_args(args: Vec<String>) -> Self {
        let clap_color_setting = if std::env::var_os("NO_COLOR").is_none() {
            clap::AppSettings::ColoredHelp
        } else {
            clap::AppSettings::ColorNever
        };
        let app = app_from_crate!()
            .setting(clap_color_setting)
            .arg(
                Arg::with_name("path")
                    .help("Path to directory to walk. Current directory by default.")
                    .default_value("."),
            )
            .arg(
                Arg::with_name("depth")
                    .help("Depth of displayed tree/list")
                    .long("depth")
                    .default_value("-1")
                    .takes_value(true)
                    .short("d"),
            )
            .arg(
                Arg::with_name("sort")
                    .help("Sort entries by size")
                    .long("sort")
                    .short("s"),
            )
            .arg(
                Arg::with_name("reverse")
                    .help("Reverse the output")
                    .long("reverse")
                    .short("r"),
            )
            .arg(
                Arg::with_name("list")
                    .help("Display entries as a list instead of a tree")
                    .long("list")
                    .short("l"),
            )
            .arg(
                Arg::with_name("machine")
                    .help("Display sizes in bytes (\"machine readable\")")
                    .long("machine-readable")
                    .short("m"),
            )
            .arg(
                Arg::with_name("only files")
                    .help("Display only files")
                    .long("only_files")
                    .requires("list")
                    .short("f"),
            )
            .arg(
                Arg::with_name("threshold")
                    .value_name("thresh")
                    .help("Only show files with size bigger than this (only for list view)")
                    .long("threshold")
                    .takes_value(true)
                    .short("t"),
            );

        // Get argument matches
        let matches = app.get_matches_from(args);
        let depth_input = matches.value_of("depth").unwrap();
        let depth = match parse_depth(depth_input) {
            Ok(v) => v,
            Err(m) => {
                println!("{}", m);
                std::process::exit(1)
            }
        };
        let path_str = matches.value_of("path").unwrap();
        let sort = matches.is_present("sort");
        let list = matches.is_present("list");
        let only_files = matches.is_present("only files");
        let machine = matches.is_present("machine");
        let reverse = matches.is_present("reverse");
        let threshold = matches.value_of("threshold").map(|a| {
            let r = str_to_file_size(a);
            match r {
                Ok(v) => v,
                Err(m) => {
                    println!("{}", m);
                    std::process::exit(1)
                }
            }
        });

        // Check if path is valid
        let path = PathBuf::from(path_str);
        if !path.exists() || !path.is_dir() {
            println!("Invalid path provided: {}", path_str);
            std::process::exit(1);
        }

        Self {
            path,
            depth,
            list,
            sort,
            only_files,
            machine,
            threshold,
            reverse,
        }
    }
}

/// Parses depth a from str
fn parse_depth(input: &str) -> Result<i32, String> {
    let mut depth = {
        if let Ok(depth) = input.parse::<i32>() {
            depth
        } else {
            return Err(format!(
                "Invalid depth provided, expected integer value, got '{}'",
                input
            ));
        }
    };
    if depth < -1 {
        return Err("Depth must be 0 or greater or -1 for max depth".to_string());
    }
    if depth == -1 {
        depth = i32::MAX;
    }
    Ok(depth)
}

/// Converts file size in bytes to human readable string
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

/// Converts human readable string to number of bytes
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

#[cfg(test)]
mod lib_tests {
    use super::*;
    #[test]
    fn file_size_to_str_test() {
        assert_eq!("1B", file_size_to_str(1));
        assert_eq!("999B", file_size_to_str(999));
        assert_eq!("1.0KB", file_size_to_str(1_000));
        assert_eq!("2.1KB", file_size_to_str(2_100));
        assert_eq!("1.0MB", file_size_to_str(1_000_000));
        assert_eq!("4.2MB", file_size_to_str(4_233_333));
        assert_eq!("5.0GB", file_size_to_str(5_000_000_000));
    }

    #[test]
    fn str_to_file_size_test() {
        assert_eq!(1, str_to_file_size("1").unwrap());
        assert_eq!(1, str_to_file_size("1.1").unwrap());
        assert_eq!(999, str_to_file_size("999B").unwrap());
        assert_eq!(1_000, str_to_file_size("1KB").unwrap());
        assert_eq!(1_000, str_to_file_size("1K").unwrap());
        assert_eq!(1_000, str_to_file_size("1.0KB").unwrap());
        assert_eq!(1_000, str_to_file_size("1.0K").unwrap());
        assert_eq!(4_200, str_to_file_size("4.2KB").unwrap());
        assert_eq!(1_000_000, str_to_file_size("1.0MB").unwrap());
        assert_eq!(1_000_000, str_to_file_size("1.0M").unwrap());
        assert_eq!(4_200_000, str_to_file_size("4.2MB").unwrap());
        assert_eq!(5_000_000_000, str_to_file_size("5.0GB").unwrap());
        assert_eq!(5_000_000_000, str_to_file_size("5.0G").unwrap());
        assert!(str_to_file_size("5..GB").is_err());
        assert!(str_to_file_size("5..TB").is_err());
        assert!(str_to_file_size("").is_err());
    }

    #[test]
    fn node_sort_test() {
        let node_1 = Node::new(PathBuf::from("foo"), 0, vec![]);
        let node_2 = Node::new(PathBuf::from("bar"), 1, vec![]);
        let node_3 = Node::new(PathBuf::from("baz"), 100, vec![]);
        let children = vec![node_1.clone(), node_3.clone(), node_2.clone()];

        let mut node = Node::new(PathBuf::from("quaz"), 0, children);

        node.sort();

        let children_out = vec![node_3, node_2, node_1];
        assert_eq!(children_out, node.children);
    }

    #[test]
    fn node_sort_test_recursive() {
        let node_1 = Node::new(PathBuf::from("foo"), 0, vec![]);
        let node_2 = Node::new(PathBuf::from("bar"), 1, vec![]);
        let mut node_3 = Node::new(
            PathBuf::from("baz"),
            100,
            vec![node_1.clone_childless(), node_2.clone_childless()],
        );

        let children = vec![node_1.clone(), node_3.clone(), node_2.clone()];
        node_3.sort();

        let mut node = Node::new(PathBuf::from("quaz"), 0, children);

        let children_out = vec![node_3, node_2, node_1];
        node.sort();

        assert_eq!(children_out, node.children);
    }

    #[test]
    fn node_as_string_line_test() {
        // Disable coloring
        colored::control::set_override(false);
        let node = Node::new(PathBuf::from("foo"), 3_233_333, vec![]);
        assert_eq!("foo 3.2MB", node.get_as_string_line(false, false, None));
        let node = Node::new(PathBuf::from("src"), 3_233_333, vec![]);
        assert_eq!("src/ 3.2MB", node.get_as_string_line(false, false, None));
        let node = Node::new(PathBuf::from("src/main.rs"), 3_233_333, vec![]);
        assert_eq!(
            "src/main.rs 3.2MB",
            node.get_as_string_line(true, false, None)
        );
    }

    #[test]
    fn node_as_string_line_test_machine_readable() {
        // Disable coloring
        colored::control::set_override(false);
        let node = Node::new(PathBuf::from("foo"), 3_233_333, vec![]);
        assert_eq!("foo 3233333", node.get_as_string_line(false, true, None));
        let node = Node::new(PathBuf::from("foo"), 3, vec![]);
        assert_eq!("foo 3", node.get_as_string_line(false, true, None));
    }

    #[test]
    fn node_as_string_tree_test() {
        colored::control::set_override(false);
        let node_1_1 = Node::new(PathBuf::from("foo/bar/biz"), 333, vec![]);
        let node_1 = Node::new(PathBuf::from("foo/bar"), 4_333, vec![node_1_1]);
        let node_2_1 = Node::new(PathBuf::from("foo/baz/qiz"), 1_233_333, vec![]);
        let node_2 = Node::new(PathBuf::from("foo/baz"), 2_233_333, vec![node_2_1]);
        let node_top = Node::new(PathBuf::from("foo"), 3_666_233_333, vec![node_1, node_2]);

        assert_eq!(
            "foo 3.7GB\n| bar 4.3KB 0.0%\n| | biz 333B 7.7%\n| baz 2.2MB 0.1%\n| | qiz 1.2MB 55.2%\n",
            node_top.get_as_string_tree(0, None, false, None).0
        );
        assert_eq!(
            "foo 3.7GB\n| baz 2.2MB 0.1%\n| | qiz 1.2MB 55.2%\n",
            node_top
                .get_as_string_tree(0, Some(1_000_000), false, None)
                .0
        );
        assert_eq!(
            "foo 3.7GB\n| bar 4.3KB 0.0%\n| baz 2.2MB 0.1%\n| | qiz 1.2MB 55.2%\n",
            node_top.get_as_string_tree(0, Some(4_000), false, None).0
        );
    }

    #[test]
    fn node_flatten_test() {
        let node_1_1 = Node::new(PathBuf::from("foo/bar/biz"), 4_333, vec![]);
        let node_1 = Node::new(PathBuf::from("foo/bar"), 333, vec![node_1_1.clone()]);
        let node_2 = Node::new(PathBuf::from("foo/baz"), 3_000_233_333, vec![]);
        let node_top = Node::new(
            PathBuf::from("foo"),
            3_233_333,
            vec![node_1.clone(), node_2.clone()],
        );
        let result = vec![
            node_top.clone_childless(),
            node_1.clone_childless(),
            node_1_1.clone_childless(),
            node_2.clone_childless(),
        ];
        assert_eq!(result, node_top.flatten());
    }

    #[test]
    fn node_clone_childless_test() {
        let node_1_1 = Node::new(PathBuf::from("foo/bar/biz"), 4_333, vec![]);
        let node_1 = Node::new(PathBuf::from("foo/bar"), 333, vec![node_1_1]);
        let node_2 = Node::new(PathBuf::from("foo/baz"), 3_000_233_333, vec![]);
        let node_top = Node::new(PathBuf::from("foo"), 3_233_333, vec![node_1, node_2]);

        assert_eq!(Vec::<Node>::new(), node_top.clone_childless().children);
    }

    #[test]
    fn parse_depth_test() {
        assert_eq!(i32::MAX, parse_depth("-1").unwrap());
        assert_eq!(1, parse_depth("1").unwrap());
        assert!(parse_depth("-2").is_err());
        assert!(parse_depth("foo").is_err());
    }

    #[test]
    fn parse_arguments_test() {
        let arguments = "sofidu -d 10 -s -r -l -m -f -t 1gb src";
        let settings =
            AppSettings::from_args(arguments.split(' ').map(|a| a.to_string()).collect());
        assert_eq!(10, settings.depth);
        assert!(settings.sort);
        assert!(settings.reverse);
        assert!(settings.list);
        assert!(settings.machine);
        assert!(settings.only_files);
        assert_eq!(Some(1_000_000_000), settings.threshold);
        assert_eq!(PathBuf::from("src"), settings.path);
    }
}
