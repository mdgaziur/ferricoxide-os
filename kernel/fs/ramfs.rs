use crate::fs::path::Path;
use crate::fs::{FSNode, FSNodeType, Filesystem, IOResult};
use crate::kutils::errors::ErrorCode;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

pub struct RamFS {
    root: RamFSNode,
}

impl RamFS {
    pub fn new() -> Self {
        Self {
            root: RamFSNode::Dir(RamFSDir {
                children: BTreeMap::new(),
                name: String::from("/"),
            }),
        }
    }

    fn resolve(path: Path, node: &mut RamFSNode) -> Result<&mut RamFSNode, ErrorCode> {
        if path.segments().is_empty() {
            Ok(node)
        } else if path.segments().len() == 1 {
            match node {
                RamFSNode::File(_) => Err(ErrorCode::ENOENT),
                RamFSNode::Dir(dir) => {
                    let Some(entry) = dir.children.get_mut(path.segments().last().unwrap()) else {
                        return Err(ErrorCode::ENOENT);
                    };

                    Ok(entry)
                }
            }
        } else {
            match node {
                RamFSNode::File(_) => Err(ErrorCode::ENOENT),
                RamFSNode::Dir(d) => {
                    if let Some(entry) = d.children.get_mut(path.segments().first().unwrap()) {
                        Self::resolve(path.path_from_idx(1), entry)
                    } else {
                        Err(ErrorCode::ENOENT)
                    }
                }
            }
        }
    }

    fn resolve_to_fsnode(
        path: Path,
        full_path: Path,
        node: &mut RamFSNode,
        arc_ref: Arc<Mutex<Box<dyn Filesystem>>>,
    ) -> IOResult {
        let entry = Self::resolve(path, node)?;

        Ok(FSNode {
            name: entry.name(),
            path: full_path,
            typ: entry.to_fs_node_type(),
            fs: arc_ref,
        })
    }
}

impl Filesystem for RamFS {
    fn root(&self, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> FSNode {
        FSNode {
            name: String::from("/"),
            path: Path::new("/"),
            typ: FSNodeType::Dir,
            fs: arc_ref.clone(),
        }
    }

    fn open(&mut self, path: Path, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> IOResult {
        Self::resolve_to_fsnode(path.clone(), path, &mut self.root, arc_ref)
    }

    fn create_file(&mut self, path: Path, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> IOResult {
        let dir_path = {
            if path.segments().len() <= 1 {
                Path::new("/")
            } else {
                path.path_from_range(0, path.segments().len() - 2)
            }
        };
        let result = Self::resolve(dir_path, &mut self.root)?;
        let dir = match result {
            RamFSNode::Dir(dir) => dir,
            RamFSNode::File(_) => return Err(ErrorCode::ENOENT),
        };
        dir.children.insert(
            path.segments().last().unwrap().clone(),
            RamFSNode::File(RamFSFile {
                name: path.segments().last().unwrap().clone(),
                content: Vec::new(),
            }),
        );

        Ok(FSNode {
            name: path.segments().last().unwrap().clone(),
            typ: FSNodeType::File,
            fs: arc_ref.clone(),
            path,
        })
    }

    fn create_dir(&mut self, path: Path, arc_ref: Arc<Mutex<Box<dyn Filesystem>>>) -> IOResult {
        let dir_path = {
            if path.segments().len() <= 1 {
                Path::new("/")
            } else {
                path.path_from_range(0, path.segments().len() - 2)
            }
        };
        let result = Self::resolve(dir_path, &mut self.root)?;
        let dir = match result {
            RamFSNode::Dir(dir) => dir,
            RamFSNode::File(_) => return Err(ErrorCode::ENOENT),
        };
        dir.children.insert(
            path.segments().last().unwrap().clone(),
            RamFSNode::Dir(RamFSDir {
                name: path.segments().last().unwrap().clone(),
                children: BTreeMap::new(),
            }),
        );

        Ok(FSNode {
            name: path.segments().last().unwrap().clone(),
            typ: FSNodeType::Dir,
            fs: arc_ref.clone(),
            path,
        })
    }

    fn list_path(
        &mut self,
        path: Path,
        arc_ref: Arc<Mutex<Box<dyn Filesystem>>>,
    ) -> Result<Vec<FSNode>, ErrorCode> {
        let result = Self::resolve(path.clone(), &mut self.root)?;
        let dir = match result {
            RamFSNode::Dir(dir) => dir,
            RamFSNode::File(_) => return Err(ErrorCode::ENOENT),
        };

        let mut fsnodes = vec![];
        for (child_name, child_node) in &dir.children {
            fsnodes.push(FSNode {
                name: child_name.clone(),
                path: path.append(child_name),
                fs: arc_ref.clone(),
                typ: child_node.to_fs_node_type(),
            })
        }

        Ok(fsnodes)
    }

    fn write(
        &mut self,
        node: &FSNode,
        bytes: Vec<u8>,
        start: usize,
        end: usize,
    ) -> Result<usize, ErrorCode> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let file = Self::resolve(node.path.clone(), &mut self.root)?;
        let f = match file {
            RamFSNode::Dir(_) => return Err(ErrorCode::EISDIR),
            RamFSNode::File(f) => f,
        };
        let bytes_increased = (end + 1).saturating_sub(f.content.len());
        for _ in 0..bytes_increased {
            f.content.push(0);
        }

        f.content[start..(end + 1)].copy_from_slice(&bytes[..(end + 1 - start)]);

        Ok(bytes_increased)
    }

    fn read(&mut self, node: &FSNode, start: usize, end: usize) -> Result<Vec<u8>, ErrorCode> {
        let file = Self::resolve(node.path.clone(), &mut self.root)?;
        let f = match file {
            RamFSNode::Dir(_) => return Err(ErrorCode::EISDIR),
            RamFSNode::File(f) => f,
        };

        let mut read_bytes = vec![];
        for i in start..end + 1 {
            read_bytes.push(f.content[i]);
        }
        Ok(read_bytes)
    }

    fn fsize(&mut self, path: Path) -> Result<usize, ErrorCode> {
        let file = Self::resolve(path, &mut self.root)?;
        match file {
            RamFSNode::Dir(_) => Err(ErrorCode::EISDIR),
            RamFSNode::File(f) => Ok(f.content.len()),
        }
    }

    fn close(&mut self, _: FSNode) {
        // do nothing
    }

    fn unmount(&mut self) {
        // do nothing
    }
}

#[derive(Debug)]
struct RamFSDir {
    name: String,
    children: BTreeMap<String, RamFSNode>,
}

#[derive(Debug)]
struct RamFSFile {
    name: String,
    content: Vec<u8>,
}

#[derive(Debug)]
enum RamFSNode {
    Dir(RamFSDir),
    File(RamFSFile),
}

impl RamFSNode {
    pub fn name(&self) -> String {
        match self {
            RamFSNode::Dir(d) => d.name.clone(),
            RamFSNode::File(f) => f.name.clone(),
        }
    }

    pub fn to_fs_node_type(&self) -> FSNodeType {
        match self {
            RamFSNode::Dir(_) => FSNodeType::Dir,
            RamFSNode::File(_) => FSNodeType::File,
        }
    }
}
