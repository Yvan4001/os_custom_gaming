extern crate alloc;
use crate::kernel::drivers::storage::{StorageDevice, StorageManager};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use std::fs;

use super::storage::Partition;

/// Filesystem types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilesystemType {
    Unknown,
    Fat16,
    Fat32,
    Ext2,
    Iso9660,
    Ntfs,
    RamFs, // RAM-based filesystem
}

/// File types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    Special,
}

/// File attributes
#[derive(Debug, Clone, Copy)]
pub struct FileAttributes {
    pub readonly: bool,
    pub hidden: bool,
    pub system: bool,
    pub directory: bool,
    pub archive: bool,
}

/// File entry information
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
    pub attributes: FileAttributes,
    pub creation_time: u64,
    pub modification_time: u64,
    pub access_time: u64,
}

/// Inode structure for RAM filesystem
#[derive(Debug, Clone)]
struct RamInode {
    file_type: FileType,
    size: u64,
    data: Option<Vec<u8>>,                   // For regular files
    children: Option<BTreeMap<String, u64>>, // For directories: child name -> inode ID
    attributes: FileAttributes,
    creation_time: u64,
    modification_time: u64,
    access_time: u64,
}

/// RAM filesystem implementation
struct RamFilesystem {
    inodes: Vec<RamInode>,
    root_inode: u64,
    next_inode_id: u64,
}

/// Filesystem structure
pub struct Filesystem {
    name: String,
    fs_type: FilesystemType,
    device: String,
    mounted: AtomicBool,
    readonly: bool,
    ram_fs: Option<RamFilesystem>, // RAM filesystem data (only used for RamFs type)
    root_dir: Option<DirectoryHandle>,
}

/// Directory handle
pub struct DirectoryHandle {
    pub path: String,
    pub entries: Vec<FileEntry>,
    pub fs_name: String,       // Name of the filesystem this handle belongs to
    pub inode_id: Option<u64>, // Internal inode ID for RAM filesystem
}

/// File handle
pub struct FileHandle {
    pub path: String,
    pub size: u64,
    pub position: u64,
    pub readonly: bool,
    pub fs_name: String,       // Name of the filesystem this handle belongs to
    pub inode_id: Option<u64>, // Internal inode ID for RAM filesystem
    closed: bool,              // Track if the file is closed
}

/// Filesystem manager
pub struct FilesystemManager {
    filesystems: Vec<Filesystem>,
    current_directory: String,
}

// Global filesystem manager
lazy_static! {
    static ref FS_MANAGER: Mutex<FilesystemManager> = Mutex::new(FilesystemManager::new());
}

impl FileAttributes {
    pub fn new() -> Self {
        Self {
            readonly: false,
            hidden: false,
            system: false,
            directory: false,
            archive: false,
        }
    }
}

impl FileEntry {
    pub fn new(name: String, file_type: FileType, size: u64) -> Self {
        Self {
            name,
            file_type,
            size,
            attributes: FileAttributes::new(),
            creation_time: 0,
            modification_time: 0,
            access_time: 0,
        }
    }

    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory
    }
}

impl RamInode {
    fn new_directory() -> Self {
        Self {
            file_type: FileType::Directory,
            size: 0,
            data: None,
            children: Some(BTreeMap::new()),
            attributes: {
                let mut attrs = FileAttributes::new();
                attrs.directory = true;
                attrs
            },
            creation_time: get_current_time(),
            modification_time: get_current_time(),
            access_time: get_current_time(),
        }
    }

    fn new_file() -> Self {
        Self {
            file_type: FileType::Regular,
            size: 0,
            data: Some(Vec::new()),
            children: None,
            attributes: FileAttributes::new(),
            creation_time: get_current_time(),
            modification_time: get_current_time(),
            access_time: get_current_time(),
        }
    }

    fn to_file_entry(&self, name: String) -> FileEntry {
        FileEntry {
            name,
            file_type: self.file_type,
            size: self.size,
            attributes: self.attributes,
            creation_time: self.creation_time,
            modification_time: self.modification_time,
            access_time: self.access_time,
        }
    }
}

impl RamFilesystem {
    fn new() -> Self {
        let mut fs = Self {
            inodes: Vec::new(),
            root_inode: 0,
            next_inode_id: 0,
        };

        // Create root directory
        let root_inode = RamInode::new_directory();
        fs.inodes.push(root_inode);

        fs
    }

