use crate::fs::path::Path;
use crate::kutils::errors::ErrorCode;
use alloc::sync::Arc;
use core::fmt::{Display, Formatter};
use spin::{Mutex, RwLock};

pub mod path;
pub mod ramfs;
pub mod vfs;

pub struct FSNode {
    name: String,
    typ: FSNodeType,
    fs: Arc<Mutex<Box<dyn Filesystem>>>,
    path: Path,
}

impl FSNode {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn typ(&self) -> FSNodeType {
        self.typ
    }

    pub fn path(&self) -> Path {
        self.path.clone()
    }
}

impl Display for FSNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} - {}", self.name, self.path)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FSNodeType {
    Dir,
    File,
    Symlink,
}

pub trait Filesystem: Send + Sync {
    fn root(&self, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> FSNode;
    fn open(&mut self, path: Path, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> IOResult;
    fn create_file(&mut self, path: Path, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> IOResult;
    fn create_dir(&mut self, path: Path, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> IOResult;
    fn list_path(
        &mut self,
        path: Path,
        arc_ref: Arc<Mutex<Box<dyn Filesystem>>>,
    ) -> Result<Vec<FSNode>, ErrorCode>;
    fn write(
        &mut self,
        node: &FSNode,
        bytes: Vec<u8>,
        start: usize,
        end: usize,
    ) -> Result<usize, ErrorCode>;
    fn read(&mut self, node: &FSNode, start: usize, end: usize) -> Result<Vec<u8>, ErrorCode>;
    fn fsize(&mut self, path: Path) -> Result<usize, ErrorCode>;
    fn close(&mut self, fs_node: FSNode);
    fn unmount(&mut self);
}

type IOResult = Result<FSNode, ErrorCode>;
