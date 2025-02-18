// Copyright (c) 2017 Isobel Redelmeier
// Copyright (c) 2021 Miguel Barreto
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{Read, Result, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[cfg(feature = "temp")]
use tempdir;

#[cfg(unix)]
use UnixFileSystem;
use {DirEntry, FileSystem, ReadDir};
#[cfg(feature = "temp")]
use {TempDir, TempFileSystem};

/// Tracks a temporary directory that will be deleted once the struct goes out of scope.
///
/// This is a wrapper around a [`TempDir`].
///
/// [`TempDir`]: https://doc.rust-lang.org/tempdir/tempdir/struct.TempDir.html
#[cfg(feature = "temp")]
#[derive(Debug)]
pub struct OsTempDir(tempdir::TempDir);

#[cfg(feature = "temp")]
impl TempDir for OsTempDir {
    fn path(&self) -> &Path {
        self.0.path()
    }
}

/// An implementation of `FileSystem` that interacts with the actual operating system's file system.
///
/// This is primarily a wrapper for [`fs`] methods.
///
/// [`fs`]: https://doc.rust-lang.org/std/fs/index.html
#[derive(Clone, Debug, Default)]
pub struct OsFileSystem {}

impl OsFileSystem {
    pub fn new() -> Self {
        OsFileSystem {}
    }
}

impl FileSystem for OsFileSystem {
    type DirEntry = fs::DirEntry;
    type ReadDir = fs::ReadDir;

    fn current_dir(&self) -> Result<PathBuf> {
        env::current_dir()
    }

    fn set_current_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        env::set_current_dir(path)
    }

    fn is_dir<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().is_dir()
    }

    fn is_file<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().is_file()
    }

    fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::create_dir(path)
    }

    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::remove_dir(path)
    }

    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::remove_dir_all(path)
    }

    fn read_dir<P: AsRef<Path>>(&self, path: P) -> Result<Self::ReadDir> {
        fs::read_dir(path)
    }

    fn write_file<P, B>(&self, path: P, buf: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        let mut file = File::create(path)?;
        file.write_all(buf.as_ref())
    }

    fn overwrite_file<P, B>(&self, path: P, buf: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        let mut file = OpenOptions::new().write(true).truncate(true).open(path)?;
        file.write_all(buf.as_ref())
    }

    fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        let mut contents = Vec::<u8>::new();
        let mut file = File::open(path)?;

        file.read_to_end(&mut contents)?;

        Ok(contents)
    }

    fn read_file_into<P, B>(&self, path: P, mut buf: B) -> Result<usize>
    where
        P: AsRef<Path>,
        B: AsMut<Vec<u8>>,
    {
        let mut file = File::open(path)?;
        file.read_to_end(buf.as_mut())
    }

    fn read_file_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let mut contents = String::new();
        let mut file = File::open(path)?;

        file.read_to_string(&mut contents)?;

        Ok(contents)
    }

    fn create_file<P, B>(&self, path: P, buf: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;

        file.write_all(buf.as_ref())
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::remove_file(path)
    }

    fn copy_file<P, Q>(&self, from: P, to: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        fs::copy(from, to).and(Ok(()))
    }

    fn rename<P, Q>(&self, from: P, to: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        fs::rename(from, to)
    }

    fn readonly<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        permissions(path.as_ref()).map(|p| p.readonly())
    }

    fn set_readonly<P: AsRef<Path>>(&self, path: P, readonly: bool) -> Result<()> {
        let mut permissions = permissions(path.as_ref())?;

        permissions.set_readonly(readonly);

        fs::set_permissions(path, permissions)
    }

    fn len<P: AsRef<Path>>(&self, path: P) -> u64 {
        fs::metadata(path.as_ref()).map(|md| md.len()).unwrap_or(0)
    }
}

impl DirEntry for fs::DirEntry {
    fn file_name(&self) -> OsString {
        self.file_name()
    }

    fn path(&self) -> PathBuf {
        self.path()
    }
}

impl ReadDir<fs::DirEntry> for fs::ReadDir {}

#[cfg(unix)]
impl UnixFileSystem for OsFileSystem {
    fn mode<P: AsRef<Path>>(&self, path: P) -> Result<u32> {
        permissions(path.as_ref()).map(|p| p.mode())
    }

    fn set_mode<P: AsRef<Path>>(&self, path: P, mode: u32) -> Result<()> {
        let mut permissions = permissions(path.as_ref())?;

        permissions.set_mode(mode);

        fs::set_permissions(path, permissions)
    }
    fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(&self, src: P, dst: Q) -> Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

    fn get_symlink_src<P: AsRef<Path>>(&self, dst: P) -> Result<PathBuf> {
        std::fs::read_link(dst)
    }
}

#[cfg(feature = "temp")]
impl TempFileSystem for OsFileSystem {
    type TempDir = OsTempDir;

    fn temp_dir<S: AsRef<str>>(&self, prefix: S) -> Result<Self::TempDir> {
        tempdir::TempDir::new(prefix.as_ref()).map(OsTempDir)
    }
}

fn permissions(path: &Path) -> Result<Permissions> {
    let metadata = fs::metadata(path)?;

    Ok(metadata.permissions())
}
