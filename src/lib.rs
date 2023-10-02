//! Redox-specific system library.
#![no_std]

pub use syscall::error;
use syscall::error::{Error, Result};

pub mod flag {
    pub use libc::{O_CREAT, O_DIRECTORY, O_CLOEXEC, O_RDONLY, O_RDWR, O_WRONLY, O_ACCMODE, O_NONBLOCK};

    pub use libc::{CLOCK_MONOTONIC, CLOCK_REALTIME};

    #[cfg(target_os = "redox")]
    pub use libc::{O_SHLOCK, O_EXLOCK};

    #[cfg(target_os = "redox")]
    pub const O_STAT: i32 = syscall::flag::O_STAT as i32;
}

pub mod errno {
    pub use libc::{EPERM, EACCES, EBADF, EBADFD, ENODEV, ESRCH, EPIPE, ESPIPE, EBUSY, EINTR, EAGAIN, EWOULDBLOCK};
}
pub mod data {
    pub use libc::timespec as TimeSpec;
}

type RawResult = usize;

extern "C" {
    // NOTE: Although there are version suffixes, there'd have to be strong reasons for adding new
    // version.
    fn redox_open_v1(path_base: *const u8, path_len: usize, flags: i32, mode: u16) -> RawResult;
    fn redox_dup_v1(fd: usize, buf: *const u8, len: usize) -> RawResult;
    fn redox_dup2_v1(old_fd: usize, new_fd: usize, buf: *const u8, len: usize) -> RawResult;
    fn redox_read_v1(fd: usize, dst_base: *mut u8, dst_len: usize) -> RawResult;
    fn redox_write_v1(fd: usize, src_base: *const u8, src_len: usize) -> RawResult;
    fn redox_fsync_v1(fd: usize) -> RawResult;
    fn redox_fdatasync_v1(fd: usize) -> RawResult;
    fn redox_fchmod_v1(fd: usize, new_mode: u16) -> RawResult;
    fn redox_fchown_v1(fd: usize, new_uid: u32, new_gid: u32) -> RawResult;
    fn redox_fpath_v1(fd: usize, dst_base: *mut u8, dst_len: usize) -> RawResult;
    fn redox_close_v1(fd: usize) -> RawResult;

    fn redox_get_euid_v1() -> RawResult;
    fn redox_get_ruid_v1() -> RawResult;
    fn redox_get_egid_v1() -> RawResult;
    fn redox_get_rgid_v1() -> RawResult;
    fn redox_setrens_v1(rns: usize, ens: usize) -> RawResult;
    fn redox_waitpid_v1(pid: usize, status: *mut i32, options: i32) -> RawResult;

    fn redox_clock_gettime_v1(clock: usize, ts: *mut data::TimeSpec) -> RawResult;
}

pub struct Fd(usize);

impl Fd {
    #[inline]
    pub fn open(path: &str, flags: i32, mode: u16) -> Result<Self> {
        Ok(Self(call::open(path, flags, mode)?))
    }
    #[inline]
    pub fn dup(&self, buf: &[u8]) -> Result<usize> {
        call::dup(self.raw(), buf)
    }
    #[inline]
    pub fn dup2(&self, new_fd: usize, buf: &[u8]) -> Result<usize> {
        call::dup2(self.raw(), new_fd, buf)
    }

    #[inline]
    pub const fn raw(&self) -> usize { self.0 }

    #[inline]
    pub fn into_raw(self) -> usize {
        let raw = self.raw();
        core::mem::forget(self);
        raw
    }

    #[inline]
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        call::read(self.raw(), buf)
    }
    #[inline]
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        call::write(self.raw(), buf)
    }
    #[inline]
    pub fn fpath(&self, path: &mut [u8]) -> Result<usize> {
        call::fpath(self.raw(), path)
    }
    #[inline]
    pub fn fsync(&self) -> Result<usize> {
        call::fsync(self.raw())
    }
    #[inline]
    pub fn fdatasync(&self) -> Result<usize> {
        call::fdatasync(self.raw())
    }

    #[inline]
    pub fn chmod(&self, new_mode: u16) -> Result<()> {
        call::fchmod(self.raw(), new_mode)
    }
    #[inline]
    pub fn chown(&self, new_uid: u32, new_gid: u32) -> Result<()> {
        call::fchown(self.raw(), new_uid, new_gid)
    }
    #[inline]
    pub fn close(self) -> Result<()> {
        call::close(self.into_raw())
    }
}
impl Drop for Fd {
    fn drop(&mut self) {
        let _ = unsafe { redox_close_v1(self.0) };
    }
}

