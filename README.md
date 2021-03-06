![sofidu_banner](https://user-images.githubusercontent.com/46327403/147984833-684f07a0-4d7e-4627-b530-7bda128861fa.png)   
[![Build and test](https://github.com/LoipesMas/sofidu/actions/workflows/rust.yml/badge.svg)](https://github.com/LoipesMas/sofidu/actions/workflows/rust.yml)  
An alternative to du(1), that lets you sort and filter the results.  
Created because I needed something like this and wanted to test my rust skills.

---  
![image](https://user-images.githubusercontent.com/46327403/147833329-15660fbb-fd40-48ba-8621-41581f2d9e70.png)


### Current functionality:
- Display files and folders in a tree-like structure with their sizes and as percentage of parent size.
- Display them as a list (`-l`)
- Sort by size (`-s`)(descending, or ascending with `-r`)
- Only show files and folders which have size above given threshold (`-t`)
- Select depth of displayed files/folders (`-d`)(e.g. show only files/folders that are at most X folders deep)
- Multithreading, thanks to [rayon](https://crates.io/crates/rayon)

### TODO:
- More tests
- More features? (need ideas)
