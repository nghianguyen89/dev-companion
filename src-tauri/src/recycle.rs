use std::path::{Component, Path, PathBuf, Prefix};

const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LinkInfo {
    pub target: PathBuf,
    pub directory: bool,
    pub tag: u32,
}

#[cfg(windows)]
mod windows_shell {
    use super::*;
    use crate::fs_safety;
    use std::{
        collections::{HashSet, VecDeque},
        os::windows::ffi::OsStrExt,
        os::windows::fs::MetadataExt,
        path::PathBuf,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Mutex,
        },
        time::SystemTime,
    };
    use windows::{
        core::{implement, Error, Ref, Result as WindowsResult, HRESULT, PCWSTR},
        Win32::{
            Foundation::{CloseHandle, E_ABORT},
            Storage::FileSystem::{
                CreateFileW, FileAttributeTagInfo, GetFileInformationByHandle,
                GetFileInformationByHandleEx, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_TAG_INFO,
                FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_BACKUP_SEMANTICS,
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
                FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            },
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                FileOperation, IFileOperation, IFileOperationProgressSink,
                IFileOperationProgressSink_Impl, IShellItem, SHCreateItemFromParsingName,
                FILEOPERATION_FLAGS, FOFX_EARLYFAILURE, FOFX_RECYCLEONDELETE, FOF_NOCONFIRMATION,
                FOF_NOERRORUI, FOF_NO_CONNECTED_ELEMENTS, FOF_SILENT,
                TSF_DELETE_RECYCLE_IF_POSSIBLE,
            },
        },
    };

    const MAX_STAGE_ENTRIES: usize = 100_000;
    const MAX_STAGE_DEPTH: usize = 64;
    static NEXT_STAGE: AtomicU64 = AtomicU64::new(1);

    struct ComApartment;

    impl ComApartment {
        fn initialize() -> Result<Self, String> {
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                .ok()
                .map_err(|error| {
                    format!("Cannot initialize the Windows Shell apartment: {error}")
                })?;
            Ok(Self)
        }
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    #[derive(Default)]
    struct SinkState {
        pre_delete_seen: bool,
        recycled_item_confirmed: bool,
        error: Option<String>,
    }

    #[implement(IFileOperationProgressSink)]
    struct RecycleSink {
        state: Arc<Mutex<SinkState>>,
    }

    impl RecycleSink {
        fn reject(message: &str) -> WindowsResult<()> {
            Err(Error::new(E_ABORT, message))
        }
    }

    #[allow(non_snake_case)]
    impl IFileOperationProgressSink_Impl for RecycleSink_Impl {
        fn StartOperations(&self) -> WindowsResult<()> {
            Ok(())
        }
        fn FinishOperations(&self, _result: HRESULT) -> WindowsResult<()> {
            Ok(())
        }
        fn PreRenameItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            _name: &PCWSTR,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn PostRenameItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            _name: &PCWSTR,
            _result: HRESULT,
            _new_item: Ref<'_, IShellItem>,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn PreMoveItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            _destination: Ref<'_, IShellItem>,
            _name: &PCWSTR,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn PostMoveItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            _destination: Ref<'_, IShellItem>,
            _name: &PCWSTR,
            _result: HRESULT,
            _new_item: Ref<'_, IShellItem>,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn PreCopyItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            _destination: Ref<'_, IShellItem>,
            _name: &PCWSTR,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn PostCopyItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            _destination: Ref<'_, IShellItem>,
            _name: &PCWSTR,
            _result: HRESULT,
            _new_item: Ref<'_, IShellItem>,
        ) -> WindowsResult<()> {
            Ok(())
        }

        fn PreDeleteItem(&self, flags: u32, _item: Ref<'_, IShellItem>) -> WindowsResult<()> {
            if flags & TSF_DELETE_RECYCLE_IF_POSSIBLE.0 as u32 == 0 {
                self.state.lock().unwrap().error =
                    Some("Windows Shell did not mark this operation for recycling.".into());
                return RecycleSink::reject("Permanent deletion was refused.");
            }
            self.state.lock().unwrap().pre_delete_seen = true;
            Ok(())
        }

        fn PostDeleteItem(
            &self,
            _flags: u32,
            _item: Ref<'_, IShellItem>,
            result: HRESULT,
            new_item: Ref<'_, IShellItem>,
        ) -> WindowsResult<()> {
            let mut state = self.state.lock().unwrap();
            if result.is_err() {
                state.error = Some(format!(
                    "Windows Shell could not recycle the item: {result:?}"
                ));
                return Ok(());
            }
            if new_item.is_null() {
                state.error = Some(
                    "Windows Shell did not return a Recycle Bin item; recycling could not be verified."
                        .into(),
                );
                return Ok(());
            }
            state.recycled_item_confirmed = true;
            Ok(())
        }

        fn PreNewItem(
            &self,
            _flags: u32,
            _destination: Ref<'_, IShellItem>,
            _name: &PCWSTR,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn PostNewItem(
            &self,
            _flags: u32,
            _destination: Ref<'_, IShellItem>,
            _name: &PCWSTR,
            _template: &PCWSTR,
            _attributes: u32,
            _result: HRESULT,
            _new_item: Ref<'_, IShellItem>,
        ) -> WindowsResult<()> {
            Ok(())
        }
        fn UpdateProgress(&self, _total: u32, _completed: u32) -> WindowsResult<()> {
            Ok(())
        }
        fn ResetTimer(&self) -> WindowsResult<()> {
            Ok(())
        }
        fn PauseTimer(&self) -> WindowsResult<()> {
            Ok(())
        }
        fn ResumeTimer(&self) -> WindowsResult<()> {
            Ok(())
        }
    }

    fn supported_path(path: &Path) -> bool {
        matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    }

    struct RecoveryStage {
        path: PathBuf,
        entries: Vec<(PathBuf, bool, Option<LinkInfo>)>,
    }

    impl RecoveryStage {
        fn create(source: &Path) -> Result<Self, String> {
            let metadata = std::fs::symlink_metadata(source)
                .map_err(|error| format!("Cannot inspect cleanup candidate: {error}"))?;
            let parent = source
                .parent()
                .ok_or("Cleanup candidate has no parent directory.")?;
            let mut last_error = None;
            for _ in 0..16 {
                let nonce = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(|error| format!("System clock is unavailable: {error}"))?
                    .as_nanos();
                let path = parent.join(format!(
                    ".dev-companion-recycle-recovery-{}-{nonce}-{}",
                    std::process::id(),
                    NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
                ));
                let created = if metadata.is_file() {
                    std::fs::hard_link(source, &path)
                } else {
                    std::fs::create_dir(&path)
                };
                match created {
                    Ok(()) => {
                        let mut stage = Self {
                            path,
                            entries: vec![(PathBuf::new(), metadata.is_dir(), None)],
                        };
                        if metadata.is_dir() {
                            if let Err(error) = stage.populate(source) {
                                return Err(stage.cleanup_error(source, error));
                            }
                        }
                        if let Err(error) = stage.verify(source) {
                            return Err(stage.cleanup_error(source, error));
                        }
                        return Ok(stage);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        last_error = Some(error);
                    }
                    Err(error) => {
                        return Err(format!(
                            "Cannot create a same-volume recovery hard link: {error}"
                        ));
                    }
                }
            }
            Err(format!(
                "Cannot allocate a collision-safe recovery path: {}",
                last_error.unwrap()
            ))
        }

        fn populate(&mut self, source: &Path) -> Result<(), String> {
            let mut queue = VecDeque::from([(source.to_path_buf(), PathBuf::new(), 0usize)]);
            while let Some((directory, relative, depth)) = queue.pop_front() {
                if depth >= MAX_STAGE_DEPTH {
                    return Err("Recovery staging exceeded its depth limit.".into());
                }
                for child in std::fs::read_dir(&directory)
                    .map_err(|error| format!("Cannot read cleanup candidate: {error}"))?
                {
                    let child = child.map_err(|error| {
                        format!("Cannot read a cleanup candidate entry: {error}")
                    })?;
                    if self.entries.len() >= MAX_STAGE_ENTRIES {
                        return Err("Recovery staging exceeded its entry limit.".into());
                    }
                    let child_path = child.path();
                    let metadata = std::fs::symlink_metadata(&child_path)
                        .map_err(|error| format!("Cannot inspect cleanup candidate: {error}"))?;
                    let child_relative = relative.join(child.file_name());
                    let staged_path = self.path.join(&child_relative);
                    if fs_safety::linked(&metadata) {
                        let link = inspect_link(&child_path)?;
                        create_link(&child_path, &staged_path, &link)?;
                        self.entries
                            .push((child_relative, link.directory, Some(link)));
                        continue;
                    }
                    fs_safety::check(&child_path)?;
                    if !metadata.is_file() && !metadata.is_dir() {
                        return Err(format!(
                            "Recovery staging refused a linked or special entry: {}",
                            child_path.display()
                        ));
                    }
                    if metadata.is_dir() {
                        std::fs::create_dir(&staged_path).map_err(|error| {
                            format!("Cannot create recovery directory: {error}")
                        })?;
                        queue.push_back((child_path, child_relative.clone(), depth + 1));
                    } else {
                        std::fs::hard_link(&child_path, &staged_path).map_err(|error| {
                            format!("Cannot create a recovery hard link: {error}")
                        })?;
                    }
                    self.entries.push((child_relative, metadata.is_dir(), None));
                }
            }
            Ok(())
        }

        fn verify(&self, source: &Path) -> Result<(), String> {
            let expected: HashSet<PathBuf> = self
                .entries
                .iter()
                .map(|(relative, _, _)| relative.clone())
                .collect();
            for (relative, directory, link) in &self.entries {
                let source_path = entry_path(source, relative);
                let staged_path = entry_path(&self.path, relative);
                if let Some(expected) = link {
                    if inspect_link(&source_path).ok().as_ref() != Some(expected) {
                        return Err("Dependency link changed during recovery staging.".into());
                    }
                    if inspect_link(&staged_path)? != *expected {
                        return Err("Recovery dependency link could not be verified.".into());
                    }
                    continue;
                }
                let source_metadata = std::fs::symlink_metadata(&source_path).map_err(|error| {
                    format!("Cleanup candidate changed during staging: {error}")
                })?;
                let staged_metadata = std::fs::symlink_metadata(&staged_path)
                    .map_err(|error| format!("Recovery staging changed: {error}"))?;
                if fs_safety::linked(&source_metadata)
                    || fs_safety::linked(&staged_metadata)
                    || source_metadata.is_dir() != *directory
                    || staged_metadata.is_dir() != *directory
                {
                    return Err("Cleanup candidate changed during recovery staging.".into());
                }
                if !directory && file_identity(&source_path)? != file_identity(&staged_path)? {
                    return Err("Recovery hard-link identity could not be verified.".into());
                }
            }
            if inventory(source)? != expected || inventory(&self.path)? != expected {
                return Err("Cleanup candidate inventory changed during recovery staging.".into());
            }
            Ok(())
        }

        fn discard(self) -> Result<(), String> {
            remove_stage(&self.path, &self.entries)
        }

        fn recover(self, source: &Path) -> Result<&'static str, String> {
            match std::fs::symlink_metadata(source) {
                Ok(_) => Err(format!(
                    "The original path still exists and was not overwritten. Recovery copy retained at {}",
                    self.path.display()
                )),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::rename(&self.path, source).map_err(|error| {
                        format!(
                            "Cannot restore the candidate: {error}. Recovery copy retained at {}",
                            self.path.display()
                        )
                    })?;
                    Ok("The candidate was restored from its recovery stage.")
                }
                Err(error) => Err(format!(
                    "Cannot inspect the original candidate: {error}. Recovery copy retained at {}",
                    self.path.display()
                )),
            }
        }

        fn cleanup_error(self, source: &Path, error: String) -> String {
            let recovery_path = self.path.clone();
            if self.verify(source).is_err() {
                return format!(
                    "{error} Source integrity could not be reverified. Recovery copy retained at {}",
                    recovery_path.display()
                );
            }
            match self.discard() {
                Ok(()) => error,
                Err(cleanup) => format!(
                    "{error} Recovery staging could not be removed: {cleanup}. Recovery copy retained at {}",
                    recovery_path.display()
                ),
            }
        }
    }

    fn file_identity(path: &Path) -> Result<(u32, u64), String> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let share = FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0);
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_READ_ATTRIBUTES.0,
                share,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        }
        .map_err(|error| format!("Cannot open a staged file for identity verification: {error}"))?;
        let mut information = BY_HANDLE_FILE_INFORMATION::default();
        let result = unsafe { GetFileInformationByHandle(handle, &mut information) }
            .map(|()| {
                (
                    information.dwVolumeSerialNumber,
                    ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
                )
            })
            .map_err(|error| format!("Cannot verify staged file identity: {error}"));
        let close = unsafe { CloseHandle(handle) }
            .map_err(|error| format!("Cannot close staged file identity handle: {error}"));
        match (result, close) {
            (Ok(identity), Ok(())) => Ok(identity),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub(super) fn inspect_link(path: &Path) -> Result<LinkInfo, String> {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|error| format!("Cannot inspect linked dependency: {error}"))?;
        if !fs_safety::linked(&metadata) {
            return Err("Dependency entry is no longer a link or junction.".into());
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let share = FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0);
        let flags = FILE_FLAGS_AND_ATTRIBUTES(
            FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_BACKUP_SEMANTICS.0,
        );
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_READ_ATTRIBUTES.0,
                share,
                None,
                OPEN_EXISTING,
                flags,
                None,
            )
        }
        .map_err(|error| format!("Cannot open linked dependency: {error}"))?;
        let mut tag = FILE_ATTRIBUTE_TAG_INFO::default();
        let result = unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfo,
                &mut tag as *mut _ as *mut _,
                std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            )
        }
        .map(|()| tag.ReparseTag)
        .map_err(|error| format!("Cannot read linked dependency tag: {error}"));
        let close = unsafe { CloseHandle(handle) }
            .map_err(|error| format!("Cannot close linked dependency handle: {error}"));
        let tag = match (result, close) {
            (Ok(tag), Ok(())) => tag,
            (Err(error), _) | (_, Err(error)) => return Err(error),
        };
        if !matches!(tag, IO_REPARSE_TAG_SYMLINK | IO_REPARSE_TAG_MOUNT_POINT) {
            return Err(format!("Unsupported dependency reparse tag: 0x{tag:08X}."));
        }
        Ok(LinkInfo {
            target: std::fs::read_link(path)
                .map_err(|error| format!("Cannot read linked dependency target: {error}"))?,
            directory: metadata.file_attributes() & 0x10 != 0,
            tag,
        })
    }

    pub(super) fn create_link(source: &Path, path: &Path, info: &LinkInfo) -> Result<(), String> {
        const FSCTL_SET_REPARSE_POINT: u32 = 589_988;
        const FSCTL_GET_REPARSE_POINT: u32 = 589_992;
        const GENERIC_WRITE: u32 = 0x4000_0000;
        const MAX_REPARSE_BYTES: usize = 16_384;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn DeviceIoControl(
                device: windows::Win32::Foundation::HANDLE,
                control: u32,
                input: *const std::ffi::c_void,
                input_size: u32,
                output: *mut std::ffi::c_void,
                output_size: u32,
                returned: *mut u32,
                overlapped: *mut std::ffi::c_void,
            ) -> i32;
        }

        fn open_reparse(
            path: &Path,
            access: u32,
        ) -> Result<windows::Win32::Foundation::HANDLE, String> {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let share =
                FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0);
            let flags = FILE_FLAGS_AND_ATTRIBUTES(
                FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_BACKUP_SEMANTICS.0,
            );
            unsafe {
                CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    access,
                    share,
                    None,
                    OPEN_EXISTING,
                    flags,
                    None,
                )
            }
            .map_err(|error| format!("Cannot open dependency reparse point: {error}"))
        }

        let source_handle = open_reparse(source, 0)?;
        let mut buffer = vec![0u8; MAX_REPARSE_BYTES];
        let mut returned = 0u32;
        let read_ok = unsafe {
            DeviceIoControl(
                source_handle,
                FSCTL_GET_REPARSE_POINT,
                std::ptr::null(),
                0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        } != 0;
        let read_error = (!read_ok).then(std::io::Error::last_os_error);
        let close_source = unsafe { CloseHandle(source_handle) };
        close_source.map_err(|error| format!("Cannot close dependency reparse handle: {error}"))?;
        if let Some(error) = read_error {
            return Err(format!("Cannot read dependency reparse data: {error}"));
        }
        if returned < 8 {
            return Err("Dependency reparse data is truncated.".into());
        }
        buffer.truncate(returned as usize);
        let tag = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        if tag != info.tag || !matches!(tag, IO_REPARSE_TAG_SYMLINK | IO_REPARSE_TAG_MOUNT_POINT) {
            return Err("Dependency reparse tag changed during recovery staging.".into());
        }

        if info.directory {
            std::fs::create_dir(path)
        } else {
            std::fs::File::create(path).map(|_| ())
        }
        .map_err(|error| format!("Cannot create recovery link placeholder: {error}"))?;
        let destination = match open_reparse(path, GENERIC_WRITE) {
            Ok(handle) => handle,
            Err(error) => {
                let _ = if info.directory {
                    std::fs::remove_dir(path)
                } else {
                    std::fs::remove_file(path)
                };
                return Err(error);
            }
        };
        let mut ignored = 0u32;
        let write_ok = unsafe {
            DeviceIoControl(
                destination,
                FSCTL_SET_REPARSE_POINT,
                buffer.as_ptr().cast(),
                buffer.len() as u32,
                std::ptr::null_mut(),
                0,
                &mut ignored,
                std::ptr::null_mut(),
            )
        } != 0;
        let write_error = (!write_ok).then(std::io::Error::last_os_error);
        let close_destination = unsafe { CloseHandle(destination) };
        close_destination
            .map_err(|error| format!("Cannot close recovery reparse handle: {error}"))?;
        if let Some(error) = write_error {
            let _ = if info.directory {
                std::fs::remove_dir(path)
            } else {
                std::fs::remove_file(path)
            };
            return Err(format!("Cannot clone dependency reparse data: {error}"));
        }
        if inspect_link(path)? != *info {
            return Err("Cloned dependency reparse point failed verification.".into());
        }
        Ok(())
    }

    fn inventory(root: &Path) -> Result<HashSet<PathBuf>, String> {
        let mut found = HashSet::from([PathBuf::new()]);
        if std::fs::symlink_metadata(root).is_ok_and(|metadata| metadata.is_file()) {
            return Ok(found);
        }
        let mut queue = VecDeque::from([(root.to_path_buf(), PathBuf::new(), 0usize)]);
        while let Some((directory, relative, depth)) = queue.pop_front() {
            if depth >= MAX_STAGE_DEPTH {
                return Err("Recovery verification exceeded its depth limit.".into());
            }
            for child in std::fs::read_dir(&directory)
                .map_err(|error| format!("Cannot verify recovery inventory: {error}"))?
            {
                let child =
                    child.map_err(|error| format!("Cannot verify recovery entry: {error}"))?;
                if found.len() >= MAX_STAGE_ENTRIES {
                    return Err("Recovery verification exceeded its entry limit.".into());
                }
                let metadata = std::fs::symlink_metadata(child.path())
                    .map_err(|error| format!("Cannot inspect recovery entry: {error}"))?;
                let child_relative = relative.join(child.file_name());
                if fs_safety::linked(&metadata) {
                    found.insert(child_relative.clone());
                    continue;
                }
                if !metadata.is_file() && !metadata.is_dir() {
                    return Err("Recovery verification found a linked or special entry.".into());
                }
                found.insert(child_relative.clone());
                if metadata.is_dir() {
                    queue.push_back((child.path(), child_relative, depth + 1));
                }
            }
        }
        Ok(found)
    }

    fn remove_stage(
        root: &Path,
        entries: &[(PathBuf, bool, Option<LinkInfo>)],
    ) -> Result<(), String> {
        for (relative, directory, link) in entries.iter().filter(|entry| entry.2.is_some()) {
            let path = entry_path(root, relative);
            let result = if *directory {
                std::fs::remove_dir(&path)
            } else {
                std::fs::remove_file(&path)
            };
            result.map_err(|error| format!("Cannot remove recovery dependency link: {error}"))?;
            let _ = link;
        }
        let mut files: Vec<&PathBuf> = entries
            .iter()
            .filter_map(|(path, directory, link)| (!directory && link.is_none()).then_some(path))
            .collect();
        files.sort_by(|a, b| b.cmp(a));
        for relative in files {
            let path = entry_path(root, relative);
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|error| format!("Cannot inspect recovery file: {error}"))?;
            if !metadata.is_file() || fs_safety::linked(&metadata) {
                return Err(format!("Recovery file changed: {}", path.display()));
            }
            std::fs::remove_file(&path)
                .map_err(|error| format!("Cannot remove recovery file: {error}"))?;
        }
        let mut directories: Vec<&PathBuf> = entries
            .iter()
            .filter_map(|(path, directory, link)| (*directory && link.is_none()).then_some(path))
            .collect();
        directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        for relative in directories {
            let path = entry_path(root, relative);
            std::fs::remove_dir(&path)
                .map_err(|error| format!("Cannot remove recovery directory: {error}"))?;
        }
        Ok(())
    }

    fn entry_path(root: &Path, relative: &Path) -> PathBuf {
        if relative.as_os_str().is_empty() {
            root.to_path_buf()
        } else {
            root.join(relative)
        }
    }

    #[cfg(test)]
    mod recovery_tests {
        use super::*;

        fn fixture(name: &str) -> PathBuf {
            let nonce = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("recycle-recovery-{name}-{nonce}"));
            std::fs::create_dir_all(&root).unwrap();
            root
        }

        #[test]
        fn restores_hardlinked_file_after_simulated_null_callback() {
            let root = fixture("restore");
            let source = root.join(".eslintcache");
            std::fs::write(&source, "recoverable").unwrap();
            let stage = RecoveryStage::create(&source).unwrap();
            stage.verify(&source).unwrap();
            std::fs::remove_file(&source).unwrap();

            assert_eq!(
                stage.recover(&source).unwrap(),
                "The candidate was restored from its recovery stage."
            );
            assert_eq!(std::fs::read_to_string(&source).unwrap(), "recoverable");
            std::fs::remove_file(source).unwrap();
            std::fs::remove_dir(root).unwrap();
        }

        #[test]
        fn retains_recovery_when_original_path_reappears() {
            let root = fixture("collision");
            let source = root.join(".stylelintcache");
            std::fs::write(&source, "original").unwrap();
            let stage = RecoveryStage::create(&source).unwrap();
            let recovery_path = stage.path.clone();
            std::fs::remove_file(&source).unwrap();
            std::fs::write(&source, "replacement").unwrap();

            let error = stage.recover(&source).unwrap_err();
            assert!(error.contains(&recovery_path.display().to_string()));
            assert_eq!(std::fs::read_to_string(&source).unwrap(), "replacement");
            assert_eq!(std::fs::read_to_string(&recovery_path).unwrap(), "original");
            std::fs::remove_file(source).unwrap();
            std::fs::remove_file(recovery_path).unwrap();
            std::fs::remove_dir(root).unwrap();
        }

        #[test]
        fn retains_complete_recovery_when_source_directory_is_partial() {
            let root = fixture("partial-directory");
            let source = root.join("node_modules");
            std::fs::create_dir(&source).unwrap();
            std::fs::write(source.join("kept"), "kept").unwrap();
            std::fs::write(source.join("missing"), "recover me").unwrap();
            let stage = RecoveryStage::create(&source).unwrap();
            let recovery_path = stage.path.clone();
            let recovery_entries = stage.entries.clone();
            std::fs::remove_file(source.join("missing")).unwrap();

            let error = stage.recover(&source).unwrap_err();
            assert!(error.contains(&recovery_path.display().to_string()));
            assert_eq!(
                std::fs::read_to_string(recovery_path.join("missing")).unwrap(),
                "recover me"
            );
            assert_eq!(
                std::fs::read_to_string(source.join("kept")).unwrap(),
                "kept"
            );
            std::fs::remove_file(source.join("kept")).unwrap();
            std::fs::remove_dir(source).unwrap();
            remove_stage(&recovery_path, &recovery_entries).unwrap();
            std::fs::remove_dir(root).unwrap();
        }

        #[test]
        fn validation_failure_retains_stage_when_source_disappears() {
            let root = fixture("validation-removal");
            let source = root.join(".eslintcache");
            std::fs::write(&source, "recoverable").unwrap();
            let stage = RecoveryStage::create(&source).unwrap();
            let recovery_path = stage.path.clone();
            std::fs::remove_file(&source).unwrap();

            let error = stage.cleanup_error(&source, "Snapshot changed.".into());
            assert!(error.contains(&recovery_path.display().to_string()));
            assert_eq!(
                std::fs::read_to_string(&recovery_path).unwrap(),
                "recoverable"
            );
            std::fs::remove_file(recovery_path).unwrap();
            std::fs::remove_dir(root).unwrap();
        }

        #[test]
        fn discards_recovery_only_after_simulated_confirmed_recycle() {
            let root = fixture("success");
            let source = root.join(".eslintcache");
            let simulated_bin = root.join("simulated-bin-item");
            std::fs::write(&source, "recycled").unwrap();
            let stage = RecoveryStage::create(&source).unwrap();
            let recovery_path = stage.path.clone();
            std::fs::rename(&source, &simulated_bin).unwrap();

            stage.discard().unwrap();

            assert!(!recovery_path.exists());
            assert_eq!(std::fs::read_to_string(&simulated_bin).unwrap(), "recycled");
            std::fs::remove_file(simulated_bin).unwrap();
            std::fs::remove_dir(root).unwrap();
        }
    }

    pub(super) fn move_to_bin<F>(
        path: &Path,
        validate_snapshot: F,
    ) -> Result<Option<String>, String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        if !path.is_absolute() || !supported_path(path) {
            return Err(
                "Recycle Bin cleanup requires a local drive path; UNC paths are unsupported."
                    .into(),
            );
        }
        fs_safety::check(path)?;
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|error| format!("Cannot inspect cleanup candidate: {error}"))?;
        if (!metadata.is_file() && !metadata.is_dir()) || fs_safety::linked(&metadata) {
            return Err("Cleanup candidate is not a regular file or directory.".into());
        }

        let _apartment = ComApartment::initialize()?;
        // Shell parsing names do not accept the verbatim prefix used by filesystem paths.
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.starts_with(&[92, 92, 63, 92]) {
            wide.drain(..4);
        }
        wide.push(0);
        let item: IShellItem = unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) }
            .map_err(|error| format!("Cannot open cleanup candidate in Windows Shell: {error}"))?;
        let operation: IFileOperation =
            unsafe { CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER) }.map_err(
                |error| format!("Cannot create the Windows Shell file operation: {error}"),
            )?;
        let flags = FILEOPERATION_FLAGS(
            FOFX_RECYCLEONDELETE.0
                | FOFX_EARLYFAILURE.0
                | FOF_NOCONFIRMATION.0
                | FOF_NOERRORUI.0
                | FOF_NO_CONNECTED_ELEMENTS.0
                | FOF_SILENT.0,
        );
        unsafe { operation.SetOperationFlags(flags) }
            .map_err(|error| format!("Cannot configure the Recycle Bin operation: {error}"))?;

        let state = Arc::new(Mutex::new(SinkState::default()));
        let sink: IFileOperationProgressSink = RecycleSink {
            state: state.clone(),
        }
        .into();
        unsafe { operation.DeleteItem(&item, &sink) }
            .map_err(|error| format!("Cannot queue the Recycle Bin operation: {error}"))?;

        let stage = RecoveryStage::create(path)?;
        if let Err(error) = validate_snapshot().and_then(|()| stage.verify(path)) {
            return Err(stage.cleanup_error(path, error));
        }

        let perform_error = unsafe { operation.PerformOperations() }
            .err()
            .map(|error| format!("Recycle Bin operation failed: {error}"));
        let aborted = unsafe { operation.GetAnyOperationsAborted() }
            .map(|value| value.as_bool())
            .map_err(|error| format!("Cannot verify the Recycle Bin operation: {error}"));
        let state = state.lock().unwrap();
        let callback_error = state.error.clone();
        let callback_confirmed = state.pre_delete_seen && state.recycled_item_confirmed;
        drop(state);
        let source_gone = matches!(
            std::fs::symlink_metadata(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        );
        if perform_error.is_none()
            && aborted.as_ref().is_ok_and(|aborted| !aborted)
            && callback_error.is_none()
            && callback_confirmed
            && source_gone
        {
            let recovery_path = stage.path.clone();
            return match stage.discard() {
                Ok(()) => Ok(None),
                Err(error) => Ok(Some(format!(
                    "Candidate reached the Recycle Bin, but its recovery stage could not be removed: {error}. Recovery copy retained at {}",
                    recovery_path.display()
                ))),
            };
        }

        let reason = perform_error
            .or_else(|| aborted.err())
            .or(callback_error)
            .unwrap_or_else(|| {
                if !callback_confirmed {
                    "Windows Shell did not confirm a Recycle Bin move.".into()
                } else {
                    "Cleanup candidate still exists after the Recycle Bin operation.".into()
                }
            });
        match stage.recover(path) {
            Ok(recovery) => Err(format!("{reason} {recovery}")),
            Err(recovery) => Err(format!("{reason} {recovery}")),
        }
    }
}

