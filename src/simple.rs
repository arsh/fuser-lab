use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirEntryExt, FileExt, MetadataExt};
use std::sync::RwLock;
use std::time::{Duration, UNIX_EPOCH};

use tracing::{error, trace};

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct SimpleFS {
    source_dir: String, // source directory
    inodes: RwLock<HashMap<u64, String>>,
}

impl SimpleFS {
    pub fn new(source_dir: String) -> Self {
        let mut inodes: HashMap<u64, String> = HashMap::new();
        inodes.insert(1, ".".into());
        SimpleFS {
            source_dir,
            inodes: RwLock::new(inodes),
        }
    }

    fn local_path(&self, path: &OsStr) -> String {
        format!("{}/{}", self.source_dir, path.to_string_lossy())
    }

    fn file_attributes(&self, md: &fs::Metadata) -> FileAttr {
        FileAttr {
            ino: md.ino(),
            size: md.size(),
            blocks: md.blocks(),
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: self.file_type(md),
            perm: md.mode() as u16,
            nlink: 1,
            uid: md.uid(),
            gid: md.gid(),
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    fn file_type(&self, md: &fs::Metadata) -> FileType {
        if md.is_dir() {
            FileType::Directory
        } else {
            FileType::RegularFile
        }
    }
}

impl Filesystem for SimpleFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        trace!(
            "lookup(parent={}, name={:?})",
            parent,
            name.to_string_lossy()
        );
        if parent != 1 {
            // we do not support directories
            error!("sub-directories are not supported");
            reply.error(ENOENT);
            return;
        }

        let md_result = fs::metadata(self.local_path(name));
        match md_result {
            Ok(md) => {
                let attr = self.file_attributes(&md);
                self.inodes
                    .write()
                    .unwrap()
                    .insert(attr.ino, name.to_string_lossy().into());
                reply.entry(&TTL, &attr, 0);
            }
            Err(err) => {
                error!("lookup error: {}", err);
                reply.error(ENOENT);
            }
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        trace!("getattr(ino={})", ino);

        match self.inodes.read().unwrap().get(&ino) {
            Some(name) => {
                let local_path = self.local_path(&OsStr::from_bytes(name.as_bytes()));
                let md = match fs::metadata(local_path) {
                    Ok(md) => md,
                    Err(err) => {
                        error!("getattr error: {}", err);
                        reply.error(ENOENT);
                        return;
                    }
                };
                trace!("metadata for {}: {:?}", name, md);
                let file_attributes = self.file_attributes(&md);
                trace!("file attributes for {}: {:?}", name, file_attributes);
                reply.attr(&TTL, &file_attributes);
            }
            None => reply.error(ENOENT),
        }
    }
    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        trace!("open(ino={})", ino);
        reply.opened(0, 0);
    }
    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        trace!(
            "read(ino={}, fh={}, offset={} size={})",
            ino,
            _fh,
            offset,
            size
        );
        match self.inodes.read().unwrap().get(&ino) {
            Some(name) => {
                let local_path = self.local_path(&OsStr::from_bytes(name.as_bytes()));
                trace!("reading local path: {}", local_path);
                let file = match File::open(local_path) {
                    Ok(f) => f,
                    Err(error) => {
                        error!("open error: {}", error);
                        reply.error(ENOENT);
                        return;
                    }
                };
                let mut buf = vec![0; size as usize];
                match file.read_at(&mut buf, offset as u64) {
                    Ok(n) => reply.data(&buf[..n]),
                    Err(error) => {
                        error!("read error: {}", error);
                        reply.error(ENOENT);
                        return;
                    }
                };
            }
            None => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        trace!("readdir(ino={}, offset={})", ino, offset);
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }
        let entries = match fs::read_dir(&self.source_dir) {
            Ok(res) => res,
            Err(error) => {
                error!("readdir error: {}", error);
                reply.error(ENOENT);
                return;
            }
        };

        for (i, entry) in entries.enumerate().skip(offset as usize) {
            trace!("processing entry: {:?}", entry);
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    error!("readdir error: {}", error);
                    reply.error(ENOENT);
                    return;
                }
            };

            if reply.add(
                entry.ino(),
                (i + 1) as i64,
                self.file_type(&entry.metadata().expect("could not read entry metadata")),
                &entry.file_name(),
            ) {
                break;
            }
        }

        reply.ok();
    }
}
