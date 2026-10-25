use crate::arch::get_rtc_time;
use crate::d;
use crate::dbg::{D_DEBUG, D_ERROR, D_INFO, D_WARNING, dmesgln};
use crate::fs::path::Path;
use crate::fs::ramfs::RamFS;
use crate::fs::{
    DEntryKind, Dentry, FSBooleanResult, FSResult, FileHandle, FileSystem, Inode, InodeKind,
    InodeStat, Mode, OpenFlags, ReaddirDirent, SuperBlock,
};
use crate::kutils::MAX_SYMLINKS;
use crate::posix::PosixError;
use crate::time::SystemTime;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use hashbrown::HashMap;
use spin::{Once, RwLock};

pub static VFS: Once<RwLock<VFS>> = Once::new();
static NEXT_MOUNT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct Mountpoint {
    pub fs_name: String,
    pub superblock: Arc<RwLock<SuperBlock>>,
    pub host_dentry: Weak<Dentry>,
}

/// TODO: Implement permissions checking when users are implemented
/// TODO: properly sync atime, ctime and mtime when files are read/written
#[allow(clippy::upper_case_acronyms)]
pub struct VFS {
    fs: HashMap<String, Box<dyn FileSystem>>,
    active_mounts: RwLock<HashMap<u64, Mountpoint>>,
    root_dentry: Arc<Dentry>,
}

impl VFS {
    pub fn new() -> Self {
        let mut active_mounts = HashMap::new();

        let root_fs = Box::new(RamFS::new());
        let root_mount_id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
        let root_superblock = root_fs
            .mount(root_mount_id, None)
            .expect("Failed to mount root filesystem");
        let root_dentry = root_superblock.read().root.clone();
        let mountpoint = Mountpoint {
            fs_name: root_fs.name().to_string(),
            superblock: root_superblock,
            // root directory doesn't have this as there's no filesystem mounted yet. We also
            // do not set this to avoid infinite loop inside [`VFS::resolve_path`].
            host_dentry: Weak::new(),
        };
        active_mounts.insert(root_mount_id, mountpoint);

        VFS {
            fs: {
                let mut map: HashMap<String, Box<dyn FileSystem>> = HashMap::new();
                map.insert(root_fs.name().to_string(), root_fs);
                map
            },
            active_mounts: RwLock::new(active_mounts),
            root_dentry,
        }
    }

    pub fn register_fs(&mut self, fs: Box<dyn FileSystem>) {
        dmesgln(d!(D_INFO "Registering filesystem: {}", fs.name()));
        self.fs.insert(fs.name().to_string(), fs);
    }

    pub fn canonicalize_path(&self, path: &Path, cwd: Option<Arc<Dentry>>) -> FSResult<Path> {
        let (_, mut current) = self.resolve_path(path, cwd, false, false)?;
        let is_dir = current.kind == DEntryKind::Directory;
        let mut canonical_path = Path::new("/");

        loop {
            let name = current.name.read().clone();

            if name != "/" {
                canonical_path.push_front(name);
            }

            let parent = current
                .parent
                .read()
                .upgrade()
                .expect("VFS: dentry hierarchy broken");

            if Arc::ptr_eq(&current, &parent) {
                break;
            }

            current = parent;
        }

        canonical_path.set_is_dir(is_dir);

        Ok(canonical_path)
    }

