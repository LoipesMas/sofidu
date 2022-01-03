extern crate sofidu;

use std::env::args;

fn main() {
    // Parse arguments
    let settings = sofidu::AppSettings::from_args(args().collect());

    // Do the magic
    let mut node = sofidu::walk_dir(&settings.path, settings.depth, false);

    if settings.sort {
        node.sort();
    }
    let mut output = if settings.list {
        // Display as list
        let mut output = "".to_owned();
        let nodes = node.flatten();
        for node in nodes {
            if settings.only_files && node.is_dir {
                continue;
            }
            if let Some(size_threshold) = settings.threshold {
                if node.size < size_threshold {
                    continue;
                }
            }
            output += &node.get_as_string_line(true, settings.machine, None);
            output += "\n";
        }
        output
    } else {
        // Display as tree
        node.get_as_string_tree(0, settings.threshold, settings.machine, None)
            .0
    };
    if settings.reverse {
        // Not sure if this can be more concise
        output = output
            .lines()
            .rev()
            .map(|l| l.to_owned() + "\n")
            .collect::<String>();
    }
    println!("{}", output);
}
