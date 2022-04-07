use anyhow::Result;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::path::Path;

#[allow(dead_code)]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    let mut buf = Vec::new();
    let fullpath = &Path::new("assets").join(&path);
    let mut file = File::open(&fullpath).unwrap();
    file.read_to_end(&mut buf).unwrap();
    Cursor::new(buf)
}

pub fn load_file<P: AsRef<Path>>(path: P) -> Result<String> {
    let fullpath = &Path::new("assets").join(&path);
    let mut file = File::open(&fullpath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
