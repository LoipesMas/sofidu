extern crate sofidu;
use clap::Arg;
use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version};
use std::path::Path;

fn main() {
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
    let matches = app.get_matches();
    let depth_input = matches.value_of("depth").unwrap();
    let depth = parse_depth(depth_input);
    let path_str = matches.value_of("path").unwrap();
    let sort = matches.is_present("sort");
    let list = matches.is_present("list");
    let only_files = matches.is_present("only files");
    let machine = matches.is_present("machine");
    let reverse = matches.is_present("reverse");
    let size_threshold = matches.value_of("threshold").map(|a| {
        let r = sofidu::str_to_file_size(a);
        match r {
            Ok(v) => v,
            Err(m) => {
                println!("{}", m);
                std::process::exit(1)
            }
        }
    });

    // Check if path is valid
    let path = Path::new(path_str);
    if !path.exists() || !path.is_dir() {
        println!("Invalid path provided: {}", path_str);
        std::process::exit(1);
    }

    // Do the magic
    let mut node = sofidu::walk_dir(path, depth, false);

    if sort {
        node.sort();
    }
    let mut output = if list {
        // Display as list
        let mut output = "".to_owned();
        let nodes = node.flatten();
        for node in nodes {
            if only_files && node.is_dir {
                continue;
            }
            if let Some(size_threshold) = size_threshold {
                if node.size < size_threshold {
                    continue;
                }
            }
            output += &node.get_as_string_line(true, machine);
            output += "\n";
        }
        output
    } else {
        // Display as tree
        node.get_as_string_tree(0, size_threshold, machine).0
    };
    if reverse {
        // Not sure if this can be more concise
        output = output
            .lines()
            .rev()
            .map(|l| l.to_owned() + "\n")
            .collect::<String>();
    }
    println!("{}", output);
}

/// Parses depth a from str
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
