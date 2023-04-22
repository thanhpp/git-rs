use std::{
    fs::{create_dir, File},
    io::{BufReader, BufWriter},
    path::Path,
};

use anyhow::{Ok, Result};
use flate2::Compression;
use sha1::{Digest, Sha1};

pub fn write_obj(hash: String, obj_content: &Vec<u8>) -> Result<()> {
    // encode object
    let mut zlib_reader =
        flate2::bufread::ZlibEncoder::new(BufReader::new(&obj_content[..]), Compression::fast());

    // create file
    let dir: String = hash.chars().take(2).collect();
    let obj_file_name: String = hash.chars().skip(2).collect();

    let mut obj_path = Path::new(".git").join("objects").join(dir);
    if !obj_path.exists() {
        create_dir(obj_path.clone())?;
    }
    obj_path = obj_path.join(obj_file_name);

    let f = File::create(obj_path)?;

    // write file
    std::io::copy(&mut zlib_reader, &mut BufWriter::new(f))?;

    Ok(())
}

pub fn gen_objects(obj_type: String, content: &Vec<u8>) -> Result<(String, Vec<u8>)> {
    let mut buffer = Vec::new();

    // header
    buffer.extend(obj_type.as_bytes());
    buffer.extend(" ".as_bytes());
    buffer.extend(content.len().to_string().as_bytes());
    buffer.push(0);

    // body
    buffer.extend_from_slice(&content);

    // generate hash
    let hash = sha1_hash(&buffer);

    return Ok((hash, buffer));
}

pub fn sha1_hash(buffer: &Vec<u8>) -> String {
    let mut hasher = Sha1::new();
    hasher.update(&buffer);
    hex::encode(hasher.finalize())
}
