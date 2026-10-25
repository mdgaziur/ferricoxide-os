use crate::d;
use crate::dbg::{D_ERROR, D_INFO, dmesgln};
use crate::fs::ramfs::{
    RAMFS_DIR_OPS, RAMFS_FILE_OPS, RamFSInode, RamFSInodeDowncastRef, RamFSInodeKind,
    RamFSInodeRegistry, RamFSInodeStat,
};
use crate::fs::{
    DEntryKind, FSBooleanResult, FSResult, Inode, InodeKind, InodeStat, Mode, ReaddirDirent,
};
use crate::posix::{DEFAULT_DIR_PERMISSIONS, DEFAULT_FILE_PERMISSIONS, PosixError};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use hashbrown::HashMap;
use spin::RwLock;

pub fn ramfs_mkdir(
    parent_inode: &mut Inode,
    target_dir: &str,
    _mode: Mode,
) -> FSResult<Arc<RwLock<Inode>>> {
    let ramfs_parent_inode_ref: RamFSInodeDowncastRef = parent_inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_mkdir");

    // We wrap the read lock in a block to ensure it is dropped before we acquire the write lock
    // later in this function, since we will need to get a write lock on the parent node to update
    // its children with the new directory entry, and holding a read lock while trying to acquire
    // a write lock would cause a deadlock.
    {
        let ramfs_parent_inode_read = ramfs_parent_inode_ref.read();

        if ramfs_parent_inode_read.i_nlink == 0 {
            // The entry has been marked as deleted, so we do not allow creating new directories in this dir,
            // as per POSIX.
            return Err(PosixError::ENOENT);
        }

        if !matches!(
            ramfs_parent_inode_read.inode_kind,
            RamFSInodeKind::Directory { .. }
        ) {
            return Err(PosixError::ENOTDIR);
        }
    }

    let super_block = parent_inode
        .super_block
        .upgrade()
        .expect("Failed to upgrade SuperBlock weak reference in ramfs_mkdir");
    let super_block_write = super_block.write();
    let mut fs_specific_data_write = super_block_write.fs_specific_data.write();
    let inode_registry = fs_specific_data_write
        .downcast_mut::<RamFSInodeRegistry>()
        .expect(
            "Failed to downcast SuperBlock fs_specific_data to RamFSInodeRegistry in ramfs_mkdir",
        );
    let mut ramfs_parent_inode_write = ramfs_parent_inode_ref.write();
    {
        let RamFSInodeKind::Directory { children } = &mut ramfs_parent_inode_write.inode_kind
        else {
            unreachable!()
        };
        if children.contains_key(target_dir) {
            // We don't have to manually evict here as the inode being dropped will do it in its Drop
            // implementation as per the docs of Inode in `crate::fs::Inode`.
            return Err(PosixError::EEXIST);
        }
    }
    let new_inode = ramfs_mkdir_inner(
        inode_registry,
        target_dir,
        Some(&mut *ramfs_parent_inode_write),
    )?;
    new_inode.write().super_block = parent_inode.super_block.clone();
    let RamFSInodeKind::Directory { children } = &mut ramfs_parent_inode_write.inode_kind else {
        unreachable!()
    };
    children.insert(target_dir.to_string(), new_inode.read().ino);
    // There are two cases for a directory:
    // 1. Self-referential/root directory:
    //    - The "." entry points to the directory itself, contributing 1 to the link count.
    //    - The ".." entry also points to the directory itself, contributing another 1
    //      to the link count.
    //    - Total link count contribution: 2
    // 2. Non-root directory with a parent:
    //    - The "." entry points to the directory itself, contributing 1 to the link count.
    //    - The parent directory has a link to this directory as well, contributing another 1 to the link count of the new directory.
    //    - Total link count contribution to the new directory: 1 (from ".") + 1 (from the parent's link to this directory) = 2
    // So, we always set the starting `i_nlink` value of a new directory to 2 to account for the above cases.
    new_inode
        .read()
        .inner_opaque
        .downcast_ref::<Arc<RwLock<RamFSInode>>>()
        .unwrap()
        .write()
        .i_nlink += 1;

    dmesgln(
        d!(D_INFO "Created directory '{}' with inode number {} under parent inode {}", target_dir, new_inode.read().ino, parent_inode.name),
    );

    Ok(new_inode)
}