    fn allocate_inode(&mut self, inode: RamInode) -> u64 {
        let id = self.next_inode_id;
        self.next_inode_id += 1;
        self.inodes.push(inode);
        id
    }

    fn get_inode(&self, id: u64) -> Option<&RamInode> {
        self.inodes.get(id as usize)
    }

    fn get_inode_mut(&mut self, id: u64) -> Option<&mut RamInode> {
        self.inodes.get_mut(id as usize)
    }

    fn lookup_path(&self, path: &str) -> Result<u64, &'static str> {
        if path.is_empty() || path == "/" {
            return Ok(self.root_inode);
        }

        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_inode_id = self.root_inode;

        for component in components {
            let current_inode = self.get_inode(current_inode_id).ok_or("Invalid inode")?;

            if current_inode.file_type != FileType::Directory {
                return Err("Not a directory");
            }

            let children = current_inode
                .children
                .as_ref()
                .ok_or("Directory has no children")?;

            if let Some(child_id) = children.get(component) {
                current_inode_id = *child_id;
            } else {
                return Err("Path component not found");
            }
        }

        Ok(current_inode_id)
    }

    fn create_directory(&mut self, parent_id: u64, name: &str) -> Result<u64, &'static str> {
        // First check if the parent exists and is a directory
        {
            let parent = self
                .get_inode(parent_id)
                .ok_or("Parent directory not found")?;

            if parent.file_type != FileType::Directory {
                return Err("Parent is not a directory");
            }

            // Check if parent has children map
            let children = parent
                .children
                .as_ref()
                .ok_or("Parent has no children map")?;

            // Check if name already exists
            if children.contains_key(name) {
                return Err("Entry already exists");
            }
        }

        // Create new directory inode
        let dir_inode = RamInode::new_directory();
        let dir_id = self.allocate_inode(dir_inode);

        // Now get the parent again and update it
        {
            let parent = self
                .get_inode_mut(parent_id)
                .ok_or("Parent directory not found")?;

            let children = parent
                .children
                .as_mut()
                .ok_or("Parent has no children map")?;

            // Add to parent
            children.insert(name.to_string(), dir_id);
            parent.modification_time = get_current_time();
        }

        Ok(dir_id)
    }

    fn create_file(&mut self, parent_id: u64, name: &str) -> Result<u64, &'static str> {
        // First check if the parent exists and is a directory
        {
            let parent = self
                .get_inode(parent_id)
                .ok_or("Parent directory not found")?;

            if parent.file_type != FileType::Directory {
                return Err("Parent is not a directory");
            }

            let children = parent
                .children
                .as_ref()
                .ok_or("Parent has no children map")?;

            // Check if name already exists
            if children.contains_key(name) {
                return Err("Entry already exists");
            }
        }

        // Create new file inode
        let file_inode = RamInode::new_file();
        let file_id = self.allocate_inode(file_inode);

        // Now get the parent again and update it
        {
            let parent = self
                .get_inode_mut(parent_id)
                .ok_or("Parent directory not found")?;

            let children = parent
                .children
                .as_mut()
                .ok_or("Parent has no children map")?;

            // Add to parent
            children.insert(name.to_string(), file_id);
            parent.modification_time = get_current_time();
        }

        Ok(file_id)
    }

    fn read_directory(&self, dir_id: u64) -> Result<Vec<FileEntry>, &'static str> {
        let dir = self.get_inode(dir_id).ok_or("Directory not found")?;

        if dir.file_type != FileType::Directory {
            return Err("Not a directory");
        }

        let children = dir.children.as_ref().ok_or("Directory has no children")?;

        let mut entries = Vec::new();

        // Add "." and ".." entries
        entries.push(FileEntry::new(".".to_string(), FileType::Directory, 0));

        entries.push(FileEntry::new("..".to_string(), FileType::Directory, 0));

        // Add all children
        for (name, &child_id) in children {
            if let Some(child) = self.get_inode(child_id) {
                entries.push(child.to_file_entry(name.clone()));
            }
        }

        Ok(entries)
    }

    fn read_file(
        &mut self,
        file_id: u64,
        buffer: &mut [u8],
        offset: u64,
    ) -> Result<usize, &'static str> {
        let file = self.get_inode_mut(file_id).ok_or("File not found")?;

        if file.file_type != FileType::Regular {
            return Err("Not a regular file");
        }

        let data = file.data.as_ref().ok_or("File has no data buffer")?;

        // Update access time
        file.access_time = get_current_time();

        // Check if we're at EOF
        if offset >= file.size {
            return Ok(0);
        }

        // Calculate how many bytes to read
        let available = file.size - offset;
        let to_read = core::cmp::min(available as usize, buffer.len());

        // Copy data to buffer
        let start = offset as usize;
        let end = start + to_read;

        if end <= data.len() {
            buffer[..to_read].copy_from_slice(&data[start..end]);
            Ok(to_read)
        } else {
            // This should never happen if size is correct
            Err("Invalid file size")
        }
    }

    fn write_file(
        &mut self,
        file_id: u64,
        buffer: &[u8],
        offset: u64,
    ) -> Result<usize, &'static str> {
        let file = self.get_inode_mut(file_id).ok_or("File not found")?;

        if file.file_type != FileType::Regular {
            return Err("Not a regular file");
        }

        let data = file.data.as_mut().ok_or("File has no data buffer")?;

        // Ensure data buffer is large enough
        let required_size = offset as usize + buffer.len();
        if data.len() < required_size {
            data.resize(required_size, 0);
        }

        // Copy data from buffer
        let start = offset as usize;
        let end = start + buffer.len();
        data[start..end].copy_from_slice(buffer);

        // Update file size if needed
        if required_size as u64 > file.size {
            file.size = required_size as u64;
        }

        // Update modification time
        file.modification_time = get_current_time();

        Ok(buffer.len())
    }

    fn delete_entry(&mut self, parent_id: u64, name: &str) -> Result<(), &'static str> {
        // Check if parent is a directory
        let parent = self
            .get_inode_mut(parent_id)
            .ok_or("Parent directory not found")?;

        if parent.file_type != FileType::Directory {
            return Err("Parent is not a directory");
        }

        let children = parent
            .children
            .as_mut()
            .ok_or("Parent has no children map")?;

        // Check if name exists
        if !children.contains_key(name) {
            return Err("Entry does not exist");
        }

        // Remove from parent
        children.remove(name);
        parent.modification_time = get_current_time();

        // Note: We don't actually free the inode here
        // In a real filesystem, we would need to manage inode allocation/deallocation

        Ok(())
    }
}