pub mod call {
    use super::*;

    /// flags and mode are binary compatible with libc
    #[inline]
    pub fn open(path: impl AsRef<str>, flags: i32, mode: u16) -> Result<usize> {
        let path = path.as_ref();
        Ok(Error::demux(unsafe { redox_open_v1(path.as_ptr(), path.len(), flags, mode) })?)
    }
    #[inline]
    pub fn dup(fd: usize, buf: impl AsRef<[u8]>) -> Result<usize> {
        let buf = buf.as_ref();
        Ok(Error::demux(unsafe { redox_dup_v1(fd, buf.as_ptr(), buf.len()) })?)
    }
    #[inline]
    pub fn dup2(old_fd: usize, new_fd: usize, buf: impl AsRef<[u8]>) -> Result<usize> {
        let buf = buf.as_ref();
        Ok(Error::demux(unsafe { redox_dup2_v1(old_fd, new_fd, buf.as_ptr(), buf.len()) })?)
    }
    #[inline]
    pub fn read(raw_fd: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(Error::demux(unsafe { redox_read_v1(raw_fd, buf.as_mut_ptr(), buf.len()) })?)
    }
    #[inline]
    pub fn write(raw_fd: usize, buf: &[u8]) -> Result<usize> {
        Ok(Error::demux(unsafe { redox_write_v1(raw_fd, buf.as_ptr(), buf.len()) })?)
    }
    #[inline]
    pub fn fsync(raw_fd: usize) -> Result<usize> {
        Ok(Error::demux(unsafe { redox_fsync_v1(raw_fd) })?)
    }
    #[inline]
    pub fn fdatasync(raw_fd: usize) -> Result<usize> {
        Ok(Error::demux(unsafe { redox_fdatasync_v1(raw_fd) })?)
    }
    #[inline]
    pub fn fchmod(raw_fd: usize, new_mode: u16) -> Result<()> {
        Error::demux(unsafe { redox_fchmod_v1(raw_fd, new_mode) })?;
        Ok(())
    }
    #[inline]
    pub fn fchown(raw_fd: usize, new_uid: u32, new_gid: u32) -> Result<()> {
        Error::demux(unsafe { redox_fchown_v1(raw_fd, new_uid, new_gid) })?;
        Ok(())
    }
    #[inline]
    pub fn fpath(raw_fd: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(Error::demux(unsafe { redox_fpath_v1(raw_fd, buf.as_mut_ptr(), buf.len()) })?)
    }
    #[inline]
    pub fn close(raw_fd: usize) -> Result<()> {
        Error::demux(unsafe { redox_close_v1(raw_fd) })?;
        Ok(())
    }

    #[inline]
    pub fn geteuid() -> Result<usize> {
        Error::demux(unsafe { redox_get_euid_v1() })
    }

    #[inline]
    pub fn getruid() -> Result<usize> {
        Error::demux(unsafe { redox_get_ruid_v1() })
    }

    #[inline]
    pub fn getegid() -> Result<usize> {
        Error::demux(unsafe { redox_get_egid_v1() })
    }
    #[inline]
    pub fn getrgid() -> Result<usize> {
        Error::demux(unsafe { redox_get_rgid_v1() })
    }
    #[inline]
    pub fn setrens(rns: usize, ens: usize) -> Result<usize> {
        Error::demux(unsafe { redox_setrens_v1(rns, ens) })
    }
    #[inline]
    pub fn waitpid(pid: usize, status: &mut i32, options: i32) -> Result<usize> {
        Error::demux(unsafe { redox_waitpid_v1(pid, status as *mut i32, options) })
    }
    #[inline]
    pub fn clock_gettime(clock: i32, ts: &mut data::TimeSpec) -> Result<usize> {
        Error::demux(unsafe { redox_clock_gettime_v1(clock as usize, ts) })
    }
}