pub fn ramfs_create(dir_inode: &mut Inode, target_file: &str) -> FSResult<Arc<RwLock<Inode>>> {
    let ramfs_parent_inode_ref: RamFSInodeDowncastRef = dir_inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_create");
    let mut ramfs_parent_inode = ramfs_parent_inode_ref.write();

    if ramfs_parent_inode.i_nlink == 0 {
        // The entry has been marked as deleted, so we do not allow creating new files in this dir,
        // as per POSIX.
        return Err(PosixError::ENOENT);
    }

    let super_block_ref = dir_inode
        .super_block
        .upgrade()
        .expect("Failed to upgrade SuperBlock weak reference in ramfs_create");
    let super_block_read = super_block_ref.read();

    if let RamFSInodeKind::Directory { children } = &mut ramfs_parent_inode.inode_kind {
        if children.contains_key(target_file) {
            return Err(PosixError::EEXIST);
        }

        let mut fs_specific_data_write = super_block_read.fs_specific_data.write();
        let inode_registry = fs_specific_data_write
            .downcast_mut::<RamFSInodeRegistry>()
            .expect(
            "Failed to downcast SuperBlock fs_specific_data to RamFSInodeRegistry in ramfs_create",
        );

        let stat = RamFSInodeStat {
            size: 0,
            permissions: DEFAULT_FILE_PERMISSIONS,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        };
        let new_file_ramfs_inode =
            inode_registry.allocate_ino(RamFSInodeKind::File { data: Vec::new() }, stat);

        if let Some(new_file_ramfs_inode) = new_file_ramfs_inode {
            let new_ino = new_file_ramfs_inode.read().ino;
            children.insert(target_file.to_string(), new_ino);

            dmesgln(
                d!(D_INFO "Created file '{}' with inode number {} under parent inode {}", target_file, new_ino, dir_inode.name),
            );

            Ok(Arc::new(RwLock::new(Inode {
                ino: new_ino,
                typ: InodeKind::File,
                name: target_file.to_string(),
                inner_opaque: Box::new(new_file_ramfs_inode),
                ops: RAMFS_FILE_OPS,
                super_block: dir_inode.super_block.clone(),
                stat,
            })))
        } else {
            Err(PosixError::ENOSPC)
        }
    } else {
        Err(PosixError::ENOTDIR)
    }
}

pub fn ramfs_readdir(inode: &Inode) -> FSResult<Vec<ReaddirDirent>> {
    let ramfs_inode_ref: RamFSInodeDowncastRef = inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_readdir");
    let ramfs_inode = ramfs_inode_ref.read();
    let super_block = inode.super_block.upgrade().ok_or(PosixError::EIO)?;
    let super_block_read = super_block.read();
    let fs_specific = super_block_read.fs_specific_data.read();
    let registry = fs_specific
        .downcast_ref::<RamFSInodeRegistry>()
        .ok_or(PosixError::EIO)?;

    if let RamFSInodeKind::Directory { children } = &ramfs_inode.inode_kind {
        let mut entries = Vec::with_capacity(children.len());

        for (name, &ino) in children {
            let kind = if let Some(child_inode) = registry.get_inode(ino) {
                match child_inode.read().inode_kind {
                    RamFSInodeKind::Directory { .. } => DEntryKind::Directory,
                    RamFSInodeKind::File { .. } => DEntryKind::File,
                    RamFSInodeKind::Symlink { .. } => DEntryKind::Symlink,
                }
            } else {
                dmesgln(
                    d!(D_ERROR "Failed to find child inode {} for entry '{}' in directory inode {}", ino, name, inode.ino),
                );
                continue;
            };

            entries.push(ReaddirDirent {
                name: name.to_string(),
                ino,
                kind,
            });
        }

        Ok(entries)
    } else {
        Err(PosixError::ENOTDIR)
    }
}

