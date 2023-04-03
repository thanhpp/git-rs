#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::fs::create_dir;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;

use sha1::Digest;

fn main() {
    // Uncomment this block to pass the first stage
    let args: Vec<String> = env::args().collect();
    match args[1].as_str() {
        "init" => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
            println!("Initialized git directory");
        }
        "cat-file" => cat_file(&args),
        "hash-object" => hash_object(&args),
        "ls-tree" => ls_tree(&args),
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}

fn cat_file(args: &Vec<String>) {
    if args[2].ne("-p") {
        return;
    }

    let hash = args[3].clone();
    if hash.len() < 2 {
        return;
    }

    let hash_prefix = hash[..2].as_bytes();
    let hash_prefix = std::str::from_utf8(hash_prefix).unwrap();

    for entry in fs::read_dir(".git/objects").unwrap() {
        let path = entry.unwrap().path();

        let basename = path.file_name().unwrap();

        if !basename.eq(hash_prefix) {
            continue;
        }

        // read file
        for f_entry in fs::read_dir(path).unwrap() {
            let file_content = fs::read(f_entry.unwrap().path()).unwrap();
            let mut z = flate2::read::ZlibDecoder::new(&file_content[..]);
            let mut s = String::new();
            z.read_to_string(&mut s).unwrap();
            print!("{}", s[8..].to_string()); // remove blob 40\x00
            return;
        }
    }
}

fn hash_object(args: &Vec<String>) {
    match args.get(2) {
        None => return,
        Some(x) => {
            if x.ne("-w") {
                return;
            }
        }
    }

    let file_path = match args.get(3) {
        None => return,
        Some(p) => p,
    };

    // read_file
    let file_content = fs::read(file_path).unwrap();
    let mut content = Vec::new(); // the hash "blob <0><file_content>"
    content.extend("blob ".as_bytes());
    content.extend(file_content.len().to_string().as_bytes());
    content.push(0);
    content.extend(file_content);
    let mut s = sha1::Sha1::new();
    s.update(&content);
    let hash = s.finalize();
    let hash = hash
        .iter()
        .map(|b| format!("{:02x?}", b)) // convert to hex
        .collect::<Vec<_>>()
        .join("");
    print!("{}", hash);

    // write_file
    let sub_dir_name: String = hash.chars().take(2).collect(); // first 2 characters hash
    let file_name: String = hash.chars().skip(2).collect(); // the remaining of the hash
    let mut blob_path = std::path::Path::new(".git")
        .join("objects")
        .join(sub_dir_name);
    if !blob_path.exists() {
        create_dir(&blob_path).unwrap(); // create directory
    }
    blob_path = blob_path.join(file_name);
    let encoded_file = File::create(blob_path).unwrap();
    let mut zlib_reader = flate2::bufread::ZlibEncoder::new(
        BufReader::new(&content[..]),
        flate2::Compression::fast(),
    );

    // write file from a buffer to another buffer
    std::io::copy(&mut zlib_reader, &mut BufWriter::new(encoded_file)).unwrap();
}

fn ls_tree(args: &Vec<String>) {
    match args.get(1) {
        None => return,
        Some(x) => {
            if x.ne("ls-tree") {
                return;
            }
        }
    }

    match args.get(2) {
        None => return,
        Some(x) => {
            if x.ne("--name-only") {
                return;
            }
        }
    }

    let tree_sha;
    match args.get(3) {
        None => return,
        Some(x) => {
            tree_sha = x;
        }
    }

    let tree_sha_prefix: String = tree_sha.chars().take(2).collect();
    let tree_sha_postfix: String = tree_sha.chars().skip(2).collect();

    for entry in fs::read_dir(".git/objects").unwrap() {
        let path = entry.unwrap().path();

        if path.file_name().unwrap().ne(tree_sha_prefix.as_str()) {
            continue;
        }

        for sub_entry in fs::read_dir(path).unwrap() {
            let sub_entry = (sub_entry).unwrap();
            let file_name = (&sub_entry).file_name();

            if !file_name
                .into_string()
                .unwrap()
                .starts_with(tree_sha_postfix.as_str())
            {
                continue;
            }

            // println!("reading {:?}", (&sub_entry).path());

            let file_content = fs::read((&sub_entry).path()).unwrap();
            let mut z = flate2::read::ZlibDecoder::new(&file_content[..]);
            let mut s = Vec::new();
            z.read_to_end(&mut s).unwrap();

            // <\0><mode>< ><name><\0><hash(len = 20)>

            let data = s.as_slice();
            let mut entries = Vec::new();
            let mut pos = s.iter().position(|&x| x == b'\0').unwrap() + 1; // after \0

            while pos < data.len() {
                let mode_end = data[pos..].iter().position(|&x| x == b' ').unwrap();
                // let mode_str = String::from_utf8(data[pos..pos + mode_end].to_vec()).unwrap();
                // let mode = u32::from_str_radix(&mode_str, 8).unwrap();

                let name_end = data[pos + mode_end + 1..]
                    .iter()
                    .position(|&x| x == b'\0')
                    .unwrap();
                let name = String::from_utf8(
                    data[pos + mode_end + 1..pos + mode_end + 1 + name_end].to_vec(),
                );

                let hash_start = pos + mode_end + 1 + name_end + 1;
                let hash_end = hash_start + 20;
                // let hash = hex::encode(&data[hash_start..hash_end]);

                entries.push(name.unwrap());

                pos = hash_end;
            }

            for e in entries.iter() {
                println!("{}", e);
            }
        }
    }
}