impl Filesystem {
    pub fn new(name: String, fs_type: FilesystemType, device: String, readonly: bool) -> Self {
        let ram_fs = if fs_type == FilesystemType::RamFs {
            Some(RamFilesystem::new())
        } else {
            None
        };

        Self {
            name,
            fs_type,
            device,
            mounted: AtomicBool::new(false),
            readonly,
            ram_fs,
            root_dir: None,
        }
    }

    pub fn shutdown(&mut self) {
        self.mounted.store(false, Ordering::SeqCst);
    }

    pub fn mount(&mut self) -> Result<(), &'static str> {
        if self.mounted.load(Ordering::SeqCst) {
            return Ok(());
        }

        match self.fs_type {
            FilesystemType::RamFs => {
                // RAM filesystem is already initialized, just mark as mounted
                if self.ram_fs.is_none() {
                    self.ram_fs = Some(RamFilesystem::new());
                }

                // Create root directory handle
                let ram_fs = self.ram_fs.as_ref().unwrap();
                let entries = ram_fs.read_directory(ram_fs.root_inode)?;

                self.root_dir = Some(DirectoryHandle {
                    path: "/".to_string(),
                    entries,
                    fs_name: self.name.clone(),
                    inode_id: Some(ram_fs.root_inode),
                });
            }
            _ => {
                // For other filesystem types, we would:
                // 1. Read filesystem metadata from the device
                // 2. Set up initial directory structure
                // 3. Verify filesystem integrity

                // For now, create a dummy root directory
                let mut root = DirectoryHandle {
                    path: "/".to_string(),
                    entries: Vec::new(),
                    fs_name: self.name.clone(),
                    inode_id: None,
                };

                // Add some test entries for std mode
                #[cfg(feature = "std")]
                {
                    root.entries
                        .push(FileEntry::new("boot".to_string(), FileType::Directory, 0));

                    root.entries
                        .push(FileEntry::new("home".to_string(), FileType::Directory, 0));

                    root.entries
                        .push(FileEntry::new("system".to_string(), FileType::Directory, 0));

                    root.entries.push(FileEntry::new(
                        "kernel.bin".to_string(),
                        FileType::Regular,
                        1024 * 1024,
                    ));
                }

                self.root_dir = Some(root);
            }
        }

