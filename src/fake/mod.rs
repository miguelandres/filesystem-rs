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
use std::ffi::{OsStr, OsString};
use std::io::Result;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::vec::IntoIter;

use FileSystem;
#[cfg(unix)]
use UnixFileSystem;
#[cfg(feature = "temp")]
use {TempDir, TempFileSystem};

#[cfg(feature = "temp")]
pub use self::tempdir::FakeTempDir;

use self::registry::Registry;

mod node;
mod registry;
#[cfg(feature = "temp")]
mod tempdir;

/// An in-memory file system.
#[derive(Clone, Debug, Default)]
pub struct FakeFileSystem {
    registry: Arc<Mutex<Registry>>,
}
impl FakeFileSystem {
    pub fn new() -> Self {
        let registry = Registry::new();

        FakeFileSystem {
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    fn apply<F, T>(&self, path: &Path, f: F) -> T
    where
        F: FnOnce(&MutexGuard<Registry>, &Path) -> T,
    {
        let registry = self.registry.lock().unwrap();
        let storage;
        let path = if path.is_relative() {
            storage = registry
                .current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .join(path);
            &storage
        } else {
            path
        };

        f(&registry, path)
    }

    fn apply_mut<F, T>(&self, path: &Path, mut f: F) -> T
    where
        F: FnMut(&mut MutexGuard<Registry>, &Path) -> T,
    {
        let mut registry = self.registry.lock().unwrap();
        let storage;
        let path = if path.is_relative() {
            storage = registry
                .current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .join(path);
            &storage
        } else {
            path
        };

        f(&mut registry, path)
    }

    fn apply_mut_from_to<F, T>(&self, from: &Path, to: &Path, mut f: F) -> T
    where
        F: FnMut(&mut MutexGuard<Registry>, &Path, &Path) -> T,
    {
        let mut registry = self.registry.lock().unwrap();
        let from_storage;
        let from = if from.is_relative() {
            from_storage = registry
                .current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .join(from);
            &from_storage
        } else {
            from
        };
        let to_storage;
        let to = if to.is_relative() {
            to_storage = registry
                .current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .join(to);
            &to_storage
        } else {
            to
        };

        f(&mut registry, from, to)
    }
}

impl FileSystem for FakeFileSystem {
    type DirEntry = DirEntry;
    type ReadDir = ReadDir;

    fn current_dir(&self) -> Result<PathBuf> {
        let registry = self.registry.lock().unwrap();
        registry.current_dir()
    }

    fn set_current_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.set_current_dir(p.to_path_buf()))
    }

    fn is_dir<P: AsRef<Path>>(&self, path: P) -> bool {
        self.apply(path.as_ref(), |r, p| r.is_dir(p))
    }

    fn is_file<P: AsRef<Path>>(&self, path: P) -> bool {
        self.apply(path.as_ref(), |r, p| r.is_file(p))
    }

    fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.create_dir(p))
    }

    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.create_dir_all(p))
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.remove_dir(p))
    }

    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.remove_dir_all(p))
    }

    fn read_dir<P: AsRef<Path>>(&self, path: P) -> Result<Self::ReadDir> {
        let path = path.as_ref();

        self.apply(path, |r, p| r.read_dir(p)).map(|entries| {
            let entries = entries
                .iter()
                .map(|e| {
                    let file_name = e.file_name().unwrap_or_else(|| e.as_os_str());

                    Ok(DirEntry::new(path, &file_name))
                })
                .collect();

            ReadDir::new(entries)
        })
    }

    fn create_file<P, B>(&self, path: P, buf: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        self.apply_mut(path.as_ref(), |r, p| r.create_file(p, buf.as_ref()))
    }

    fn write_file<P, B>(&self, path: P, buf: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        self.apply_mut(path.as_ref(), |r, p| r.write_file(p, buf.as_ref()))
    }

    fn overwrite_file<P, B>(&self, path: P, buf: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        self.apply_mut(path.as_ref(), |r, p| r.overwrite_file(p, buf.as_ref()))
    }

    fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        self.apply(path.as_ref(), |r, p| r.read_file(p))
    }

    fn read_file_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        self.apply(path.as_ref(), |r, p| r.read_file_to_string(p))
    }

    fn read_file_into<P, B>(&self, path: P, mut buf: B) -> Result<usize>
    where
        P: AsRef<Path>,
        B: AsMut<Vec<u8>>,
    {
        self.apply(path.as_ref(), |r, p| r.read_file_into(p, buf.as_mut()))
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.remove_file(p))
    }

    fn copy_file<P, Q>(&self, from: P, to: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        self.apply_mut_from_to(from.as_ref(), to.as_ref(), |r, from, to| {
            r.copy_file(from, to)
        })
    }

    fn rename<P, Q>(&self, from: P, to: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        self.apply_mut_from_to(from.as_ref(), to.as_ref(), |r, from, to| r.rename(from, to))
    }

    fn readonly<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        self.apply(path.as_ref(), |r, p| r.readonly(p))
    }

    fn set_readonly<P: AsRef<Path>>(&self, path: P, readonly: bool) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.set_readonly(p, readonly))
    }

    fn len<P: AsRef<Path>>(&self, path: P) -> u64 {
        self.apply(path.as_ref(), |r, p| r.len(p))
    }
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    parent: PathBuf,
    file_name: OsString,
}

impl DirEntry {
    fn new<P, S>(parent: P, file_name: S) -> Self
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        DirEntry {
            parent: parent.as_ref().to_path_buf(),
            file_name: file_name.as_ref().to_os_string(),
        }
    }
}

impl crate::DirEntry for DirEntry {
    fn file_name(&self) -> OsString {
        self.file_name.clone()
    }

    fn path(&self) -> PathBuf {
        self.parent.join(&self.file_name)
    }
}

#[derive(Debug)]
pub struct ReadDir(IntoIter<Result<DirEntry>>);

impl ReadDir {
    fn new(entries: Vec<Result<DirEntry>>) -> Self {
        ReadDir(entries.into_iter())
    }
}

impl Iterator for ReadDir {
    type Item = Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl crate::ReadDir<DirEntry> for ReadDir {}

#[cfg(unix)]
impl UnixFileSystem for FakeFileSystem {
    fn mode<P: AsRef<Path>>(&self, path: P) -> Result<u32> {
        self.apply(path.as_ref(), |r, p| r.mode(p))
    }

    fn set_mode<P: AsRef<Path>>(&self, path: P, mode: u32) -> Result<()> {
        self.apply_mut(path.as_ref(), |r, p| r.set_mode(p, mode))
    }

    fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(&self, src: P, dst: Q) -> Result<()> {
        self.apply_mut_from_to(src.as_ref(), dst.as_ref(), |r, src, dst| {
            r.symlink(src, dst)
        })
    }

    fn get_symlink_src<P: AsRef<Path>>(&self, dst: P) -> Result<PathBuf> {
        return self.apply(dst.as_ref(), |r, p| r.read_link(p));
    }
}

#[cfg(feature = "temp")]
impl TempFileSystem for FakeFileSystem {
    type TempDir = FakeTempDir;

    fn temp_dir<S: AsRef<str>>(&self, prefix: S) -> Result<Self::TempDir> {
        let base = env::temp_dir();
        let dir = FakeTempDir::new(Arc::downgrade(&self.registry), &base, prefix.as_ref());

        self.create_dir_all(&dir.path()).and(Ok(dir))
    }
}
