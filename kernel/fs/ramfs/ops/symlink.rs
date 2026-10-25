use crate::arch::get_rtc_time;
use crate::d;
use crate::dbg::{D_ERROR, dmesgln};
use crate::fs::ramfs::{
    RAMFS_SYM_OPS, RamFSInodeDowncastRef, RamFSInodeKind, RamFSInodeRegistry, RamFSInodeStat,
};
use crate::fs::{FSResult, Inode, InodeKind};
use crate::posix::PosixError;
use crate::time::SystemTime;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use spin::RwLock;

pub fn ramfs_readlink(inode: &Inode) -> FSResult<String> {
    let ramfs_inode_ref: RamFSInodeDowncastRef = inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_readlink");
    let ramfs_inode = ramfs_inode_ref.read();

    if let RamFSInodeKind::Symlink { target } = &ramfs_inode.inode_kind {
        Ok(target.clone())
    } else {
        dmesgln(
            d!(D_ERROR "VFS BUG!!! Attempted to readlink from a non-symlink inode in ramfs_readlink"),
        );
        Err(PosixError::EINVAL)
    }
}

pub fn ramfs_symlink(parent: &Inode, target: &str, name: &str) -> FSResult<Arc<RwLock<Inode>>> {
    let parent_inode_ref: RamFSInodeDowncastRef = parent
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_symlink");
    let mut parent_inode = parent_inode_ref.write();

    if let RamFSInodeKind::Directory { children } = &mut parent_inode.inode_kind {
        if children.contains_key(name) {
            return Err(PosixError::EEXIST);
        }

        let superblock = parent.super_block.upgrade().ok_or_else(|| {
            panic!("VFS BUG!!! Attempted to create a symlink in a parent inode whose super block has been dropped in ramfs_symlink")
        })?;
        let superblock_read = superblock.read();

        let mut fs_specific_data_read = superblock_read.fs_specific_data.write();
        let inode_registry = fs_specific_data_read
            .downcast_mut::<RamFSInodeRegistry>()
            .unwrap();

        let now = get_rtc_time() as SystemTime;
        let symlink_ramfs_inode = inode_registry
            .allocate_ino(
                RamFSInodeKind::Symlink {
                    target: target.to_string(),
                },
                RamFSInodeStat {
                    permissions: 0o777,
                    atime: now,
                    ctime: now,
                    mtime: now,
                    ..RamFSInodeStat::default()
                },
            )
            .ok_or(PosixError::ENOSPC)?;

        let symlink_ramfs_inode_read = symlink_ramfs_inode.read();
        let ino = symlink_ramfs_inode_read.ino;
        let stat = symlink_ramfs_inode_read.stat;
        children.insert(name.to_string(), symlink_ramfs_inode_read.ino);
        drop(symlink_ramfs_inode_read);

        let new_inode = Inode {
            ino,
            typ: InodeKind::Symlink,
            stat,
            super_block: parent.super_block.clone(),
            inner_opaque: Box::new(symlink_ramfs_inode),
            name: name.to_string(),
            ops: RAMFS_SYM_OPS,
        };

        Ok(Arc::new(RwLock::new(new_inode)))
    } else {
        dmesgln(
            d!(D_ERROR "VFS BUG!!! Attempted to create a symlink in a non-directory inode in ramfs_symlink"),
        );
        Err(PosixError::ENOTDIR)
    }
}
