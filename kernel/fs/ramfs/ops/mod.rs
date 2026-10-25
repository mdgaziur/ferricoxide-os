use crate::fs::ramfs::RamFSInodeDowncastRef;
use crate::fs::{FSBooleanResult, Inode};
pub mod dir;
pub mod file;
pub mod symlink;

pub fn ramfs_sync_stat(inode: &Inode) -> FSBooleanResult {
    let ramfs_inode_ref: RamFSInodeDowncastRef = inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_dir_stat");
    ramfs_inode_ref.write().stat = inode.stat;

    Ok(())
}
