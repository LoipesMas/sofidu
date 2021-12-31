# sofidu
An alternative to du(1), that lets you sort and filter the results.  
Created because I needed something like this and wanted to test my rust skills.

### Current functionality:
- Display files and folders in a tree-like structers with their sizes
- Display them as a list (`-l`)
- Sort by size (`-s`)(descending, or ascending with `-r`)
- Only show files and folders which have size above given threshold (`-t`)
- Select depth of displayed files/folders (`-d`)(e.g. show only files/folders that are at most X folders deep)
- Multithreading, thanks to [rayon](https://crates.io/crates/rayon)

### TODO:
- More tests
- Windows support
- More features? (need ideas)
