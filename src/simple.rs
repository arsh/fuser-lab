use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::os::unix::fs::MetadataExt;
use std::sync::RwLock;
use std::time::{Duration, UNIX_EPOCH};

use tracing::{error, trace};

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

pub struct SimpleFS {
    source_dir: String, // source directory
    inodes: RwLock<HashMap<u64, String>>,
}

impl SimpleFS {
    pub fn new(source_dir: String) -> Self {
        SimpleFS {
            source_dir,
            inodes: RwLock::new(HashMap::new()),
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
            kind: FileType::RegularFile,
            perm: md.mode() as u16,
            nlink: 1,
            uid: md.uid(),
            gid: md.gid(),
            rdev: 0,
            flags: 0,
            blksize: 512,
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
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
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
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        trace!(
            "read(ino={}, fh={}, offset={} size={})",
            ino,
            _fh,
            offset,
            _size
        );
        match self.inodes.read().unwrap().get(&ino) {
            Some(_name) => {
                reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
                // let local_path = self.local_path(&OsStr::from(name));
                // let mut file = File::open(local_path).unwrap();
                // file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
                // let mut buf = [0; 1024];
                // let n = file.read(&mut buf).unwrap();
                // reply.data(&buf[..n]);
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

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}
