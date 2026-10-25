//! # Filesystem Architecture of FerricOxide OS
//!
//! This module provides the entry-point and core abstractions for the filesystem subsystem in
//! FerricOxide OS.
//!
//! The filesystem implementation aims to be POSIX-compliant; any non-standard behavior that is not
//! a strict superset of POSIX specifications is considered a bug.
//!
//! ## Overview & Layered Design
//!
//! FerricOxide OS employs an abstraction layer inspired by Unix Virtual Filesystem (VFS) architectures,
//! cleanly separating generic filesystem interfaces and pathname resolution from specific filesystem
//! backend implementations:
//!
//! ```text
//! +-------------------------------------------------------+
//! |                    User / Kernel Syscalls             |
//! |          (open, read, write, close, mkdir, rmdir)     |
//! +-------------------------------------------------------+
//!                            |
//!                            v
//! +--------------------------------------------------------+
//! |              Virtual Filesystem (VFS)                  |
//! |  - Global mount table (`active_mounts`)                |
//! |  - Path traversal & canonicalization (`resolve_path`)  |
//! |  - Dentry cache hierarchy & mount point traversal      |
//! |  - File handle & position management (`FileHandle`)    |
//! +--------------------------------------------------------+
//!             |                                |
//!             v                                v
//! +-----------------------+        +----------------------+
//! | VFS Inode Abstraction |        | SuperBlock & Dentry  |
//! | - Dispatch table:     |        | - Root dentry        |
//! |   `FSOperations`      |        | - Subdirectory cache |
//! | - `inner_opaque` data |        | - Mount point stack  |
//! +-----------------------+        +----------------------+
//!             |                                |
//!             +---------------+----------------+
//!                             |
//!                             v
//!          +--------------------------------------+
//!          | Specific Filesystem Implementations  |
//!          |   e.g. `RamFS` (ferricoxide-ramfs)   |
//!          +--------------------------------------+
//! ```
//!
//! ## Core Components
//!
//! ### 1. Virtual Filesystem Layer ([`vfs::VFS`])
//! - **Singleton instance**: Accessible via [`get_vfs()`] and initialized once by [`init()`].
//! - **Filesystem Registry**: File systems register themselves using [`FileSystem`] implementations
//!   (e.g., [`RamFS`]).
//! - **Mount Table**: Tracks active mounts using uniquely generated 64-bit mount IDs (`NEXT_MOUNT_ID`).
//!   The root filesystem (a default `RamFS` instance) is automatically mounted at `/` upon initialization.
//!   Mount points can be stacked on top of existing directories.
//! - **Path Resolution**: [`vfs::VFS::resolve_path`] handles relative and absolute path resolution,
//!   current working directory (`cwd`) context, `.` / `..` traversal (including crossing mount points
//!   upwards to host dentries), symlink resolution (bounded by `MAX_SYMLINKS` to prevent cycles), and
//!   dentry cache misses via inode lookup.
//!
//! ### 2. Dentries ([`Dentry`]) & Path Caching
//! - Dentries represent directory entries cached in memory to avoid repeated lookups.
//! - Each dentry maintains:
//!   - The entry `name` and its [`DEntryKind`] (Directory, File, Symlink, Device).
//!   - An `Arc<RwLock<Inode>>` pointing to the underlying open inode.
//!   - A weak reference to its parent dentry (`Weak<Dentry>`), preventing reference cycles.
//!   - A child cache (`subdirs`) for fast lookup of descendant dentries.
//!   - A stack of mount IDs (`mounted_here`) to transparently redirect traversal into mounted filesystems.
//! - Dentries provide cache pruning utilities like [`Dentry::flush_cache`].
//!
//! ### 3. Inodes ([`Inode`]), Reference Handling & Eviction
//! - Represents an open instance of an inode within the VFS, rather than a raw on-disk structure.
//! - **Reference Handling & Implicit `i_count`**:
//!   - Inodes are shared across the VFS wrapped in `Arc<RwLock<Inode>>`.
//!   - The strong count of the outer `Arc` functions as the Unix in-memory usage counter (`i_count`).
//!     Every component referencing the open inode—such as cached [`Dentry`] nodes, open [`FileHandle`]s,
//!     or temporary resolution frames—increments this reference count.
//!   - `super_block` is stored as a `Weak<RwLock<SuperBlock>>` to avoid circular reference cycles between
//!     the superblock/dentry tree and the inode.
//! - **Freeing & Eviction Lifecycle**:
//!   - When all in-memory references (`Arc` clones) are dropped and the `Arc` strong count reaches zero,
//!     Rust invokes [`Inode::drop`].
//!   - [`Inode::drop`] upgrades the weak reference to the [`SuperBlock`] and dispatches the inode eviction
//!     callback: [`FSOperations::evict_node`].
//!   - **Hard Link Count (`i_nlink`) vs In-Memory References (`i_count`)**:
//!     Eviction does not automatically destroy underlying filesystem data if the file is still linked on
//!     disk. The underlying filesystem driver inspects the hard link count (`i_nlink`):
//!     - If `i_nlink > 0`: The inode was merely closed/evicted from the in-memory cache, but its on-disk or
//!       in-storage representation remains intact.
//!     - If `i_nlink == 0` (e.g., after being unlinked via `unlink` or `rmdir`): All directory entries referencing
//!       it are gone, and now all in-memory users have released their handles. The filesystem frees the allocated
//!       inode number (e.g., deallocating the bit in `RamFSInodeRegistry::ino_map`), reclaims allocated blocks
//!       or data buffers, and removes the inode record from storage.
//! - Stores an opaque type-erased payload (`inner_opaque: Box<dyn Any + Send + Sync>`) allowing filesystem
//!   drivers to attach custom backend structures (such as `RamFSInode`).
//! - Dispatches operations through a function-pointer table: [`FSOperations`].
//!
//! ### 4. Operations Vector ([`FSOperations`])
//! Filesystem drivers populate function pointers to support operations appropriate to their inode types:
//! - **Directory Operations**: `lookup`, `create`, `mkdir`, `rmdir`, `readdir`, `link`, `symlink`.
//! - **File Operations**: `read`, `write`, `truncate`, `close`.
//! - **Common / Node Operations**: `stat`, `unlink`, `mv`, `readlink`, `evict_node`.
//!
//! Unsupported operations are represented as `None`, which cause the VFS to return standard POSIX
//! errors such as [`PosixError::ENOSYS`] or [`PosixError::ENOTDIR`].
//!
//! ### 5. Filesystem Driver Trait ([`FileSystem`])
//! Defines the driver contract for mounting and metadata:
//! - `name()`: Identifier for the filesystem driver (e.g. `"ferricoxide-ramfs"`).
//! - `mount()`: Creates a [`SuperBlock`] binding a block device (or memory) to a root dentry and inode.
//! - `statfs()`, `sync()`, `root()`: Query and synchronize filesystem state.
//!
//! ### 6. File Handles ([`FileHandle`])
//! - Returned by [`vfs::VFS::open`] when a file or directory is opened.
//! - Encapsulates the target `Arc<RwLock<Inode>>`, open flags ([`OpenFlags`]), byte cursor position,
//!   and allocated file descriptor ID (`fd`).
//! - Tracks state across sequential `read` and `write` operations.
//!
//! ### 7. Path Abstraction ([`path::Path`])
//! - Represents normalized filesystem paths decomposed into components stored in a `VecDeque<String>`.
//! - Distinguishes between absolute and relative paths, as well as directory targets (trailing `/`).
//! - Provides path manipulation helpers (`strip_prefix`, `relative_to`, push/pop components).
//!
//! ## Built-in Implementations
//!
//! - **[`ramfs::RamFS`]**: An in-memory, volatile filesystem initialized as root (`/`). It tracks inodes
//!   via an allocation bitmap, stores directory entries in hash maps, keeps file data in memory buffers,
//!   and supports creation, lookup, reading, writing, directories, and unlinking.
//!
//! ## Frequently Asked Questions (FAQs)
//!
//! ### What happens if a file is open by one program but deleted/unlinked by another?
//! In accordance with POSIX semantics:
//! 1. **Unlinking removes directory visibility**: When program B unlinks the file, the directory entry
//!    mapping `filename -> inode_number` is removed from the parent directory's child map, and the on-disk/in-memory
//!    link counter (`i_nlink`) is decremented by 1. New path lookups for this filename will fail with
//!    [`PosixError::ENOENT`].
//! 2. **Active handles keep the file alive**: Program A still holds a [`FileHandle`], which contains an
//!    `Arc<RwLock<Inode>>` pointing to the open inode. The in-memory reference count (`i_count`) is greater than zero,
//!    meaning the `Inode` is NOT dropped.
//! 3. **I/O remains completely functional**: Program A can continue to seek, read, and write using its existing file
//!    handle as if nothing happened.
//! 4. **Deferred resource reclamation**: Once program A closes its handle (or exits) and all other references
//!    (such as cached dentries) are released, the `Arc<RwLock<Inode>>` strong count reaches zero, triggering
//!    [`Inode::drop`]. The eviction callback ([`FSOperations::evict_node`]) checks `i_nlink`. Seeing that `i_nlink == 0`,
//!    the filesystem backend finally deallocates the inode number and frees all underlying data buffers/storage blocks.
//!
//! ### Why does `Inode::drop` calling `evict_node` not always delete the inode from disk/storage?
//! There is a fundamental distinction between an **in-memory VFS inode representation** and an **on-disk/in-storage inode**:
//!
//! 1. **What `Inode::drop` means**:
//!    The [`Inode`] struct in memory represents an *open instance* of an inode tracked by the VFS. When its
//!    outer `Arc` reference count reaches zero, it indicates that **no in-memory kernel components are currently using it**
//!    (all file handles have been closed and the dentry cache entry is no longer held in active use).
//! 2. **Cache Eviction vs. Permanent Deletion**:
//!    Calling `evict_node` is a notification to the filesystem driver that this in-memory inode instance is being discarded.
//!    However, whether the file should actually be deleted from persistent storage depends entirely on its hard link count:
//!    ```text
//!    Inode::drop() (in-memory refcount / i_count reaches 0)
//!          |
//!          v
//!    ops.evict_node(inode)
//!          |
//!          +---> If i_nlink > 0:
//!          |     File is STILL LINKED in one or more directories.
//!          |     Actions: Flush any dirty metadata/buffers to disk,
//!          |     release in-memory driver caches. DO NOT DELETE on-disk inode.
//!          |
//!          +---> If i_nlink == 0:
//!                File has been UNLINKED (rm / unlink) AND has no remaining open handles.
//!                Actions: Reclaim disk blocks, free the inode number on disk/bitmap,
//!                erase metadata. PERMANENTLY PURGE from storage.
//!    ```
//! 3. **Example (Ordinary File Close)**:
//!    When a process opens `/etc/motd`, reads it, and closes the file descriptor, the in-memory `Inode` will eventually
//!    be dropped if not kept in the dcache. `Inode::drop` calls `evict_node`. Because `/etc/motd` has `i_nlink == 1`,
//!    `evict_node` merely evicts the cached data from memory—the file remains safely on disk.
//! 4. **Example (Unlinked but Open File)**:
//!    If a process creates a temporary file and immediately unlinks it (`i_nlink = 0`), the directory entry disappears,
//!    but the process continues reading and writing to its open `FileHandle` (`i_count > 0`). When the process terminates
//!    or closes the descriptor, `i_count` drops to 0, triggering `Inode::drop`. Only *then* does `evict_node` find `i_nlink == 0`
//!    and finally delete the inode and free its disk/memory blocks.
//!
//! ### What happens if a directory is mounted over an existing directory with files?
//! In FerricOxide OS, each [`Dentry`] maintains a `mounted_here` stack of mount IDs:
//! - When a filesystem is mounted on a directory, its root dentry is pushed onto `mounted_here`.
//! - During path resolution ([`vfs::VFS::resolve_path`]), the VFS transparently detects active mount IDs and redirects
//!   traversal into the root dentry of the mounted superblock.
//! - The original directory's files are shadowed (hidden) until the filesystem is unmounted, but remain unharmed on the
//!   underlying filesystem.
//!
//! ### Why does `Arc<RwLock<Inode>>` not drop to zero immediately when a file is closed, and is this an issue?
//! When a process closes a file descriptor, its [`FileHandle`] is dropped, decrementing the `Arc` reference count.
//! However, the in-memory reference count usually **does not drop to zero**:
//! 1. **Dentry Cache Retention**:
//!    The [`Dentry`] in the directory cache still holds a strong reference (`Dentry.inode = Arc<RwLock<Inode>>`),
//!    and the parent directory dentry holds a strong reference to that child in its `subdirs` map. Thus, as long as
//!    the path remains cached, the `Arc` strong count remains at least 1.
//! 2. **Primary Purpose of the Runtime Reference Count**:
//!    This behavior is intentional and matches how modern VFS architectures (e.g. Linux) operate. The runtime
//!    reference count's primary role is **life-cycle safety**: it guarantees that an active [`FileHandle`] never points
//!    to a dangling inode (preventing use-after-free) and ensures that if a file is deleted (`unlink`), its on-disk/in-memory
//!    data is kept alive until every open handle is closed.
//! 3. **When Does it Actually Drop to Zero?**:
//!    - **On Deletion**: When an entry is unlinked or removed via `rmdir`, the dentry is removed from the parent's
//!      `subdirs` map. Once all active file handles close, the refcount drops to 0, triggering [`Inode::drop`].
//!    - **On Cache Pruning**: If the dentry is evicted from memory (e.g. via [`Dentry::flush_cache`]), the cached
//!      `Arc<RwLock<Inode>>` is dropped. If no open file handles remain, the inode is dropped as well.
//!
//! ### How are circular directory hierarchies avoided?
//! 1. **Weak Parent References**: Child dentries point to their parent via `Weak<Dentry>`, while parents own strong
//!    `Arc<Dentry>` handles to their cached children in `subdirs`. This prevents memory leaks caused by reference cycles.
//! 2. **Symlink Loop Bounding**: Symlinks encountered during path resolution are tracked and capped at
//!    `MAX_SYMLINKS`. Exceeding this threshold aborts with [`PosixError::ELOOP`].
//! 3. **Directory Hard Link Restrictions**: Inodes for directories can only be linked via specific operations
//!    (`mkdir`), maintaining a strict tree/DAG structure.
//!
use crate::dbg::{D_INFO, dmesgln};
use crate::fs::ramfs::{InodeNumber, RamFS};
use crate::fs::vfs::VFS;
use crate::io::blockdev::BlockDevice;
use crate::posix::PosixError;
use crate::time::SystemTime;
use crate::{d, verify_called_once};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::any::Any;
use hashbrown::HashMap;
use spin::RwLock;

