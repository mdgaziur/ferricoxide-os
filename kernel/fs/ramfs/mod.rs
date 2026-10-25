mod ops;

use crate::d;
use crate::dbg::{D_INFO, dmesgln};
use crate::ds::{StaticBitmap, static_bitmap_size};
use crate::fs::ramfs::ops::dir::{
    ramfs_create, ramfs_link, ramfs_lookup, ramfs_mkdir, ramfs_mkdir_inner, ramfs_readdir,
    ramfs_rmdir,
};
use crate::fs::ramfs::ops::file::{ramfs_read, ramfs_truncate, ramfs_write};
use crate::fs::ramfs::ops::ramfs_sync_stat;
use crate::fs::ramfs::ops::symlink::{ramfs_readlink, ramfs_symlink};
use crate::fs::{
    DEntryKind, Dentry, FSBooleanResult, FSOperations, FSResult, FileHandle, FileSystem, Inode,
    InodeStat, SuperBlock, SuperBlockOps,
};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashMap;
use ops::dir::ramfs_unlink_from_parent_inode;
use spin::RwLock;

pub struct RamFS {}

pub const RAMFS_NAME: &str = "ferricoxide-ramfs";
pub const RAMFS_MAX_INODES: usize = 1000000;
type RamFSInodeDowncastRef<'a> = &'a Arc<RwLock<RamFSInode>>;

const RAMFS_DIR_OPS: FSOperations = FSOperations {
    lookup: Some(ramfs_lookup),
    create: Some(ramfs_create),
    mkdir: Some(ramfs_mkdir),
    rmdir: Some(ramfs_rmdir),
    link: Some(ramfs_link),
    symlink: Some(ramfs_symlink),
    readdir: Some(ramfs_readdir),

    close: None,
    read: None,
    write: None,
    truncate: None,

    unlink: Some(ramfs_unlink_from_parent_inode),
    mv: None,
    readlink: None,
    sync_stat: Some(ramfs_sync_stat),
    evict_node: Some(ramfs_evict_node),
};

const RAMFS_FILE_OPS: FSOperations = FSOperations {
    lookup: None,
    create: None,
    mkdir: None,
    rmdir: None,
    link: None,
    symlink: None,
    readdir: None,

    close: None,
    read: Some(ramfs_read),
    write: Some(ramfs_write),
    truncate: Some(ramfs_truncate),

    unlink: None,
    mv: None,
    readlink: None,
    sync_stat: Some(ramfs_sync_stat),
    evict_node: Some(ramfs_evict_node),
};

const RAMFS_SYM_OPS: FSOperations = FSOperations {
    lookup: None,
    create: None,
    mkdir: None,
    rmdir: None,
    link: None,
    symlink: None,
    readdir: None,

    close: None,
    read: None,
    write: None,
    truncate: None,

    unlink: None,
    mv: None,
    readlink: Some(ramfs_readlink),
    sync_stat: Some(ramfs_sync_stat),
    evict_node: Some(ramfs_evict_node),
};

impl RamFS {
    pub fn new() -> Self {
        RamFS {}
    }
}

impl FileSystem for RamFS {
    fn name(&self) -> &'static str {
        RAMFS_NAME
    }

    fn requires_dev(&self) -> bool {
        false
    }

    fn mount(&self, mount_id: u64, _: Option<&FileHandle>) -> FSResult<Arc<RwLock<SuperBlock>>> {
        let mut registry = RamFSInodeRegistry::new();

        let root_inode = ramfs_mkdir_inner(&mut registry, "/", None)?;
        let root_dirent = Arc::new(Dentry {
            name: RwLock::new("/".to_string()),
            inode: root_inode.clone(),
            parent: RwLock::new(Weak::new()),
            kind: DEntryKind::Directory,
            subdirs: Some(RwLock::new(HashMap::new())),
            mounted_here: Arc::new(RwLock::new(vec![])),
        });
        *root_dirent.parent.write() = Arc::downgrade(&root_dirent);

        let superblock = Arc::new(RwLock::new(SuperBlock {
            mount_id,
            magic: None,
            fs_specific_data: RwLock::new(Box::new(registry)),
            device: None,
            root: root_dirent.clone(),
            ops: SuperBlockOps {
                sync: None,
                statfs: Some(|_| todo!()),
            },
        }));

        root_inode.write().super_block = Arc::downgrade(&superblock);

        Ok(superblock)
    }
}