    /// Resolves a path to dentry, and returns the parent dentry and the target dentry.
    ///
    /// # Note
    /// `cwd` is ignored if the given path is absolute.
    ///
    /// We resolve component by component. We go through the components of the path,
    /// and for each component, we first check if it exists in the current dentry's subdir cache.
    /// If it does, we move to that dentry. If it doesn't, we perform a lookup on the current
    /// dentry's inode to see if the component exists. If it does, we create a new dentry for it,
    /// insert it into the cache, and move to that dentry. If it doesn't, we return an error.
    fn resolve_path(
        &self,
        path: &Path,
        cwd: Option<Arc<Dentry>>,
        stop_before_last_component: bool,
        stop_before_symlink: bool,
    ) -> FSResult<(Arc<Dentry>, Arc<Dentry>)> {
        let mut current = if path.is_absolute() {
            self.root_dentry.clone()
        } else if let Some(cwd) = cwd {
            cwd
        } else {
            dmesgln(
                d!(D_WARNING "Relative path {} provided to resolve_path, but no CWD provided", path),
            );
            return Err(PosixError::ENOENT);
        };

        let mut symlink_count = 0;
        let mut prev = current.clone();
        let mut components: VecDeque<String> = path.iter().map(|c| c.to_string()).collect();
        while let Some(component) = components.pop_front() {
            if stop_before_last_component && components.is_empty() {
                components.push_front(component);
                break;
            }

            if component == "." {
                continue;
            }

            if component == ".." {
                let parent_weak = current
                    .parent
                    .read()
                    .upgrade()
                    .expect("VFS: dentry hierarchy broken");

                // Check if we're at a root
                if Arc::ptr_eq(&current, &parent_weak) {
                    // Check if the current filesystem is mounted inside another directory
                    let superblock_id = current
                        .inode
                        .read()
                        .super_block
                        .upgrade()
                        .ok_or(PosixError::EIO)?
                        .read()
                        .mount_id;

                    let active_mounts_read = self.active_mounts.read();
                    let mount_data = active_mounts_read
                        .get(&superblock_id)
                        .ok_or(PosixError::EIO)?;

                    if let Some(host_dentry) = &mount_data.host_dentry.upgrade() {
                        current = host_dentry.clone();
                    } else {
                        continue;
                    }
                }

                prev = current.clone();
                let parent = current
                    .parent
                    .read()
                    .upgrade()
                    .expect("VFS: dentry hierarchy broken");
                current = parent;

                continue;
            };

            let next_dentry = {
                let subdirs = current.subdirs.as_ref().ok_or(PosixError::ENOTDIR)?;
                let subdirs_read = subdirs.read();

                if let Some(cached) = subdirs_read.get(&component) {
                    cached.clone()
                } else {
                    // Drop subdirs readlock to avoid deadlock when we push the new dentry into the cache after lookup.
                    drop(subdirs_read);

                    // Cache miss, need to do a lookup
                    let parent_inode = current.inode.read();
                    let lookup_fn = parent_inode.ops.lookup.ok_or_else(|| {
                        dmesgln(d!(D_WARNING "Lookup not supported on inode {} in VFS resolve_path", parent_inode.name));
                        PosixError::ENOTDIR
                    })?;

                    let child_inode = lookup_fn(&parent_inode, &component)?;

                    let mut new_dentry = Arc::new(Dentry {
                        name: RwLock::new(component.to_string()),
                        inode: Arc::clone(&child_inode),
                        parent: RwLock::new(Arc::downgrade(&current)),
                        kind: match child_inode.read().typ {
                            InodeKind::Directory => DEntryKind::Directory,
                            InodeKind::File => DEntryKind::File,
                            InodeKind::Symlink => DEntryKind::Symlink,
                            InodeKind::BlockDev => DEntryKind::BlockDev,
                            InodeKind::CharDev => DEntryKind::CharDev,
                        },
                        subdirs: if matches!(child_inode.read().typ, InodeKind::Directory) {
                            Some(RwLock::new(HashMap::new()))
                        } else {
                            None
                        },
                        mounted_here: Arc::new(RwLock::new(vec![])),
                    });

                    let mut subdirs_write = subdirs.write();
                    // In multithreaded environment, other thread may have "beat" us
                    // when it comes to resolving same path, so we need to check the cache
                    // again before inserting the new dentry.
                    if let Some(cached) = subdirs_write.get(&component) {
                        new_dentry = cached.clone();
                    } else {
                        subdirs_write.insert(component.to_string(), new_dentry.clone());
                    }
                    new_dentry
                }
            };

            let mut target = next_dentry;
            loop {
                let mount_id = target.mounted_here.read().last().copied();

                if let Some(mount_id) = mount_id {
                    let active_mounts_read = self.active_mounts.read();
                    let mount_data = active_mounts_read.get(&mount_id).ok_or(PosixError::EIO)?;

                    target = mount_data.superblock.read().root.clone();
                } else {
                    break;
                }
            }

            if target.kind == DEntryKind::Symlink {
                // If this is the final component, the path has no trailing slash and
                // the caller asked not to follow the symlink, we stop here.
                // We continue if there's a trailing slash because POSIX requires us to
                // do so.
                if components.is_empty() && !path.is_dir() && stop_before_symlink {
                    prev = current;
                    current = target;
                    break;
                }

                symlink_count += 1;
                if symlink_count > MAX_SYMLINKS {
                    dmesgln(
                        d!(D_WARNING "Too many symlinks encountered while resolving path {} in VFS resolve_path, possible symlink loop", path),
                    );
                    return Err(PosixError::ELOOP);
                }

                let inode = target.inode.read();
                let readlink_fn = inode.ops.readlink.ok_or_else(|| {
                    dmesgln(d!(D_WARNING "Readlink not supported on inode {} in VFS resolve_path", inode.name));
                    PosixError::EINVAL
                })?;
                let link_target_path = Path::new(&readlink_fn(&inode)?);
                dmesgln(
                    d!(D_DEBUG "Resolving symlink {} to {}", target.name.read(), link_target_path),
                );

                if link_target_path.is_absolute() {
                    current = self.root_dentry.clone();
                }

                for component in link_target_path.iter().rev() {
                    components.push_front(component.to_owned());
                }

                continue;
            }

            prev = current;
            current = target;
        }

        if path.is_dir() && current.kind != DEntryKind::Directory {
            dmesgln(
                d!(D_WARNING "Path {} is a directory path, but resolved dentry is not a directory in VFS resolve_path", path),
            );
            return Err(PosixError::ENOTDIR);
        }

        Ok((prev, current))
    }