pub mod path;
pub mod ramfs;
pub mod vfs;

pub struct StatFS {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub block_size: u64,
    pub max_filename_length: u64,
}

/// The `Inode` struct represents an open inode in the VFS. It contains necessary metadata about the
/// inode, as well as a reference to the superblock it belongs to and the operations that the underlying
/// FS supports in case of this particular instance of the inode. The `inner_opaque` field is used by
/// the underlying  FS to store any additional data it needs to manage the inode, and is opaque to the VFS layer.
///
/// Important thing to keep in mind is that this *does not* represent a literal item in the file
/// system. When an operation triggers the creation of this Inode, this represents an open instance
/// of the inode. The file system is responsible for creating this. It is absolutely *important* to
/// ensure that the Inode is always wrapped in an `Arc<RwLock<>>` when being used in the VFS layer. As
/// this struct does not implement Copy or Clone, it is not possible to accidentally copy it, but it
/// is still possible to accidentally move it. The usage of the mentioned wrapper is to allow multiple places
/// to share one instance of this, where each reference made using the `Arc` acts like each increment in
/// `i_count`. Here, the usage of `Arc<RwLock<>>` allows us to make `i_count` implicit. We can evict the underlying
/// inode when the Drop implementation is called. It is the responsibility of the fs's implementation to ensure
/// that it doesn't evict the underlying inode without checking `i_nlink` first.
#[derive(Debug)]
pub struct Inode {
    pub ino: InodeNumber,
    pub typ: InodeKind,
    pub name: String,
    pub inner_opaque: Box<dyn Any + Send + Sync>,
    pub ops: FSOperations,
    pub super_block: Weak<RwLock<SuperBlock>>,
    pub stat: InodeStat,
}

