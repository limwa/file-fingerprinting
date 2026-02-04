use std::io::{Write, BufReader, Read};
use std::process::exit;
use std::{io, env, thread};
use std::fs::{self, OpenOptions, File};
use std::path::{Path, PathBuf};
use threadpool::ThreadPool;
use std::sync::mpsc::{channel, Sender};
use crc::{Crc};

const BUFFER_SIZE: usize = 64 * 4096;
const CRC: Crc<u64> = Crc::<u64>::new(&crc::CRC_64_ECMA_182);

fn main() {
    let mut checksums = open_checksums_file().unwrap();
    
    let pool = ThreadPool::new(6);
    let (tx, rx) = channel();
    
    match queue_cwd(tx, pool) {
        Ok(_) => {}
        Err(e) => eprintln!("error: could not traverse directory\n{}\n", e)
    }

    let cwd = env::current_dir().unwrap();
    rx.iter().for_each(move |result| {
        if let Some((digest, path)) = result {

            let file: &str;
            match path.strip_prefix(&cwd) {
                Ok(path_str) => { 
                    file = path_str.to_str().unwrap();
                }
                Err(_) => {
                    file = path.to_str().unwrap();
                }
            }

            match checksums.write_fmt(format_args!("{:<20}    {}\n", digest, file)) {
                Ok(_) => {}
                Err(e) => eprintln!("error: could not write to checksums\n{}\n", e)
            }
        } else {
            exit(0);
        }
    });
}

fn open_checksums_file() -> io::Result<File> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Usage: fingerprints <output>");
    }

    let path = Path::new(args.get(1).unwrap());

    let mut options = OpenOptions::new();
    options.create(true);
    options.append(true);

    let checksums = options.open(path)?;
    Ok(checksums)
}

fn queue_cwd(tx: Sender<Option<(u64, PathBuf)>>, pool: ThreadPool) -> io::Result<()> {
    let cwd = env::current_dir()?;

    thread::spawn(move || {
        let this_tx = tx.clone();
        visit_cwd(&cwd, tx, &pool);
        
        pool.join();
        pool.execute(move || {
            match this_tx.send(None) {
                Ok(_) => {}
                Err(e) => eprintln!("error: could not send EOF back to main thread\n{}\n", e)
            }
        })
    });
    
    Ok(())
}

fn visit_cwd(cwd: &Path, tx: Sender<Option<(u64, PathBuf)>>, pool: &ThreadPool) {
    let result = visit_dirs(cwd, cwd, tx, &move |tx, entry: &Path| {
        
        let tx = tx.clone();
        let path = entry.to_path_buf();
        pool.execute(move|| {
            match visit_file(&path) {
                Ok(digest) => {
                    match tx.send(Some((digest, path))) {
                        Ok(_) => {}
                        Err(e) => eprintln!("error: could not send data back to main thread\n{}\n", e)
                    }
                }
                Err(e) => eprintln!("error: could not process {}\n{}\n", path.to_str().unwrap(), e)
            }
        });
    });

    match result {
        Ok(_) => {}
        Err(e) => eprintln!("error: could not traverse directory\n{}\n", e)
    }
}

fn visit_dirs(cwd: &Path, dir: &Path, tx: Sender<Option<(u64, PathBuf)>>, cb: &dyn Fn(Sender<Option<(u64, PathBuf)>>, &Path)) -> io::Result<()> {
    if dir.is_dir() {
        let (directories, files): (Vec<_>, Vec<_>) = fs::read_dir(dir)?.partition(|entry| {
            let entry = entry.as_ref().unwrap();
            let path = entry.path();
            path.is_dir()
        });

        for entry in files {
            let path = entry?.path();
            if let Ok(path) = &path.strip_prefix(cwd) {
                cb(tx.clone(), &path);
            } else {
                cb(tx.clone(), &path);
            }
        }

        for entry in directories {
            let entry = entry?;
            let path = entry.path();
            if path.file_name().unwrap() == ".ignore" {
                continue;
            }
            
            visit_dirs(&cwd, &path, tx.clone(), cb)?;
        }
    }
    Ok(())
}

fn visit_file(path: &Path) -> io::Result<u64> {
    println!("processing: {}", path.to_str().unwrap());
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut buffer = [0; BUFFER_SIZE];
    
    let mut digest = CRC.digest();

    loop {
        let length = reader.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        
        digest.update(&buffer[0..length]);
    }

    Ok(digest.finalize())
}