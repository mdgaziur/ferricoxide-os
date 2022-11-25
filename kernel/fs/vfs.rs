use crate::fs::path::Path;
use crate::fs::{FSNode, Filesystem, IOResult};
use crate::kutils::errors::ErrorCode;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

pub static VFS: Mutex<Vfs> = Mutex::new(Vfs::new());

pub struct Vfs {
    mounts: BTreeMap<Path, Arc<Mutex<Box<dyn Filesystem>>>>,
}

impl Vfs {
    pub const fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
        }
    }

    pub fn mount(&mut self, path: Path, fs: Arc<Mutex<Box<dyn Filesystem>>>) {
        if let Some(fs) = self.mounts.insert(path, fs) {
            fs.lock().unmount();
        }
    }

    #[allow(clippy::type_complexity)]
    fn resolve_mountpoint(
        &mut self,
        path: Path,
    ) -> Result<(Arc<Mutex<Box<dyn Filesystem>>>, Path), ErrorCode> {
        if let Some(mountpoint) = self.mounts.get(&path) {
            Ok((mountpoint.clone(), Path::new("/")))
        } else if let Some(mountpoint) = self.mounts.get(&Path::new("/")) {
            Ok((mountpoint.clone(), path))
        } else {
            Err(ErrorCode::ENOENT)
        }
    }

    pub fn open(&mut self, path: Path) -> IOResult {
        let (mountpoint, path_in_mountpoint) = self.resolve_mountpoint(path)?;
        let node = mountpoint
            .lock()
            .open(path_in_mountpoint, mountpoint.clone())?;
        Ok(node)
    }

    pub fn write(
        &mut self,
        fsnode: &FSNode,
        bytes: Vec<u8>,
        start: usize,
        end: usize,
    ) -> Result<usize, ErrorCode> {
        let mountpoint = fsnode.fs.clone();
        let mut mountpoint_locked = mountpoint.lock();
        mountpoint_locked.write(fsnode, bytes, start, end)
    }

    pub fn read(
        &mut self,
        fsnode: &FSNode,
        start: usize,
        end: usize,
    ) -> Result<Vec<u8>, ErrorCode> {
        let mountpoint = fsnode.fs.clone();
        let mut mountpoint_locked = mountpoint.lock();
        mountpoint_locked.read(fsnode, start, end)
    }

    pub fn create_file(&mut self, path: Path) -> IOResult {
        let (mountpoint, path_in_mountpoint) = self.resolve_mountpoint(path)?;
        let node = mountpoint
            .lock()
            .create_file(path_in_mountpoint, mountpoint.clone())?;
        Ok(node)
    }

    pub fn create_dir(&mut self, path: Path) -> IOResult {
        let (mountpoint, path_in_mountpoint) = self.resolve_mountpoint(path)?;
        let node = mountpoint
            .lock()
            .create_dir(path_in_mountpoint, mountpoint.clone())?;
        Ok(node)
    }

    pub fn list_path(&mut self, path: Path) -> Result<Vec<FSNode>, ErrorCode> {
        let (mountpoint, path_in_mountpoint) = self.resolve_mountpoint(path)?;
        let nodes = mountpoint
            .lock()
            .list_path(path_in_mountpoint, mountpoint.clone())?;
        Ok(nodes)
    }

    pub fn fsize(&mut self, path: Path) -> Result<usize, ErrorCode> {
        let (mountpoint, path_in_mountpoint) = self.resolve_mountpoint(path)?;
        let mut mountpoint_locked = mountpoint.lock();
        mountpoint_locked.fsize(path_in_mountpoint)
    }
}
