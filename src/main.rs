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
        node.get_as_string_list(settings.only_files, settings.threshold, settings.machine)
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