pub fn ramfs_lookup(dir_inode: &Inode, name: &str) -> FSResult<Arc<RwLock<Inode>>> {
    let dir_ramfs_inode_ref: RamFSInodeDowncastRef = dir_inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_lookup");

    if let RamFSInodeKind::Directory { children } = &dir_ramfs_inode_ref.read().inode_kind {
        if let Some(inode_number) = children.get(name) {
            let super_block = dir_inode
                .super_block
                .upgrade()
                .expect("Failed to upgrade SuperBlock weak reference in ramfs_lookup");
            let super_block_read = super_block.read();
            let fs_specific_data_read = super_block_read.fs_specific_data.read();
            let inode_registry = fs_specific_data_read.downcast_ref::<RamFSInodeRegistry>()
                .expect("Failed to downcast Inode inner_opaque to Arc<RwLock<RamFSInodeRegistry>> in ramfs_lookup");
            if let Some(inode) = inode_registry.get_inode(*inode_number) {
                let inode_read = inode.read();

                Ok(Arc::new(RwLock::new(Inode {
                    ino: *inode_number,
                    typ: match &inode_read.inode_kind {
                        RamFSInodeKind::Directory { .. } => InodeKind::Directory,
                        RamFSInodeKind::File { .. } => InodeKind::File,
                        RamFSInodeKind::Symlink { .. } => InodeKind::Symlink,
                    },
                    name: name.to_string(),
                    inner_opaque: Box::new(inode.clone()),
                    ops: match &inode_read.inode_kind {
                        RamFSInodeKind::Directory { .. } => RAMFS_DIR_OPS,
                        RamFSInodeKind::File { .. } => RAMFS_FILE_OPS,
                        RamFSInodeKind::Symlink { .. } => unimplemented!(),
                    },
                    super_block: dir_inode.super_block.clone(),
                    stat: InodeStat {
                        size: inode_read.stat.size,
                        permissions: inode_read.stat.permissions,
                        uid: inode_read.stat.uid,
                        gid: inode_read.stat.gid,
                        atime: inode_read.stat.atime,
                        mtime: inode_read.stat.mtime,
                        ctime: inode_read.stat.ctime,
                    },
                })))
            } else {
                panic!(
                    "RamFS BUG!!! Inode number {} found in directory children but not in inode registry in `ramfs_lookup`. Directory inode name: `{}`, ino: {}, lookup: `{}`",
                    inode_number, dir_inode.name, dir_inode.ino, name
                );
            }
        } else {
            Err(PosixError::ENOENT)
        }
    } else {
        dmesgln(
            d!(D_ERROR "VFS BUG!!! Attempted to lookup in a non-directory inode in ramfs_lookup. RamFS Inode name: `{}`, ino: {}, lookup: `{}`",
            dir_inode.name, dir_inode.ino, name),
        );
        Err(PosixError::ENOTDIR)
    }
}

pub fn ramfs_mkdir_inner(
    inode_registry: &mut RamFSInodeRegistry,
    target_dir: &str,
    parent_inode: Option<&mut RamFSInode>,
) -> FSResult<Arc<RwLock<Inode>>> {
    let new_dir_ramfs_inode = inode_registry
        .allocate_ino(
            RamFSInodeKind::Directory {
                children: HashMap::new(),
            },
            RamFSInodeStat {
                size: 0,
                permissions: DEFAULT_DIR_PERMISSIONS,
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
            },
        )
        .ok_or(PosixError::ENOSPC)?;

    let new_inode = Arc::new(RwLock::new(Inode {
        ino: new_dir_ramfs_inode.read().ino,
        typ: InodeKind::Directory,
        name: target_dir.to_string(),
        inner_opaque: Box::new(new_dir_ramfs_inode.clone()),
        ops: RAMFS_DIR_OPS,
        super_block: Weak::new(), // We will set the correct super_block reference later when we have access to the SuperBlock in ramfs_mkdir
        stat: InodeStat {
            size: 0,
            permissions: DEFAULT_DIR_PERMISSIONS,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        },
    }));

    let new_ino = new_dir_ramfs_inode.read().ino;
    let mut new_dir_children = HashMap::new();
    new_dir_children.insert(".".to_string(), new_ino);

    if let Some(parent_ino) = parent_inode {
        new_dir_children.insert("..".to_string(), parent_ino.ino);
        parent_ino.i_nlink += 1; // Increment the link count of the parent directory for the
    // new directory's ".." entry
    } else {
        // If there is no parent inode (i.e., we are creating the root directory), then the ".."
        // entry should point to itself, and we should increment the link count by 2 to account for both the "." and ".." entries.
        new_dir_children.insert("..".to_string(), new_ino);
    }

    {
        let mut new_dir_ramfs_inode_mut = new_dir_ramfs_inode.write();
        new_dir_ramfs_inode_mut.inode_kind = RamFSInodeKind::Directory {
            children: new_dir_children,
        };
    }

    dmesgln(
        d!(D_INFO "Created directory '{}' with inode number {} in ramfs_mkdir_inner", target_dir, new_ino),
    );

    Ok(new_inode)
}