        self.mounted.store(true, Ordering::SeqCst);

        #[cfg(feature = "std")]
        log::info!(
            "Mounted {} filesystem from {}",
            match self.fs_type {
                FilesystemType::Unknown => "Unknown",
                FilesystemType::Fat16 => "FAT16",
                FilesystemType::Fat32 => "FAT32",
                FilesystemType::Ext2 => "EXT2",
                FilesystemType::Iso9660 => "ISO9660",
                FilesystemType::Ntfs => "NTFS",
                FilesystemType::RamFs => "RamFs",
            },
            self.device
        );

        Ok(())
    }

    pub fn unmount(&mut self) -> Result<(), &'static str> {
        if !self.mounted.load(Ordering::SeqCst) {
            return Ok(());
        }

        // In a real driver, this would:
        // 1. Flush any cached data
        // 2. Release all file handles
        // 3. Mark the filesystem as unmounted

        self.mounted.store(false, Ordering::SeqCst);
        self.root_dir = None;

        Ok(())
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted.load(Ordering::SeqCst)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> FilesystemType {
        self.fs_type
    }

    pub fn get_device(&self) -> &str {
        &self.device
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn create_directory(&mut self, path: &str) -> Result<(), &'static str> {
        if !self.mounted.load(Ordering::SeqCst) {
            return Err("Filesystem not mounted");
        }

        if self.readonly {
            return Err("Cannot create directory on readonly filesystem");
        }

        match self.fs_type {
            FilesystemType::RamFs => {
                let ram_fs = self
                    .ram_fs
                    .as_mut()
                    .ok_or("RAM filesystem not initialized")?;

                // Split path into parent directory and new directory name
                let (parent_path, name) = split_path(path)?;

                // Find parent directory
                let parent_id = ram_fs.lookup_path(parent_path)?;

                // Create new directory
                ram_fs.create_directory(parent_id, name)?;

                Ok(())
            }
            _ => Err("Directory creation not implemented for this filesystem type"),
        }
    }

    pub fn create_file(&mut self, path: &str) -> Result<(), &'static str> {
        if !self.mounted.load(Ordering::SeqCst) {
            return Err("Filesystem not mounted");
        }

        if self.readonly {
            return Err("Cannot create file on readonly filesystem");
        }

        match self.fs_type {
            FilesystemType::RamFs => {
                let ram_fs = self
                    .ram_fs
                    .as_mut()
                    .ok_or("RAM filesystem not initialized")?;

                // Split path into parent directory and file name
                let (parent_path, name) = split_path(path)?;

                // Find parent directory
                let parent_id = ram_fs.lookup_path(parent_path)?;

                // Create new file
                ram_fs.create_file(parent_id, name)?;

                Ok(())
            }
            _ => Err("File creation not implemented for this filesystem type"),
        }
    }

    pub fn open_directory(&self, path: &str) -> Result<DirectoryHandle, &'static str> {
        if !self.mounted.load(Ordering::SeqCst) {
            return Err("Filesystem not mounted");
        }

        match self.fs_type {
            FilesystemType::RamFs => {
                let ram_fs = self
                    .ram_fs
                    .as_ref()
                    .ok_or("RAM filesystem not initialized")?;

                // Find directory inode
                let dir_id = ram_fs.lookup_path(path)?;

                // Read directory entries
                let entries = ram_fs.read_directory(dir_id)?;

                Ok(DirectoryHandle {
                    path: path.to_string(),
                    entries,
                    fs_name: self.name.clone(),
                    inode_id: Some(dir_id),
                })
            }
            _ => {
                // For other filesystem types, we would traverse the directory structure
                // For now, we just return the root directory for any path
                if let Some(root) = &self.root_dir {
                    return Ok(DirectoryHandle {
                        path: path.to_string(),
                        entries: root.entries.clone(),
                        fs_name: self.name.clone(),
                        inode_id: None,
                    });
                }

                Err("Root directory not found")
            }
        }
    }

    pub fn open_file(&self, path: &str, readonly: bool) -> Result<FileHandle, &'static str> {
        if !self.mounted.load(Ordering::SeqCst) {
            return Err("Filesystem not mounted");
        }

        if !readonly && self.readonly {
            return Err("Cannot open file for writing on readonly filesystem");
        }

        match self.fs_type {
            FilesystemType::RamFs => {
                let ram_fs = self
                    .ram_fs
                    .as_ref()
                    .ok_or("RAM filesystem not initialized")?;

                // Find file inode
                let file_id = ram_fs.lookup_path(path)?;

                // Check if it's a regular file
                let file = ram_fs.get_inode(file_id).ok_or("File not found")?;

                if file.file_type != FileType::Regular {
                    return Err("Not a regular file");
                }

                Ok(FileHandle {
                    path: path.to_string(),
                    size: file.size,
                    position: 0,
                    readonly,
                    fs_name: self.name.clone(),
                    inode_id: Some(file_id),
                    closed: false,
                })
            }
            _ => {
                // For other filesystem types, create a dummy file handle
                Ok(FileHandle {
                    path: path.to_string(),
                    size: 1024,
                    position: 0,
                    readonly,
                    fs_name: self.name.clone(),
                    inode_id: None,
                    closed: false,
                })
            }
        }
    }

    pub fn delete_entry(&mut self, path: &str) -> Result<(), &'static str> {
        if !self.mounted.load(Ordering::SeqCst) {
            return Err("Filesystem not mounted");
        }

        if self.readonly {
            return Err("Cannot delete entry on readonly filesystem");
        }

        match self.fs_type {
            FilesystemType::RamFs => {
                let ram_fs = self
                    .ram_fs
                    .as_mut()
                    .ok_or("RAM filesystem not initialized")?;

                // Split path into parent directory and entry name
                let (parent_path, name) = split_path(path)?;

                // Find parent directory
                let parent_id = ram_fs.lookup_path(parent_path)?;

                // Delete entry
                ram_fs.delete_entry(parent_id, name)?;

                Ok(())
            }
            _ => Err("Delete operation not implemented for this filesystem type"),
        }
    }
}