impl Drop for Inode {
    fn drop(&mut self) {
        if let Some(super_block) = self.super_block.upgrade() {
            if let Some(evict_node_fn) = self.ops.evict_node {
                if let Err(e) = evict_node_fn(self) {
                    dmesgln(d!(D_INFO "Failed to evict node {}: {:?}", self.ino, e));
                } else {
                    dmesgln(d!(D_INFO "Evicted node {}", self.ino));
                }
            } else {
                dmesgln(
                    d!(D_INFO "No evict_node function for node {}, skipping eviction. Superblock: {:?}", self.ino, super_block.read()),
                );
            }
        }
    }
}

#[allow(clippy::type_complexity)]
#[derive(Debug, Copy, Clone)]
pub struct FSOperations {
    // dir functions
    pub lookup: Option<fn(&Inode, &str) -> FSResult<Arc<RwLock<Inode>>>>,
    pub create: Option<fn(&mut Inode, &str) -> FSResult<Arc<RwLock<Inode>>>>,
    pub mkdir: Option<fn(&mut Inode, &str, Mode) -> FSResult<Arc<RwLock<Inode>>>>,
    pub rmdir: Option<fn(&mut Inode, &str) -> FSBooleanResult>,
    pub link: Option<fn(&mut Inode, &str, &Inode) -> FSBooleanResult>,
    pub symlink: Option<fn(&Inode, &str, &str) -> FSResult<Arc<RwLock<Inode>>>>,
    pub readdir: Option<fn(&Inode) -> FSResult<Vec<ReaddirDirent>>>,

