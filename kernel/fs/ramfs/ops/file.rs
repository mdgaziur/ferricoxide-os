use crate::fs::ramfs::{RamFSInodeDowncastRef, RamFSInodeKind};
use crate::fs::{FSResult, Inode};
use crate::posix::PosixError;

pub fn ramfs_read(inode: &Inode, buf: &mut [u8], bytes: u64, offset: u64) -> FSResult<u64> {
    let inode_inner: RamFSInodeDowncastRef = inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_read");

    match &inode_inner.read().inode_kind {
        RamFSInodeKind::File { data } => {
            let file_size = data.len() as u64;

            if offset >= file_size {
                return Ok(0);
            }

            let available_bytes = (file_size - offset) as usize;
            let read_bytes = available_bytes.min(bytes as usize).min(buf.len());

            if read_bytes > 0 {
                let start = offset as usize;
                let end = start + read_bytes;
                buf[..read_bytes].copy_from_slice(&data[start..end]);
            }

            Ok(read_bytes as u64)
        }
        RamFSInodeKind::Directory { .. } => Err(PosixError::EISDIR),
        RamFSInodeKind::Symlink { .. } => Err(PosixError::EINVAL),
    }
}

pub fn ramfs_write(inode: &Inode, offset: u64, bytes: u64, buf: &[u8]) -> FSResult<u64> {
    let inode_inner: RamFSInodeDowncastRef = inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_write");

    match &mut inode_inner.write().inode_kind {
        RamFSInodeKind::File { data } => {
            let write_bytes = bytes.min(buf.len() as u64);

            if write_bytes == 0 {
                return Ok(0);
            }

            let start = usize::try_from(offset).map_err(|_| PosixError::E2BIG)?;
            let count = usize::try_from(write_bytes).map_err(|_| PosixError::E2BIG)?;
            let end = start.checked_add(count).ok_or(PosixError::E2BIG)?;

            // POSIX: writing past the current EOF causes the file to grow (zero-filling any sparse gap)
            if end > data.len() {
                data.try_reserve(end - data.len())
                    .map_err(|_| PosixError::ENOSPC)?;
                data.resize(end, 0);
            }

            data[start..end].copy_from_slice(&buf[..count]);

            Ok(write_bytes)
        }
        RamFSInodeKind::Directory { .. } => Err(PosixError::EISDIR),
        RamFSInodeKind::Symlink { .. } => Err(PosixError::EINVAL),
    }
}
pub fn ramfs_truncate(inode: &mut Inode, size: u64) -> FSResult<()> {
    let inode_inner: RamFSInodeDowncastRef = inode
        .inner_opaque
        .downcast_ref()
        .expect("Failed to downcast Inode inner_opaque to RamFSInode in ramfs_truncate");

    match &mut inode_inner.write().inode_kind {
        RamFSInodeKind::File { data } => {
            let target_len = usize::try_from(size).map_err(|_| PosixError::EFBIG)?;

            if target_len > data.len() {
                data.try_reserve(target_len - data.len())
                    .map_err(|_| PosixError::ENOSPC)?;
            }

            data.resize(target_len, 0);

            if target_len == 0 {
                data.shrink_to_fit();
            }

            Ok(())
        }
        RamFSInodeKind::Directory { .. } => Err(PosixError::EISDIR),
        RamFSInodeKind::Symlink { .. } => Err(PosixError::EINVAL),
    }
}