impl DirectoryHandle {
    pub fn read_entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn refresh(&mut self, fs_manager: &FilesystemManager) -> Result<(), &'static str> {
        if let Some(fs) = fs_manager.get_filesystem(&self.fs_name) {
            let new_dir = fs.open_directory(&self.path)?;
            self.entries = new_dir.entries;
            self.inode_id = new_dir.inode_id;
            Ok(())
        } else {
            Err("Filesystem not found")
        }
    }
}

impl FileHandle {
    pub fn new(path: String, size: u64, readonly: bool, fs_name: String) -> Self {
        Self {
            path,
            size,
            position: 0,
            readonly,
            fs_name,
            inode_id: None,
            closed: false,
        }
    }
    pub fn read(
        &mut self,
        buffer: &mut [u8],
        fs_manager: &FilesystemManager,
    ) -> Result<usize, &'static str> {
        if let Some(fs) = fs_manager.get_filesystem(&self.fs_name) {
            match fs.fs_type {
                FilesystemType::RamFs => {
                    if let Some(inode_id) = self.inode_id {
                        if let Some(ram_fs) = fs.ram_fs.as_ref() {
                            // We need to cheat a bit here since we need a mutable reference
                            // to the RAM filesystem to update access times
                            let ram_fs_ptr = ram_fs as *const RamFilesystem as *mut RamFilesystem;
                            let ram_fs_mut = unsafe { &mut *ram_fs_ptr };

                            let bytes_read =
                                ram_fs_mut.read_file(inode_id, buffer, self.position)?;
                            self.position += bytes_read as u64;
                            return Ok(bytes_read);
                        }
                    }
                    Err("Invalid file handle")
                }
                _ => {
                    // For other filesystem types, just fill with test data
                    let to_read = buffer.len().min((self.size - self.position) as usize);
                    if to_read == 0 {
                        return Ok(0); // EOF
                    }

                    for i in 0..to_read {
                        buffer[i] = ((self.position as u8) + (i as u8)) % 256;
                    }

                    self.position += to_read as u64;
                    Ok(to_read)
                }
            }
        } else {
            Err("Filesystem not found")
        }
    }

    pub fn write(
        &mut self,
        buffer: &[u8],
        fs_manager: &FilesystemManager,
    ) -> Result<usize, &'static str> {
        if self.readonly {
            return Err("File is readonly");
        }

        if let Some(fs) = fs_manager.get_filesystem(&self.fs_name) {
            if fs.readonly {
                return Err("Filesystem is readonly");
            }

            match fs.fs_type {
                FilesystemType::RamFs => {
                    if let Some(inode_id) = self.inode_id {
                        if let Some(ram_fs) = fs.ram_fs.as_ref() {
                            // We need to cheat a bit here since we need a mutable reference
                            let ram_fs_ptr = ram_fs as *const RamFilesystem as *mut RamFilesystem;
                            let ram_fs_mut = unsafe { &mut *ram_fs_ptr };

                            let bytes_written =
                                ram_fs_mut.write_file(inode_id, buffer, self.position)?;
                            self.position += bytes_written as u64;

                            // Update file size
                            if self.position > self.size {
                                self.size = self.position;
                            }

                            return Ok(bytes_written);
                        }
                    }
                    Err("Invalid file handle")
                }
                _ => {
                    // For other filesystem types, just simulate a write
                    let to_write = buffer.len();
                    self.position += to_write as u64;
                    if self.position > self.size {
                        self.size = self.position;
                    }

                    Ok(to_write)
                }
            }
        } else {
            Err("Filesystem not found")
        }
    }

    pub fn seek(&mut self, position: u64) -> Result<(), &'static str> {
        if position > self.size {
            return Err("Seek position beyond file size");
        }

        self.position = position;
        Ok(())
    }

    pub fn get_size(&self) -> u64 {
        self.size
    }

    pub fn get_position(&self) -> u64 {
        self.position
    }

    pub fn close(&mut self, fs_manager: &FilesystemManager) -> Result<(), &'static str> {
        if self.closed {
            return Ok(()); // Already closed, nothing to do
        }

        // Update file metadata if the file was open for writing
        if !self.readonly {
            if let Some(fs) = fs_manager.get_filesystem(&self.fs_name) {
                match fs.fs_type {
                    FilesystemType::RamFs => {
                        if let Some(inode_id) = self.inode_id {
                            if let Some(ram_fs) = fs.ram_fs.as_ref() {
                                // We need a mutable reference to update metadata
                                let ram_fs_ptr =
                                    ram_fs as *const RamFilesystem as *mut RamFilesystem;
                                let ram_fs_mut = unsafe { &mut *ram_fs_ptr };

                                // Get the file inode and update its modification time
                                if let Some(file) = ram_fs_mut.get_inode_mut(inode_id) {
                                    file.modification_time = get_current_time();

                                    // If there's any buffered data that needs to be written, this
                                    // would be the place to do it, but our implementation writes
                                    // directly to the in-memory data structures
                                }
                            }
                        }
                    }
                    _ => {
                        // For other filesystem types, commit any cached changes
                        #[cfg(feature = "std")]
                        log::debug!("Closing file: {}", self.path);
                    }
                }
            }
        }

        // Mark the file as closed
        self.closed = true;

        #[cfg(feature = "std")]
        log::debug!("File closed: {}", self.path);

        Ok(())
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        if !self.closed {
            // We can't access fs_manager here, so just update the internal state
            #[cfg(feature = "std")]
            log::warn!(
                "File handle dropped without being closed properly: {}",
                self.path
            );

            // Mark as closed to prevent further operations
            self.closed = true;
        }
    }
}