    // file functions
    pub close: Option<fn(&FileHandle) -> FSBooleanResult>,
    pub read: Option<fn(&Inode, &mut [u8], u64, u64) -> FSResult<u64>>,
    pub write: Option<fn(&Inode, u64, u64, &[u8]) -> FSResult<u64>>,
    pub truncate: Option<fn(&mut Inode, u64) -> FSBooleanResult>,

    // file and symlink operations
    pub unlink: Option<fn(&mut Inode, &str) -> FSBooleanResult>,
    pub mv: Option<fn(&mut Inode, &str) -> FSBooleanResult>,

    // symlink functions
    pub readlink: Option<fn(&Inode) -> FSResult<String>>,

    // common functions
    pub sync_stat: Option<fn(&Inode) -> FSBooleanResult>,
    pub evict_node: Option<fn(&Inode) -> FSBooleanResult>,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: i32 {
        const O_RDONLY   = 1 << 0;
        const O_RDWR     = 1 << 1;
        const O_SEARCH   = 1 << 2;
        const O_WRONLY   = 1 << 3;
        const O_APPEND    = 1 << 4;
        const O_CLOEXEC   = 1 << 5;
        const O_CREAT     = 1 << 6;
        const O_DIRECTORY = 1 << 7;
        const O_DSYNC     = 1 << 8;
        const O_EXCL      = 1 << 9;
        const O_NOCTTY    = 1 << 10;
        const O_NOFOLLOW  = 1 << 11;
        const O_NONBLOCK  = 1 << 12;
        const O_RSYNC     = 1 << 13;
        const O_SYNC      = 1 << 14;
        const O_TRUNC     = 1 << 15;
        const O_TTY_INIT  = 1 << 16;
        const O_EXEC     = 1 << 17;
    }
}

