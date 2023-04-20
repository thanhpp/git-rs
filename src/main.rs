use std::env;
use std::fs;
use std::fs::create_dir;
use std::fs::read_dir;
use std::fs::DirEntry;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::os::unix::prelude::OsStrExt;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;

use flate2::bufread::ZlibEncoder;
use flate2::Compression;
use sha1::Digest;
use sha1::Sha1;

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
        "write-tree" => {
            let hash = write_tree(".");
            println!("{hash}");
        }
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

fn write_tree<P: AsRef<Path>>(path: P) -> String {
    // FILE MODE: They are a 16-bit number, with the high 4 bits specifying Git's type of the file and the low 9 bits map to Unix file permissions
    const DIRECTORY_FLAG: u16 = 1 << 14; // 0100 0000 0000 0000
    const FILE_FLAG: u16 = 1 << 15; // 1000 0000 0000 0000

    // list all files, sort by file name
    let mut files: Vec<DirEntry> = read_dir(path).unwrap().map(|f| f.unwrap()).collect();
    files.sort_by_key(|f| f.file_name());

    let mut tree_content: Vec<u8> = Vec::new();

    for f in files.iter() {
        // skip .git directory & target directory
        if f.file_name().eq(".git") || f.file_name().eq("target") {
            continue;
        }

        let f_type = f.file_type().unwrap();

        let (hash, mode) = if f_type.is_dir() {
            let hash = write_tree(f.path());
            (hash, DIRECTORY_FLAG)
        } else if f_type.is_file() {
            // read file
            let source_f = File::open(f.path()).unwrap();
            let mut reader = BufReader::new(source_f);
            let mut f_content_buffer = Vec::new();
            reader.read_to_end(&mut f_content_buffer).unwrap();

            // write git content of a file
            let mut store_buffer = Vec::new();

            // write header "blob" + " " + file_length_as_bytes + 0
            store_buffer.extend("blob ".to_string().as_bytes());
            store_buffer.extend(f_content_buffer.len().to_string().as_bytes());
            store_buffer.push(0);

            // write file content
            store_buffer.append(&mut f_content_buffer);

            // calculate sha1 hash
            let mut hasher = Sha1::new();
            hasher.update(&store_buffer);
            let hex_hash = hex::encode(hasher.finalize());

            // write to git file
            let dir: String = hex_hash.chars().take(2).collect();
            let git_file_name: String = hex_hash.chars().skip(2).collect();
            let output_path = Path::new(".git").join("objects").join(dir);
            if !output_path.exists() {
                create_dir(output_path.clone()).unwrap();
            }
            let output_path = output_path.join(git_file_name);
            let git_file = File::create(output_path).unwrap();
            let mut zlib_reader =
                ZlibEncoder::new(BufReader::new(&store_buffer[..]), Compression::fast());

            std::io::copy(&mut zlib_reader, &mut BufWriter::new(git_file)).unwrap();

            // file mode
            #[cfg(unix)]
            let file_mode = f.metadata().unwrap().permissions().mode();
            #[cfg(not(unix))]
            let file_mode = 0o644;

            (hex_hash, FILE_FLAG | ((file_mode & 0o777) as u16))
        } else {
            continue;
        };

        // println!("{:06o} | {:?}", mode, f.file_name());
        tree_content.extend_from_slice(format!("{:06o} ", mode).as_bytes());
        tree_content.extend_from_slice(f.file_name().as_bytes());
        tree_content.push(0);
        tree_content.append(&mut hex::decode(hash).unwrap());
    }

    // hash tree
    let mut tree_buffer = Vec::new();

    // write header "tree" + " " + file_length_as_bytes + 0
    tree_buffer.extend("tree ".to_string().as_bytes());
    tree_buffer.extend(tree_content.len().to_string().as_bytes());
    tree_buffer.push(0);

    // write file content
    tree_buffer.append(&mut tree_content);

    // calculate sha1 hash
    let mut hasher = Sha1::new();
    hasher.update(&tree_buffer);
    let hex_hash = hex::encode(hasher.finalize());

    // write to git file
    let dir: String = hex_hash.chars().take(2).collect();
    let git_file_name: String = hex_hash.chars().skip(2).collect();
    let output_path = Path::new(".git").join("objects").join(dir);
    if !output_path.exists() {
        create_dir(output_path.clone()).unwrap();
    }
    let output_path = output_path.join(git_file_name);
    let git_file = File::create(output_path).unwrap();
    let mut zlib_reader = ZlibEncoder::new(BufReader::new(&tree_buffer[..]), Compression::fast());

    std::io::copy(&mut zlib_reader, &mut BufWriter::new(git_file)).unwrap();

    return hex_hash;
}