impl FilesystemManager {
    pub fn new() -> Self {
        Self {
            filesystems: Vec::new(),
            current_directory: "/".to_string(),
        }
    }
    pub fn init() {
        // Initialize the filesystem manager
        let mut fs_manager = FS_MANAGER.lock();
        *fs_manager = FilesystemManager::new();
    }
    pub fn detect_filesystem_type(
        &self,
        storage_manager: &StorageManager,
        device_name: &str,
    ) -> Result<FilesystemType, &'static str> {
        let device = storage_manager
            .get_device(device_name)
            .ok_or("Device not found")?;

        // Read first sector (usually contains filesystem metadata)
        let mut buffer = vec![0u8; device.get_sector_size() as usize];
        device.read_sectors(0, 1, &mut buffer)?;

        // Check for FAT16
        if &buffer[54..58] == b"FAT1" {
            return Ok(FilesystemType::Fat16);
        }

        // Check for FAT32
        if &buffer[82..90] == b"FAT32   " {
            return Ok(FilesystemType::Fat32);
        }

        // Check for Ext2 (skip since we don't have access to superblock easily)

        // Check for NTFS
        if &buffer[3..7] == b"NTFS" {
            return Ok(FilesystemType::Ntfs);
        }

        // Check for ISO9660
        if &buffer[1..6] == b"CD001" {
            return Ok(FilesystemType::Iso9660);
        }