const S_IFMT: u32 = 0o170000; // bitmask for the file type bitfields
const S_IFBLK: u32 = 0o060000; // block device
const S_IFCHR: u32 = 0o020000; // character device
const S_IFIFO: u32 = 0o010000; // FIFO
const S_IFREG: u32 = 0o100000; // regular file
const S_IFDIR: u32 = 0o040000; // directory
const S_IFLNK: u32 = 0o120000; // symbolic link
const S_IFSOCK: u32 = 0o140000; // socket

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileMode: u32 {
        const S_IRWXU = 0o700; // read, write, execute/search by owner
        const S_IRUSR = 0o400; // read permission, owner
        const S_IWUSR = 0o200; // write permission, owner
        const S_IXUSR = 0o100; // execute/search permission, owner

        const S_IRWXG = 0o070; // read, write, execute/search by group
        const S_IRGRP = 0o040; // read permission, group
        const S_IWGRP = 0o020; // write permission, group
        const S_IXGRP = 0o010; // execute/search permission, group

        const S_IRWXO = 0o007; // read, write, execute/search by others
        const S_IROTH = 0o004; // read permission, others
        const S_IWOTH = 0o002; // write permission, others
        const S_IXOTH = 0o001; // execute/search permission, others

        const S_ISUID = 0o4000; // set-user-ID on execution
        const S_ISGID = 0o2000; // set-group-ID on execution
        const S_ISVTX = 0o1000; // sticky bit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FileType {
    Block = S_IFBLK,     // block device
    Character = S_IFCHR, // character device
    Fifo = S_IFIFO,      // FIFO
    Regular = S_IFREG,   // regular file
    Directory = S_IFDIR, // directory
    Link = S_IFLNK,      // symbolic link
    Socket = S_IFSOCK,   // socket
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Mode(pub u32);

impl Mode {
    pub fn file_type(&self) -> Option<FileType> {
        let file_type_bits = self.0 & S_IFMT;

        match file_type_bits {
            S_IFBLK => Some(FileType::Block),
            S_IFCHR => Some(FileType::Character),
            S_IFIFO => Some(FileType::Fifo),
            S_IFREG => Some(FileType::Regular),
            S_IFDIR => Some(FileType::Directory),
            S_IFLNK => Some(FileType::Link),
            S_IFSOCK => Some(FileType::Socket),
            _ => None,
        }
    }

    pub fn permissions(&self) -> FileMode {
        FileMode::from_bits_truncate(self.0)
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct InodeStat {
    pub size: u64,
    pub permissions: u16,
    pub uid: u32,
    pub gid: u32,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InodeKind {
    Directory,
    File,
    Symlink,
    CharDev,
    BlockDev,
}

#[derive(Debug, Clone)]
pub struct FileHandle {
    pub inode: Arc<RwLock<Inode>>,
    pub position: u64,
    pub flags: OpenFlags,
    pub fd: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SeekOffset {
    /// SEEK_SET: whence
    Set = 0,
    /// SEEK_CUR: cur + whence
    Cur = 1,
    /// SEEK_END: end + whence
    End = 2,
}

impl FileHandle {
    pub fn lseek(&mut self, offset: i64, whence: SeekOffset) -> FSResult<u64> {
        // TODO: fail on pipe, FIFO or socket

        fn calc_offset(base: u64, offset: i64) -> FSResult<u64> {
            if offset < 0 {
                base.checked_sub(offset.unsigned_abs())
                    .ok_or(PosixError::EINVAL)
            } else {
                base.checked_add(offset as u64).ok_or(PosixError::EOVERFLOW)
            }
        }

        // Read lock already prevents any write access
        let inode = self.inode.read();
        let new_position = match whence {
            SeekOffset::Set => {
                if offset < 0 {
                    return Err(PosixError::EINVAL);
                }
                offset as u64
            }
            SeekOffset::Cur => calc_offset(self.position, offset)?,
            SeekOffset::End => calc_offset(inode.stat.size, offset)?,
        };

        self.position = new_position;
        Ok(new_position)
    }
}

pub type FSResult<T> = Result<T, PosixError>;

type FSBooleanResult = FSResult<()>;

pub trait FileSystem: Send + Sync {
    fn name(&self) -> &'static str;

    fn requires_dev(&self) -> bool {
        true
    }

    fn mount(
        &self,
        mount_id: u64,
        device: Option<&FileHandle>,
    ) -> FSResult<Arc<RwLock<SuperBlock>>>;
}

#[derive(Debug)]
pub struct SuperBlock {
    mount_id: u64,
    magic: Option<Vec<u8>>,
    fs_specific_data: RwLock<Box<dyn Any + Send + Sync>>,
    device: Option<Arc<RwLock<BlockDevice>>>,
    root: Arc<Dentry>,
    ops: SuperBlockOps,
}

#[derive(Debug)]
pub struct SuperBlockOps {
    pub statfs: Option<fn(&SuperBlock) -> FSResult<StatFS>>,
    pub sync: Option<fn(&SuperBlock) -> FSBooleanResult>,
}

#[derive(Debug)]
pub struct Dentry {
    name: RwLock<String>,
    kind: DEntryKind,
    inode: Arc<RwLock<Inode>>,
    parent: RwLock<Weak<Dentry>>,
    subdirs: Option<RwLock<HashMap<String, Arc<Dentry>>>>,
    mounted_here: Arc<RwLock<Vec<u64>>>,
}

impl Dentry {
    pub fn flush_cache(&self) -> bool {
        if let Some(subdirs_lock) = self.subdirs.as_ref() {
            let mut subdirs = subdirs_lock.write();

            subdirs.retain(|_, child| {
                let child_has_empty_dcache = child.flush_cache();
                let child_in_use = Arc::strong_count(child) > 1;

                child_has_empty_dcache && child_in_use
            });

            subdirs.is_empty()
        } else {
            true
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum DEntryKind {
    Directory,
    File,
    Symlink,
    CharDev,
    BlockDev,
}

#[derive(Debug)]
pub struct ReaddirDirent {
    pub name: String,
    pub ino: InodeNumber,
    pub kind: DEntryKind,
}

pub fn init() {
    verify_called_once!();

    dmesgln(d!(D_INFO "Initializing VFS"));
    VFS.call_once(|| {
        let mut vfs = VFS::new();
        vfs.register_fs(Box::new(RamFS::new()));

        RwLock::new(vfs)
    });
}

pub fn get_vfs<'l>() -> &'l RwLock<VFS> {
    VFS.get().expect("VFS not initialized")
}