pub fn ramfs_unlink_from_parent_inode(parent_inode: &mut Inode, target_file: &str) -> FSResult<()> {
    let ramfs_parent_inode_ref: RamFSInodeDowncastRef = parent_inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_inode_unlink");
    let mut ramfs_parent_inode = ramfs_parent_inode_ref.write();

    if let RamFSInodeKind::Directory { children } = &mut ramfs_parent_inode.inode_kind {
        if let Some(inode_number) = children.get(target_file) {
            let inode_number = *inode_number;
            dmesgln(
                d!(D_INFO "Unlinked inode {} from parent inode {}", target_file, parent_inode.name),
            );

            let super_block = parent_inode.super_block.upgrade().expect(
                "Failed to upgrade SuperBlock weak reference in ramfs_inode_unlink_from_parent_inode",
            );
            let super_block_read = super_block.read();
            let mut fs_specific_data_write = super_block_read.fs_specific_data.write();
            let inode_registry = fs_specific_data_write.downcast_mut::<RamFSInodeRegistry>()
                .expect("Failed to downcast SuperBlock fs_specific_data to RamFSInodeRegistry in ramfs_inode_unlink_from_parent_inode");

            if let Some(inode) = inode_registry.get_inode(inode_number) {
                let mut inode_ref = inode.write();
                return if matches!(inode_ref.inode_kind, RamFSInodeKind::Directory { .. }) {
                    Err(PosixError::EISDIR)
                } else {
                    children.remove_entry(target_file);
                    inode_ref.i_nlink = inode_ref.i_nlink.checked_sub(1)
                        .expect("FS BUG!!! Attempted to decrement i_nlink below 0 in ramfs_inode_unlink_from_parent_inode");

                    Ok(())
                };
            }
        } else {
            return Err(PosixError::ENOENT);
        }
    }

    Err(PosixError::ENOTDIR)
}

pub fn ramfs_rmdir(parent_inode: &mut Inode, target_dir: &str) -> FSResult<()> {
    let ramfs_parent_inode_ref: RamFSInodeDowncastRef = parent_inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_rmdir");
    let mut ramfs_parent_inode = ramfs_parent_inode_ref.write();

    if let RamFSInodeKind::Directory {
        children: parent_children,
    } = &mut ramfs_parent_inode.inode_kind
        && let Some((_, inode_number)) = parent_children.get_key_value(target_dir)
    {
        let super_block = parent_inode
            .super_block
            .upgrade()
            .expect("Failed to upgrade SuperBlock weak reference in ramfs_rmdir");
        let super_block_read = super_block.read();
        let mut fs_specific_data_write = super_block_read.fs_specific_data.write();
        let inode_registry = fs_specific_data_write.downcast_mut::<RamFSInodeRegistry>()
                .expect("Failed to downcast SuperBlock fs_specific_data to RamFSInodeRegistry in ramfs_rmdir");
        if let Some(inode) = inode_registry.get_inode(*inode_number) {
            let mut inode_write = inode.write();
            if let RamFSInodeKind::Directory { children } = &mut inode_write.inode_kind {
                return if !children.keys().all(|name| name == "." || name == "..") {
                    Err(PosixError::ENOTEMPTY)
                } else {
                    children.clear();
                    parent_children.remove_entry(target_dir);
                    inode_write.i_nlink = inode_write
                        .i_nlink
                        .checked_sub(2)
                        .expect("FS BUG!!! Attempted to decrement i_nlink below 0 in ramfs_rmdir");
                    ramfs_parent_inode.i_nlink = ramfs_parent_inode.i_nlink.checked_sub(1)
                            .expect("FS BUG!!! Attempted to decrement parent directory i_nlink below 0 in ramfs_rmdir");
                    Ok(())
                };
            }
        }
    }

    Err(PosixError::ENOTDIR)
}

pub fn ramfs_link(parent_inode: &mut Inode, new_name: &str, target: &Inode) -> FSBooleanResult {
    let ramfs_parent_inode_ref: RamFSInodeDowncastRef = parent_inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_link");
    let mut ramfs_parent_inode = ramfs_parent_inode_ref.write();

    if let RamFSInodeKind::Directory { children } = &mut ramfs_parent_inode.inode_kind {
        if children.contains_key(new_name) {
            return Err(PosixError::EEXIST);
        }

        let target_ramfs_inode_ref: RamFSInodeDowncastRef = target
            .inner_opaque
            .downcast_ref()
            .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_link");
        let mut target_ramfs_inode = target_ramfs_inode_ref.write();

        target_ramfs_inode.i_nlink = target_ramfs_inode
            .i_nlink
            .checked_add(1)
            .ok_or(PosixError::EMLINK)?;
        children.insert(new_name.to_string(), target.ino);

        dmesgln(
            d!(D_INFO "Created hard link '{}' to inode {} in parent inode {}", new_name, target.ino, parent_inode.name),
        );

        Ok(())
    } else {
        Err(PosixError::ENOTDIR)
    }
}