        Ok(FilesystemType::Unknown)
    }

    // Mount a filesystem from a partition
    pub fn mount_partition(
        &mut self,
        storage_manager: &StorageManager,
        partition: &Partition,
        mount_point: &str,
    ) -> Result<(), &'static str> {
        // Read first few sectors to detect filesystem type
        let mut buffer = vec![0u8; 4096]; // Large enough for filesystem headers
        storage_manager.read_partition(partition, 0, 8, &mut buffer)?;

        // Detect filesystem type from the data
        let fs_type = self.detect_filesystem_type_from_data(&buffer)?;

        // Create appropriate filesystem handler
        let fs_name = format!("{}:{}", partition.get_device_name(), mount_point);
        let fs = Filesystem::new(
            fs_name,
            fs_type,
            partition.get_device_name().to_string(),
            false, // Not read-only by default
        );

        // Add and mount the filesystem
        self.add_filesystem(fs)?;

        // Associate with mount point
        // In a real implementation, you'd maintain a mount points map

        Ok(())
    }

    fn detect_filesystem_type_from_data(
        &self,
        data: &[u8],
    ) -> Result<FilesystemType, &'static str> {
        // Implementation similar to detect_filesystem_type but works on raw data
        // This avoids duplicate reads

        // For now, return a default
        Ok(FilesystemType::Unknown)
    }

    pub fn add_filesystem(&mut self, mut fs: Filesystem) -> Result<(), &'static str> {
        // Mount the filesystem
        fs.mount()?;

        // Add to our list
        self.filesystems.push(fs);
        Ok(())
    }

    pub fn get_filesystem(&self, name: &str) -> Option<&Filesystem> {
        self.filesystems.iter().find(|fs| fs.get_name() == name)
    }

    pub fn get_filesystem_mut(&mut self, name: &str) -> Option<&mut Filesystem> {
        self.filesystems.iter_mut().find(|fs| fs.get_name() == name)
    }

    pub fn get_filesystems(&self) -> &[Filesystem] {
        &self.filesystems
    }

    pub fn set_current_directory(&mut self, path: String) {
        self.current_directory = path;
    }

    pub fn get_current_directory(&self) -> &str {
        &self.current_directory
    }

    pub fn create_directory(&mut self, path: &str) -> Result<(), &'static str> {
        // Find the appropriate filesystem
        // For now, we just use the first mounted filesystem
        if let Some(fs) = self.filesystems.iter_mut().find(|fs| fs.is_mounted()) {
            return fs.create_directory(path);
        }

        Err("No mounted filesystem found")
    }

    pub fn create_file(&mut self, path: &str) -> Result<(), &'static str> {
        // Find the appropriate filesystem
        // For now, we just use the first mounted filesystem
        if let Some(fs) = self.filesystems.iter_mut().find(|fs| fs.is_mounted()) {
            return fs.create_file(path);
        }

        Err("No mounted filesystem found")
    }

    pub fn open_file(&self, path: &str, readonly: bool) -> Result<FileHandle, &'static str> {
        // Find the appropriate filesystem
        // For now, we just use the first mounted filesystem
        if let Some(fs) = self.filesystems.iter().find(|fs| fs.is_mounted()) {
            return fs.open_file(path, readonly);
        }

        Err("No mounted filesystem found")
    }

    pub fn open_directory(&self, path: &str) -> Result<DirectoryHandle, &'static str> {
        // Find the appropriate filesystem
        // For now, we just use the first mounted filesystem
        if let Some(fs) = self.filesystems.iter().find(|fs| fs.is_mounted()) {
            return fs.open_directory(path);
        }

        Err("No mounted filesystem found")
    }

    pub fn delete_entry(&mut self, path: &str) -> Result<(), &'static str> {
        // Find the appropriate filesystem
        // For now, we just use the first mounted filesystem
        if let Some(fs) = self.filesystems.iter_mut().find(|fs| fs.is_mounted()) {
            return fs.delete_entry(path);
        }

        Err("No mounted filesystem found")
    }
}