type RamFSInodeStat = InodeStat;

struct RamFSInodeRegistry {
    inodes: HashMap<u64, Arc<RwLock<RamFSInode>>>,
    ino_map: StaticBitmap<{ static_bitmap_size(RAMFS_MAX_INODES) }>,
}

impl RamFSInodeRegistry {
    pub fn new() -> Self {
        RamFSInodeRegistry {
            inodes: HashMap::new(),
            ino_map: StaticBitmap::new(),
        }
    }

    pub fn allocate_ino(
        &mut self,
        inode_kind: RamFSInodeKind,
        stat: RamFSInodeStat,
    ) -> Option<Arc<RwLock<RamFSInode>>> {
        let mut ino = None;
        for (c_ino, used) in self.ino_map.iter().enumerate() {
            if !used {
                ino = Some(c_ino as u64);
                break;
            }
        }

        if let Some(ino) = ino {
            let inode = Arc::new(RwLock::new(RamFSInode {
                ino,
                inode_kind,
                stat,
                // we set i_nlink to 1 assuming that it is owned by a parent directory.
                i_nlink: 1,
            }));
            self.inodes.insert(ino, inode.clone());
            self.ino_map.set(ino as usize);
            Some(inode)
        } else {
            None
        }
    }

    pub fn get_inode(&self, ino: InodeNumber) -> Option<Arc<RwLock<RamFSInode>>> {
        self.inodes.get(&ino).cloned()
    }

    /// Evicts the inode with the given inode number if it exists and has a link count of 0.
    ///
    /// # Panics
    /// If the inode does not exist in the registry this function panics. Such a panic indicates
    /// a bug in the VFS layer, since it should never attempt to evict an inode that does not exist
    /// in the registry.
    pub fn evict_node(&mut self, ino: InodeNumber) {
        if let Some(inode_arc_ref) = self.inodes.get(&ino) {
            let inode = inode_arc_ref.read();

            if inode.i_nlink == 0 {
                dmesgln(d!(D_INFO "Evicting inode {} with link count 0", ino));

                drop(inode);
                self.deallocate_ino(ino);
            }
        } else {
            panic!(
                "VFS BUG!!! Attempted to evict an inode that does not exist in RamFSInodeRegistry::evict_node"
            );
        }
    }

    pub fn deallocate_ino(&mut self, ino: InodeNumber) {
        if self.inodes.remove(&ino).is_some() {
            self.ino_map.clear(ino as usize);
        }
    }
}

fn ramfs_evict_node(inode: &Inode) -> FSBooleanResult {
    let super_block_read_arc_optional = inode.super_block.upgrade();
    let super_block_read_arc = super_block_read_arc_optional
        .ok_or_else(|| {
            panic!("VFS BUG!!! Attempted to evict an inode whose super block has been dropped in ramfs_evict_node")
        })?;
    let super_block_read = super_block_read_arc.read();
    let mut inode_registry_write_opaque = super_block_read.fs_specific_data.write();
    let inode_registry_write = inode_registry_write_opaque
        .downcast_mut::<RamFSInodeRegistry>()
        .expect("Failed to downcast SuperBlock fs_specific_data to RamFSInodeRegistry in ramfs_evict_node");

    inode_registry_write.evict_node(inode.ino);
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum RamFSUnlinkKind {
    Rm,
    Close,
}

pub type InodeNumber = u64;

struct RamFSInode {
    ino: InodeNumber,
    inode_kind: RamFSInodeKind,
    stat: RamFSInodeStat,
    i_nlink: u64,
}

pub enum RamFSInodeKind {
    Directory {
        children: HashMap<String, InodeNumber>,
    },
    File {
        data: Vec<u8>,
    },
    Symlink {
        target: String,
    },
}