pub(super) fn inspect_link(path: &Path) -> Result<LinkInfo, String> {
    #[cfg(windows)]
    {
        windows_shell::inspect_link(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Dependency links are supported only on Windows.".into())
    }
}

pub fn move_to_bin<F>(path: &Path, validate_snapshot: F) -> Result<Option<String>, String>
where
    F: FnOnce() -> Result<(), String>,
{
    #[cfg(windows)]
    {
        windows_shell::move_to_bin(path, validate_snapshot)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, validate_snapshot);
        Err("Recycle Bin cleanup is available only on Windows.".into())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    #[ignore = "moves one uniquely named file owned by this test into the Windows Recycle Bin"]
    fn native_recycles_owned_file() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("project-cleanup-native-recycle");
        fs::create_dir_all(&fixture).unwrap();
        let cache = fixture.join(format!(".eslintcache-{nonce}"));
        fs::write(&cache, "owned native recycle fixture").unwrap();

        assert!(move_to_bin(&cache, || Ok(())).unwrap().is_none());

        assert!(!cache.exists());
        fs::remove_dir(&fixture).unwrap();
    }

    #[test]
    #[ignore = "moves one tiny owned directory into the Windows Recycle Bin"]
    fn native_recycles_owned_directory() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("project-cleanup-native-recycle-directory");
        let candidate = fixture.join(format!("node_modules-{nonce}"));
        fs::create_dir_all(candidate.join("nested")).unwrap();
        fs::write(candidate.join("first"), "one").unwrap();
        fs::write(candidate.join("nested/second"), "two").unwrap();

        assert!(move_to_bin(&candidate, || Ok(())).unwrap().is_none());

        assert!(!candidate.exists());
        assert!(fs::read_dir(&fixture).unwrap().next().is_none());
        fs::remove_dir(&fixture).unwrap();
    }

    #[test]
    #[ignore = "moves one tiny owned pnpm-like linked directory into the Windows Recycle Bin"]
    fn native_recycles_owned_pnpm_linked_directory() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("project-cleanup-native-recycle-pnpm dự án");
        let candidate = fixture.join(format!("node_modules-{nonce}"));
        let store = candidate
            .join(".pnpm")
            .join("pkg")
            .join("node_modules")
            .join("pkg");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join("index.js"), "generated").unwrap();
        let junction = candidate.join("pkg");
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&store)
            .status()
            .unwrap();
        assert!(status.success());
        let outside = fixture.join(format!("outside-{nonce}"));
        fs::create_dir(&outside).unwrap();
        let outside_sentinel = outside.join("sentinel.txt");
        fs::write(&outside_sentinel, "outside store").unwrap();
        fs::hard_link(&outside_sentinel, store.join("hardlinked.txt")).unwrap();

        let canonical_candidate = fs::canonicalize(&candidate).unwrap();
        let result = move_to_bin(&canonical_candidate, || Ok(()));
        if result.is_err() {
            fs::remove_dir(&junction).unwrap();
            fs::remove_dir_all(&candidate).unwrap();
        }

        assert!(!candidate.exists());
        assert_eq!(
            fs::read_to_string(&outside_sentinel).unwrap(),
            "outside store"
        );
        fs::remove_dir_all(outside).unwrap();
        assert!(fs::read_dir(&fixture).unwrap().next().is_none());
        fs::remove_dir(&fixture).unwrap();
        assert!(result.unwrap().is_none());
    }
}