const DIRECTORY_LIST: [&str; 41] = [
    "/bin", "/etc", "/home", "/tmp", "/var", "/boot", "/lib", "/lib64", "/opt", "/srv", "/mnt",
    "/media", "/dev", "/proc", "/sys", "/run", "/usr", "/root", "/sysroot", "/tmpfs", "/sysfs", "/devfs", "/procfs", "/netfs",
    "/cgroup", "/debugfs", "/devpts", "/hugetlbfs", "/mqueue", "/pstore", "/tracefs", "/configfs",
    "/securityfs", "/fusectl", "/selinuxfs", "/sys/kernel/debug", "/sys/kernel/security",
    "/sys/kernel/tracing", "/sys/kernel/cgroup", "/sys/kernel/hugepages", "/sys/kernel/mqueue",
];

/// Initialize the filesystem subsystem
pub fn init(storage_manager: &StorageManager) -> Result<(), &'static str> {
    let mut fs_manager = FilesystemManager::new();

    // In a real OS, we would:
    // 1. Detect filesystem types on storage devices
    // 2. Mount root filesystem
    // 3. Set up initial directory structure

    // Create a RAM filesystem for temporary storage
    let ramfs = Filesystem::new(
        "ramfs".to_string(),
        FilesystemType::RamFs,
        "ram".to_string(),
        false,
    );

    fs_manager.add_filesystem(ramfs)?;

    #[cfg(feature = "std")]
    {
        // For testing, create a virtual filesystem
        if let Some(device) = storage_manager.get_device("sda") {
            let fs = Filesystem::new(
                "root".to_string(),
                FilesystemType::Ext2,
                "sda".to_string(),
                false,
            );

            fs_manager.add_filesystem(fs)?;
        }

        log::info!(
            "Filesystem initialized with {} filesystems",
            fs_manager.get_filesystems().len()
        );

        // Create some test directories and files in RAM filesystem
        for dir in DIRECTORY_LIST {
            fs_manager.create_directory(dir)?;
        }

        // Create a test file
        fs_manager.create_file("/hello.txt")?;
        let mut file = fs_manager.open_file("/hello.txt", false)?;
        let data = b"Hello, world from RAM filesystem!";
        file.write(data, &fs_manager)?;
    }

    // Store the manager in the global instance
    *FS_MANAGER.lock() = fs_manager;

    Ok(())
}

/// Get the filesystem manager
pub fn get_fs_manager() -> &'static Mutex<FilesystemManager> {
    &FS_MANAGER
}

// Helper functions

/// Get current system time (simplified)
fn get_current_time() -> u64 {
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    #[cfg(not(feature = "std"))]
    {
        // In a real OS, you would read from RTC or system timer
        0
    }
}

/// Split a path into parent directory and file/dir name
fn split_path(path: &str) -> Result<(&str, &str), &'static str> {
    let path = path.trim_end_matches('/');

    if path.is_empty() {
        return Err("Empty path");
    }

    if let Some(last_slash) = path.rfind('/') {
        let parent = if last_slash == 0 {
            "/"
        } else {
            &path[..last_slash]
        };
        let name = &path[last_slash + 1..];

        if name.is_empty() {
            return Err("Invalid path");
        }

        Ok((parent, name))
    } else {
        // No slash, so it's in the root directory
        Ok(("/", path))
    }
}

pub fn shutdown() {
    // Get a mutable reference to the filesystem manager and shutdown all mounted filesystems
    if let Some(mut fs_manager) = FS_MANAGER.try_lock() {
        for fs in fs_manager.filesystems.iter_mut() {
            fs.shutdown();
        }
    }
}