extern crate sofidu;
use clap::Arg;
use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version};
use std::path::Path;

fn main() {
    let app = app_from_crate!()
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
            Arg::with_name("size filter")
                .help("Only show files with size bigger than this")
                .long("size_filter")
                .takes_value(true)
                .short("f"),
        );

    let matches = app.get_matches();
    let depth_input = matches.value_of("depth").unwrap();
    let depth = parse_depth(depth_input);
    let path_str = matches.value_of("path").unwrap();
    let sort = matches.is_present("sort");
    let list = matches.is_present("list");
    let reverse = matches.is_present("reverse");
    let size_filter = matches.value_of("size filter").map(|a| {
        let r = sofidu::str_to_file_size(a);
        match r {
            Ok(v) => v,
            Err(m) => {
                println!("{}", m);
                std::process::exit(1)
            }
        }
    });

    let path = Path::new(path_str);
    if !path.exists() || !path.is_dir() {
        println!("Invalid path provided: {}", path_str);
        std::process::exit(1);
    }

    let mut node = sofidu::walk_dir(path, depth, false);
    if sort {
        node.sort();
    }
    let mut output = if list {
        let mut output = "".to_owned();
        let nodes = node.flatten();
        for node in nodes {
            if let Some(size_filter) = size_filter {
                if node.size < size_filter {
                    continue;
                }
            }
            output += &node.get_as_string_line(true);
            output += "\n";
        }
        output
    } else {
        node.get_as_string_tree(0, size_filter).0
    };
    if reverse {
        output = output
            .lines()
            .rev()
            .map(|l| l.to_owned() + "\n")
            .collect::<String>();
    }
    println!("{}", output);
}

fn parse_depth(input: &str) -> i32 {
    let mut depth = {
        if let Ok(depth) = input.parse::<i32>() {
            depth
        } else {
            println!(
                "Invalid depth provided, expected integer value, got '{}'",
                input
            );
            std::process::exit(1)
        }
    };
    if depth < -1 {
        println!("Depth must be 0 or greater or -1 for max depth");
        std::process::exit(1)
    }
    if depth == -1 {
        depth = i32::MAX;
    }
    depth
}