    pub fn exists(&mut self, path: &Path, cwd: Option<Arc<Dentry>>, follow_symlinks: bool) -> bool {
        self.resolve_path(path, cwd, false, follow_symlinks).is_ok()
    }

    pub fn open(
        &mut self,
        path: Path,
        cwd: Option<Arc<Dentry>>,
        flags: OpenFlags,
    ) -> FSResult<FileHandle> {
        match self.resolve_path(&path, cwd.clone(), false, false) {
            Ok((_, target_dentry)) => {
                if matches!(target_dentry.kind, DEntryKind::Directory) {
                    if flags.intersects(
                        OpenFlags::O_WRONLY
                            | OpenFlags::O_RDWR
                            | OpenFlags::O_TRUNC
                            | OpenFlags::O_CREAT,
                    ) {
                        dmesgln(
                            d!(D_WARNING "Attempted to open directory {} with non-readonly mode {:?} in VFS open, this is not supported", path, flags),
                        );
                        Err(PosixError::EISDIR)
                    } else {
                        let file_handle = FileHandle {
                            inode: target_dentry.inode.clone(),
                            position: 0,
                            flags,
                            fd: 0, // TODO: implement file descriptor allocation
                        };
                        Ok(file_handle)
                    }
                } else {
                    if flags.contains(OpenFlags::O_CREAT | OpenFlags::O_EXCL) {
                        dmesgln(
                            d!(D_WARNING "Attempted to open file {} with O_CREAT and O_EXCL flags, but the file already exists in VFS open", path),
                        );
                        return Err(PosixError::EEXIST);
                    }

                    if flags.contains(OpenFlags::O_DIRECTORY) {
                        dmesgln(
                            d!(D_WARNING "Attempted to open file {} with O_DIRECTORY flag, but the file is not a directory in VFS open", path),
                        );
                        return Err(PosixError::ENOTDIR);
                    }

                    if flags.contains(OpenFlags::O_TRUNC) {
                        if !flags.contains(OpenFlags::O_WRONLY)
                            && !flags.contains(OpenFlags::O_RDWR)
                        {
                            dmesgln(
                                d!(D_WARNING "Attempted to open file {} with O_TRUNC flag but without write permissions in VFS open", path),
                            );
                            return Err(PosixError::EINVAL);
                        }

                        let mut inode = target_dentry.inode.write();
                        let truncate_fn = inode.ops.truncate.ok_or_else(|| {
                            dmesgln(d!(D_WARNING "Truncate not supported on inode {} in VFS open", inode.name));
                            PosixError::ENOSYS
                        })?;

                        truncate_fn(&mut inode, 0)?;

                        let now = get_rtc_time() as SystemTime;
                        inode.stat.size = 0;
                        inode.stat.mtime = now;
                        inode.stat.ctime = now;

                        Self::sync_stat(&inode)?
                    }

                    let file_handle = FileHandle {
                        inode: target_dentry.inode.clone(),
                        position: 0,
                        flags,
                        fd: 0, // TODO: implement file descriptor allocation
                    };

                    Ok(file_handle)
                }
            }
            Err(e) => {
                if e == PosixError::ENOENT && flags.contains(OpenFlags::O_CREAT) && !path.is_dir() {
                    if flags.contains(OpenFlags::O_DIRECTORY) {
                        dmesgln(
                            d!(D_WARNING "Attempted to create directory {} with O_DIRECTORY flag, but the path is not a directory in VFS open", path),
                        );
                        return Err(PosixError::EINVAL);
                    }

                    let file_name = path.last_component().ok_or_else(|| {
                        dmesgln(d!(D_WARNING "Attempted to create a file with an empty name at path {} in VFS open", path));
                        PosixError::EINVAL
                    })?;

                    dmesgln(
                        d!(D_DEBUG "File {} not found in VFS open, but O_CREAT flag is set, attempting to create the file", path),
                    );
                    let (_, parent_dir_dentry) = self.resolve_path(&path, cwd, true, false)?;

                    dmesgln(
                        d!(D_DEBUG "Creating file {} in directory with inode name {} in VFS open", file_name, parent_dir_dentry.inode.read().name),
                    );

                    let mut parent_dir_inode = parent_dir_dentry.inode.write();
                    let create_fn = parent_dir_inode.ops.create.ok_or_else(|| {
                        dmesgln(d!(D_WARNING "Create not supported on inode {} in VFS open", parent_dir_inode.name));
                        PosixError::ENOSYS
                    })?;
                    let new_inode = create_fn(&mut parent_dir_inode, file_name)?;
                    let new_dentry = Arc::new(Dentry {
                        name: RwLock::new(path.last_component().unwrap().to_string()),
                        inode: Arc::clone(&new_inode),
                        parent: RwLock::new(Arc::downgrade(&parent_dir_dentry)),
                        kind: match new_inode.read().typ {
                            InodeKind::Directory => DEntryKind::Directory,
                            InodeKind::File => DEntryKind::File,
                            InodeKind::Symlink => DEntryKind::Symlink,
                            InodeKind::BlockDev => DEntryKind::BlockDev,
                            InodeKind::CharDev => DEntryKind::CharDev,
                        },
                        subdirs: None,
                        mounted_here: Arc::new(RwLock::new(vec![])),
                    });
                    parent_dir_dentry
                        .subdirs
                        .as_ref()
                        .ok_or(PosixError::ENOTDIR)?
                        .write()
                        .insert(file_name.to_string(), new_dentry);

                    let now = get_rtc_time() as SystemTime;
                    parent_dir_inode.stat.ctime = now;
                    parent_dir_inode.stat.mtime = now;
                    Self::sync_stat(&parent_dir_inode)?;

                    Ok(FileHandle {
                        inode: new_inode,
                        position: 0,
                        flags,
                        fd: 0, // TODO: implement file descriptor allocation
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    fn sync_stat(inode: &Inode) -> FSBooleanResult {
        if let Some(fs_sync_stat) = inode.ops.sync_stat {
            fs_sync_stat(inode)?;
        } else {
            dmesgln(
                d!(D_WARNING "Sync stat not supported on inode {} in VFS sync_stat", inode.ino),
            );
        }

        Ok(())
    }

    pub fn read(&mut self, handle: &mut FileHandle, buf: &mut [u8], bytes: u64) -> FSResult<u64> {
        if handle.flags.contains(OpenFlags::O_WRONLY) {
            dmesgln(
                d!(D_WARNING "Attempted to read from a file opened with O_WRONLY flag in VFS read"),
            );
            return Err(PosixError::EBADF);
        }

        let inode = handle.inode.read();
        if inode.typ == InodeKind::Directory {
            dmesgln(
                d!(D_WARNING "Attempted to read from a directory inode {} in VFS read", inode.name),
            );
            return Err(PosixError::EISDIR);
        }
        let read_fn = inode.ops.read.ok_or_else(|| {
            dmesgln(d!(D_WARNING "Read not supported on inode {} in VFS read", inode.name));
            PosixError::ENOSYS
        })?;

        let bytes_read = read_fn(&inode, buf, bytes, handle.position)?;
        let now = get_rtc_time() as SystemTime;
        let needs_atime_update = bytes_read != 0 && inode.stat.atime != now;
        drop(inode);

        if needs_atime_update {
            let mut inode = handle.inode.write();
            inode.stat.atime = now;
            if let Err(e) = Self::sync_stat(&inode) {
                dmesgln(
                    d!(D_ERROR "Failed to sync stat for inode {} after read in VFS read: {:?}", inode.name, e),
                );
            }
        }

        handle.position += bytes_read;
        Ok(bytes_read)
    }

    pub fn write(&mut self, handle: &mut FileHandle, buf: &[u8], bytes: u64) -> FSResult<u64> {
        if !handle
            .flags
            .intersects(OpenFlags::O_WRONLY | OpenFlags::O_RDWR)
        {
            dmesgln(
                d!(D_WARNING "Attempted to write to a file that is not opened as writable in VFS write"),
            );
            return Err(PosixError::EBADF);
        }

        let mut inode = handle.inode.write();
        if inode.typ == InodeKind::Directory {
            dmesgln(
                d!(D_WARNING "Attempted to write to a directory inode {} in VFS write", inode.name),
            );
            return Err(PosixError::EISDIR);
        }

        let write_fn = inode.ops.write.ok_or_else(|| {
            dmesgln(d!(D_WARNING "Write not supported on inode {} in VFS write", inode.name));
            PosixError::ENOSYS
        })?;

        if handle.flags.contains(OpenFlags::O_APPEND) {
            handle.position = inode.stat.size;
        }

        let bytes_written = write_fn(&inode, handle.position, bytes, buf)?;
        let new_position = handle.position + bytes_written;

        if bytes_written != 0 {
            let current_time = get_rtc_time() as SystemTime;
            inode.stat.mtime = current_time;
            inode.stat.ctime = current_time;

            if new_position > inode.stat.size {
                inode.stat.size = new_position;
            }

            if let Err(e) = Self::sync_stat(&inode) {
                dmesgln(
                    d!(D_ERROR "Failed to sync stat for inode {} after write in VFS write: {:?}", inode.name, e),
                );
            }
        }

        handle.position = new_position;
        Ok(bytes_written)
    }

    pub fn close(&mut self, handle: FileHandle) -> FSBooleanResult {
        let inode = handle.inode.read();
        if let Some(close_fn) = inode.ops.close {
            close_fn(&handle)?;
        } else {
            dmesgln(
                d!(D_DEBUG "Close not supported on inode `{}` in VFS close, skipping", inode.name),
            );
        }

        dmesgln(d!(D_DEBUG "Closed file handle for inode `{}` in VFS close", inode.name));

        Ok(())
    }

    pub fn stat(&self, handle: &FileHandle) -> FSResult<InodeStat> {
        Ok(handle.inode.read().stat)
    }

    pub fn readdir(&self, handle: &FileHandle) -> FSResult<Vec<ReaddirDirent>> {
        let inode = handle.inode.read();

        if inode.typ != InodeKind::Directory {
            dmesgln(
                d!(D_WARNING "Attempted to read directory entries from a non-directory inode {} in VFS readdir", inode.name),
            );
            return Err(PosixError::ENOTDIR);
        }

        inode.ops.readdir.ok_or_else(|| {
            dmesgln(d!(D_WARNING "Readdir not supported on inode {} in VFS readdir", inode.name));
            PosixError::ENOSYS
        })?(&inode)
    }

    pub fn mkdir(&mut self, path: Path, cwd: Option<Arc<Dentry>>, mode: Mode) -> FSBooleanResult {
        let dir_name = path.last_component().ok_or_else(|| {
            dmesgln(d!(D_WARNING "Attempted to create a directory with an empty name at path {} in VFS mkdir", path));
            PosixError::EINVAL
        })?;

        let (_, parent_dir_dentry) = self.resolve_path(&path, cwd, true, false)?;

        // Check if there's an existing entry under parent directory with the same name
        if let Some(subdirs) = &parent_dir_dentry.subdirs {
            let subdirs_read = subdirs.read();
            if subdirs_read.contains_key(dir_name) {
                dmesgln(
                    d!(D_WARNING "Attempted to create a directory {} in VFS mkdir, but an entry with the same name already exists in the parent directory", dir_name),
                );
                return Err(PosixError::EEXIST);
            }
        }

        // TODO: permission check

        let mut parent_dir_inode = parent_dir_dentry.inode.write();
        let mkdir_fn = parent_dir_inode.ops.mkdir.ok_or_else(|| {
            dmesgln(
                d!(D_WARNING "Mkdir not supported on inode {} in VFS mkdir", parent_dir_inode.name),
            );
            PosixError::ENOSYS
        })?;

        let dir_inode = mkdir_fn(&mut parent_dir_inode, dir_name, mode)?;
        let dir_dentry = Arc::new(Dentry {
            name: RwLock::new(dir_name.to_string()),
            inode: Arc::clone(&dir_inode),
            parent: RwLock::new(Arc::downgrade(&parent_dir_dentry)),
            kind: DEntryKind::Directory,
            subdirs: Some(RwLock::new(HashMap::new())),
            mounted_here: Arc::new(RwLock::new(vec![])),
        });

        parent_dir_dentry
            .subdirs
            .as_ref()
            .ok_or(PosixError::ENOTDIR)?
            .write()
            .insert(dir_name.to_string(), dir_dentry);

        Ok(())
    }

    /// https://pubs.opengroup.org/onlinepubs/9799919799/functions/rmdir.html
    pub fn rmdir(&mut self, path: Path, cwd: Option<Arc<Dentry>>) -> FSBooleanResult {
        let dir_name = path.last_component().map(|v| v.as_str()).unwrap_or("");

        // If the path argument refers to a path whose final component is
        // either dot or dot-dot, rmdir() shall fail.
        if dir_name == ".." || dir_name == "." {
            return Err(PosixError::EINVAL);
        }

        let (parent_dir_dentry, target_dir_dentry) = self.resolve_path(&path, cwd, false, false)?;

        if target_dir_dentry.kind != DEntryKind::Directory {
            dmesgln(
                d!(D_WARNING "Attempted to remove a directory {} in VFS rmdir, but the target is not a directory", dir_name),
            );
            return Err(PosixError::ENOTDIR);
        }

        // Check if the target directory is the same as the parent directory,
        // which would indicate an attempt to remove the root directory.
        let parent_weak = target_dir_dentry
            .parent
            .read()
            .upgrade()
            .ok_or(PosixError::EIO)?;
        if Arc::ptr_eq(&parent_weak, &target_dir_dentry) {
            dmesgln(
                d!(D_WARNING "Attempted to remove the root directory {} in VFS rmdir, which is not allowed", dir_name),
            );
            return Err(PosixError::EBUSY);
        }

        if !target_dir_dentry.mounted_here.read().is_empty() {
            dmesgln(
                d!(D_WARNING "Attempted to remove a directory {} in VFS rmdir, but the directory is a mount point", dir_name),
            );
            return Err(PosixError::EBUSY);
        }

        if let Some(dirs) = target_dir_dentry.subdirs.as_ref()
            && !dirs
                .read()
                .iter()
                .all(|(child, _)| child == "." || child == "..")
        {
            dmesgln(
                d!(D_WARNING "Attempted to remove a directory {} in VFS rmdir, but the directory is not empty", dir_name),
            );

            return Err(PosixError::ENOTEMPTY);
        }

        {
            let mut parent_dir_dentry = parent_dir_dentry.inode.write();
            let rmdir_fn = parent_dir_dentry.ops.rmdir.ok_or_else(|| {
                dmesgln(d!(D_WARNING "Rmdir not supported on inode {} in VFS rmdir", parent_dir_dentry.name));
                PosixError::ENOSYS
            })?;

            rmdir_fn(&mut parent_dir_dentry, dir_name)?;
        }

        if let Some(subdirs) = parent_dir_dentry.subdirs.as_ref() {
            subdirs.write().remove(dir_name);
        }

        *target_dir_dentry.parent.write() = Weak::new();

        Ok(())
    }

    pub fn mount(
        &mut self,
        path: Path,
        device: Option<&FileHandle>,
        fs_name: &str,
    ) -> FSBooleanResult {
        // TODO: we may have to handle mounting devices separately?

        let (_, mount_path_dentry) = self.resolve_path(&path, None, false, false).map_err(|e| {
            dmesgln(d!(D_WARNING "Failed to resolve mount path {} in VFS mount: {:?}", path, e));
            PosixError::ENOENT
        })?;
        if mount_path_dentry.kind != DEntryKind::Directory {
            dmesgln(
                d!(D_WARNING "Attempted to mount on path {} which is not a directory in VFS mount", path),
            );
            return Err(PosixError::ENOTDIR);
        }

        let fs = self.fs.get(fs_name).ok_or(PosixError::ENOSYS)?;

        if device.is_none() && fs.requires_dev() {
            dmesgln(
                d!(D_WARNING "Filesystem {} requires a device, but no device was provided. Mounting failed.", fs_name),
            );
            return Err(PosixError::ENOSYS);
        }

        // The VFS maintains a map of active mounts, and each dentry has a list of mount IDs
        // that are mounted to it.

        let mount_id = NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst);
        let superblock = fs.mount(mount_id, device)?;
        let mountpoint = Mountpoint {
            fs_name: fs_name.to_string(),
            superblock,
            host_dentry: Arc::downgrade(&mount_path_dentry),
        };

        self.active_mounts.write().insert(mount_id, mountpoint);
        mount_path_dentry.mounted_here.write().push(mount_id);

        dmesgln(
            d!(D_DEBUG "Mounted filesystem at path: {}, dentry: {:?}, mount stack at dentry: {:?}", path, mount_path_dentry, mount_path_dentry.mounted_here.read()),
        );

        Ok(())
    }

    pub fn umount(&mut self, path: Path, cwd: Option<Arc<Dentry>>) -> FSBooleanResult {
        let (_, target_dentry) = self.resolve_path(&path, cwd, false, false)?;

        dmesgln(d!(D_DEBUG "Attempting umount at {:?}, path: {}", target_dentry, path));

        let superblock_weak = target_dentry.inode.read().super_block.clone();
        let superblock = superblock_weak.upgrade().ok_or(PosixError::EIO)?;

        if !Arc::ptr_eq(&target_dentry, &superblock.read().root) {
            dmesgln(
                d!(D_WARNING "Attempt to unmount a filesystem at a non-root dentry is not allowed"),
            );
            return Err(PosixError::EINVAL);
        }

        let mount_id = superblock.read().mount_id;
        let mut active_mounts = self.active_mounts.write();
        let mount_data = active_mounts.get(&mount_id).ok_or(PosixError::EINVAL)?;
        let host_dentry = mount_data.host_dentry.upgrade().ok_or_else(|| {
            dmesgln(
                d!(D_WARNING "Attempt to unmount the system root filesystem is not allowed at all"),
            );
            PosixError::EINVAL
        })?;

        let mut mounted_here = host_dentry.mounted_here.write();

        if mounted_here.last() != Some(&mount_id) {
            dmesgln(
                d!(D_WARNING "Attempt to umount a filesystem that is shadowed by another mountpoint"),
            );
            return Err(PosixError::EINVAL);
        }

        target_dentry.flush_cache();

        let expected_root_baseline_refs = 2;

        if Arc::strong_count(&target_dentry) > expected_root_baseline_refs {
            dmesgln(d!(D_WARNING "Attempt to unmount busy filesystem"));
            return Err(PosixError::EBUSY);
        }

        if let Some(ops_fn) = mount_data.superblock.read().ops.sync {
            ops_fn(&mount_data.superblock.read())?;
        }

        mounted_here.pop();
        active_mounts.remove(&mount_id);

        dmesgln(d!(D_INFO "Successfully unmounted filesystem at path: {}, \
                                dentry: {:?}, \
                                mount stack at dentry: {:?}", path, target_dentry, mounted_here));

        Ok(())
    }

    pub fn readlink(&mut self, path: &Path, cwd: Option<Arc<Dentry>>) -> FSResult<String> {
        let (_, target_dentry) = self.resolve_path(path, cwd, false, true)?;

        if target_dentry.kind != DEntryKind::Symlink {
            dmesgln(d!(D_WARNING "Attempted to readlink on a non-symlink path: {}", path));
            return Err(PosixError::EINVAL);
        }

        let inode = target_dentry.inode.read();
        let readlink_fn = inode.ops.readlink.ok_or_else(|| {
            dmesgln(d!(D_WARNING "Readlink not supported on inode {} in VFS readlink", inode.name));
            PosixError::ENOSYS
        })?;

        let link_target = readlink_fn(&inode)?;
        Ok(link_target)
    }

    pub fn symlink(
        &mut self,
        target: &str,
        link_path: &Path,
        cwd: Option<Arc<Dentry>>,
    ) -> FSBooleanResult {
        if self.exists(link_path, cwd.clone(), false) {
            dmesgln(
                d!(D_WARNING "Attempted to create a symlink at an existing path: {}", link_path),
            );
            return Err(PosixError::EEXIST);
        }

        let symlink_name = link_path.last_component().ok_or(PosixError::ENOENT)?;

        let (_, parent_dir_dentry) = self.resolve_path(link_path, cwd, true, false)?;
        let parent_dir_inode = parent_dir_dentry.inode.write();
        let symlink_fn = parent_dir_inode.ops.symlink.ok_or_else(|| {
            dmesgln(d!(D_WARNING "Symlink not supported on inode {} in VFS symlink", parent_dir_inode.name));
            PosixError::ENOSYS
        })?;

        let symlink_res = symlink_fn(&parent_dir_inode, target, symlink_name)?;
        let symlink_dentry = Arc::new(Dentry {
            name: RwLock::new(symlink_name.to_string()),
            kind: DEntryKind::Symlink,
            inode: symlink_res,
            parent: RwLock::new(Arc::downgrade(&parent_dir_dentry)),
            subdirs: None,
            mounted_here: Arc::new(RwLock::new(vec![])),
        });

        parent_dir_dentry
            .subdirs
            .as_ref()
            .ok_or(PosixError::EINVAL)?
            .write()
            .insert(symlink_name.to_string(), symlink_dentry);

        Ok(())
    }

    /// https://pubs.opengroup.org/onlinepubs/9799919799/functions/link.html
    pub fn link(
        &mut self,
        oldpath: &Path,
        newpath: &Path,
        cwd: Option<Arc<Dentry>>,
    ) -> FSBooleanResult {
        if oldpath.is_empty() || newpath.is_empty() {
            return Err(PosixError::ENOENT);
        }

        // Resolving oldpath. follow_last_symlink = false (POSIX defaults to not following symlinks for link)
        let (_, old_dentry) = self.resolve_path(oldpath, cwd.clone(), false, false)?;

        if old_dentry.kind == DEntryKind::Directory {
            return Err(PosixError::EPERM); // POSIX explicitly forbids hard-linking directories
        }

        if newpath.is_dir() {
            return Err(PosixError::ENOENT);
        }

        let new_name = newpath.last_component().ok_or(PosixError::ENOENT)?;
        if new_name == "." || new_name == ".." {
            return Err(PosixError::EINVAL);
        }

        let (_, new_parent_dentry) = self.resolve_path(newpath, cwd, true, false)?;
        if new_parent_dentry.kind != DEntryKind::Directory {
            return Err(PosixError::ENOTDIR);
        }

        // [EXDEV]: Check if they are on the same mounted filesystem
        let old_sb = old_dentry
            .inode
            .read()
            .super_block
            .upgrade()
            .ok_or(PosixError::EIO)?;
        let new_sb = new_parent_dentry
            .inode
            .read()
            .super_block
            .upgrade()
            .ok_or(PosixError::EIO)?;
        if !Arc::ptr_eq(&old_sb, &new_sb) {
            return Err(PosixError::EXDEV);
        }

        if let Some(subdirs) = &new_parent_dentry.subdirs
            && subdirs.read().contains_key(new_name)
        {
            return Err(PosixError::EEXIST);
        }

        let mut parent_inode = new_parent_dentry.inode.write();
        let link_fn = parent_inode.ops.link.ok_or(PosixError::ENOSYS)?;

        // Lock target to pass safely into the driver
        let target_inode = old_dentry.inode.read();
        link_fn(&mut parent_inode, new_name, &target_inode)?;
        drop(target_inode); // Drop target lock before mutating VFS stat

        let now = get_rtc_time() as SystemTime;

        // Update Parent Metadata
        parent_inode.stat.mtime = now;
        parent_inode.stat.ctime = now;
        let _ = Self::sync_stat(&parent_inode);
        drop(parent_inode);

        let mut target_write = old_dentry.inode.write();
        target_write.stat.ctime = now;
        let _ = Self::sync_stat(&target_write);
        drop(target_write);

        // Map the new Dentry to the exact same Inode
        let new_dentry = Arc::new(Dentry {
            name: RwLock::new(new_name.to_string()),
            inode: Arc::clone(&old_dentry.inode),
            parent: RwLock::new(Arc::downgrade(&new_parent_dentry)),
            kind: old_dentry.kind,
            subdirs: None,
            mounted_here: Arc::new(RwLock::new(vec![])),
        });

        new_parent_dentry
            .subdirs
            .as_ref()
            .unwrap()
            .write()
            .insert(new_name.to_string(), new_dentry);

        Ok(())
    }

    pub fn unlink(&mut self, path: &Path, cwd: Option<Arc<Dentry>>) -> FSBooleanResult {
        if path.is_dir() {
            return Err(PosixError::EISDIR);
        }

        let file_name = path.last_component().ok_or(PosixError::ENOENT)?;
        if file_name == "." || file_name == ".." {
            return Err(PosixError::EINVAL);
        }

        let (parent_dir_dentry, target_dentry) = self.resolve_path(path, cwd, false, false)?;
        if target_dentry.kind == DEntryKind::Directory {
            dmesgln(d!(D_WARNING "Attempted to unlink a directory path: {}", path));
            return Err(PosixError::EISDIR);
        }

        let mut parent_dir_inode = parent_dir_dentry.inode.write();
        let unlink_fn = parent_dir_inode.ops.unlink.ok_or_else(|| {
            dmesgln(d!(D_WARNING "Unlink not supported on inode {} in VFS unlink", parent_dir_inode.name));
            PosixError::ENOSYS
        })?;

        unlink_fn(&mut parent_dir_inode, file_name)?;

        let now = get_rtc_time() as SystemTime;
        parent_dir_inode.stat.mtime = now;
        parent_dir_inode.stat.ctime = now;
        let _ = Self::sync_stat(&parent_dir_inode);
        drop(parent_dir_inode);

        let mut target_inode = target_dentry.inode.write();
        target_inode.stat.ctime = now;
        let _ = Self::sync_stat(&target_inode);

        if let Some(subdirs) = &parent_dir_dentry.subdirs {
            subdirs.write().remove(file_name);
        }

        *target_dentry.parent.write() = Weak::new();

        Ok(())
    }
}
