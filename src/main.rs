#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::io::Read;

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
