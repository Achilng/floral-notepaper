//! Native OLE drag-and-drop for handing a note's `.md` file to another
//! application (QQ / WeChat / Explorer). Only a clean copy — with the note id
//! marker already stripped — is exposed through a `CF_HDROP` data object.
//!
//! This module is Windows-only; the caller in `lib.rs` gates it with
//! `#[cfg(target_os = "windows")]`.

use std::cell::Cell;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::core::{implement, Error, Ref, BOOL, HRESULT};
use windows::Win32::Foundation::{
    GlobalFree, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC,
    E_NOTIMPL, E_OUTOFMEMORY, POINT, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC,
    IEnumFORMATETC_Impl, IEnumSTATDATA, COINIT_MULTITHREADED, DVASPECT_CONTENT, FORMATETC,
    STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::{
    DoDragDrop, IDropSource, IDropSource_Impl, OleInitialize, OleUninitialize, CF_HDROP,
    DROPEFFECT, DROPEFFECT_COPY,
};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Shell::DROPFILES;

/// Runs a blocking OLE drag so the target application receives the file at
/// `path` as a `CF_HDROP`. Must be called from a thread with a message pump
/// (the main thread); the call does not return until the drag finishes.
pub fn drag_file_out(path: &Path) -> Result<(), String> {
    // The absolute path is used verbatim. `fs::canonicalize` would prefix it
    // with `\\?\`, which the receiving application cannot read.
    let mut files: Vec<u16> = path.as_os_str().encode_wide().collect();
    // CF_HDROP stores a list of NUL-terminated file names ended by an extra
    // NUL. A single file therefore needs two trailing NULs.
    files.push(0);
    files.push(0);

    let formatetc = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };

    let data_object: IDataObject = FileDataObject { formatetc, files }.into();
    let drop_source: IDropSource = DropSource.into();

    // The main thread may already have OLE initialized (WebView2 uses an STA).
    // OleInitialize then returns S_FALSE (still success) and must be balanced
    // with OleUninitialize; on RPC_E_CHANGED_MODE fall back to MTA COM.
    let mut ole_initialized = false;
    let mut com_initialized = false;
    unsafe {
        if OleInitialize(None).is_ok() {
            ole_initialized = true;
        } else if CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() {
            com_initialized = true;
        }
    }

    let mut effect = DROPEFFECT(0);
    let hr = unsafe { DoDragDrop(&data_object, &drop_source, DROPEFFECT_COPY, &mut effect) };

    unsafe {
        if ole_initialized {
            OleUninitialize();
        }
        if com_initialized {
            CoUninitialize();
        }
    }

    if hr.is_err() {
        return Err(format!("拖拽发送文件失败 (0x{:08X})", hr.0));
    }
    Ok(())
}

/// Exposes the single `CF_HDROP` format backed by an in-memory `DROPFILES`
/// allocation built on demand in `GetData`.
#[implement(IDataObject)]
struct FileDataObject {
    formatetc: FORMATETC,
    files: Vec<u16>,
}

impl FileDataObject {
    fn build_medium(&self) -> windows::core::Result<STGMEDIUM> {
        let header = std::mem::size_of::<DROPFILES>();
        let payload = self.files.len() * std::mem::size_of::<u16>();
        let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE, header + payload)? };
        let locked = unsafe { GlobalLock(hglobal) };
        if locked.is_null() {
            // Rare (out of memory). Release the handle rather than leak it.
            unsafe {
                let _ = GlobalFree(Some(hglobal));
            }
            return Err(Error::from_hresult(E_OUTOFMEMORY));
        }

        unsafe {
            let dropfiles = locked.cast::<DROPFILES>();
            (*dropfiles).pFiles = header as u32;
            (*dropfiles).pt = POINT { x: 0, y: 0 };
            (*dropfiles).fNC = BOOL(0);
            (*dropfiles).fWide = BOOL(1);
            let file_list = locked.add(header).cast::<u16>();
            std::ptr::copy_nonoverlapping(self.files.as_ptr(), file_list, self.files.len());
            let _ = GlobalUnlock(hglobal);
        }

        Ok(STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: hglobal },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        })
    }
}

impl IDataObject_Impl for FileDataObject_Impl {
    fn GetData(&self, pformatetcin: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        let requested = unsafe { &*pformatetcin };
        if requested.cfFormat != self.formatetc.cfFormat
            || requested.tymed & self.formatetc.tymed == 0
        {
            return Err(Error::from_hresult(DV_E_FORMATETC));
        }
        self.build_medium()
    }

    fn GetDataHere(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *mut STGMEDIUM,
    ) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        let requested = unsafe { &*pformatetc };
        if requested.cfFormat == self.formatetc.cfFormat
            && requested.tymed & self.formatetc.tymed != 0
        {
            S_OK
        } else {
            S_FALSE
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const FORMATETC,
        _pformatetcout: *mut FORMATETC,
    ) -> HRESULT {
        E_NOTIMPL
    }

    fn SetData(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *const STGMEDIUM,
        _frelease: BOOL,
    ) -> windows::core::Result<()> {
        // A drag source has nothing to store; DoDragDrop reports the effect.
        Ok(())
    }

    fn EnumFormatEtc(&self, dwdirection: u32) -> windows::core::Result<IEnumFORMATETC> {
        // DATADIR_GET == 1. Drag sources only ever enumerate for Get.
        if dwdirection != 1 {
            return Err(E_NOTIMPL.into());
        }
        Ok(FileFormatEnum {
            formatetc: self.formatetc,
            index: Cell::new(0),
        }
        .into())
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: Ref<'_, IAdviseSink>,
    ) -> windows::core::Result<u32> {
        Err(E_NOTIMPL.into())
    }

    fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> {
        Err(E_NOTIMPL.into())
    }
}

/// Enumerates the single `CF_HDROP` format. A real `IEnumFORMATETC` is
/// required: returning `E_NOTIMPL` here makes the drop target see no formats,
/// which is why an earlier attempt produced "empty" files in QQ.
#[implement(IEnumFORMATETC)]
struct FileFormatEnum {
    formatetc: FORMATETC,
    index: Cell<u32>,
}

impl IEnumFORMATETC_Impl for FileFormatEnum_Impl {
    fn Next(&self, celt: u32, rgelt: *mut FORMATETC, pceltfetched: *mut u32) -> HRESULT {
        if celt == 0 || self.index.get() >= 1 {
            unsafe {
                if !pceltfetched.is_null() {
                    *pceltfetched = 0;
                }
            }
            return S_FALSE;
        }
        unsafe {
            *rgelt = self.formatetc;
            if !pceltfetched.is_null() {
                *pceltfetched = 1;
            }
        }
        self.index.set(1);
        S_OK
    }

    fn Skip(&self, celt: u32) -> windows::core::Result<()> {
        if celt == 0 {
            return Ok(());
        }
        if self.index.get() >= 1 {
            return Err(Error::from_hresult(S_FALSE));
        }
        self.index.set(1);
        Ok(())
    }

    fn Reset(&self) -> windows::core::Result<()> {
        self.index.set(0);
        Ok(())
    }

    fn Clone(&self) -> windows::core::Result<IEnumFORMATETC> {
        Ok(FileFormatEnum {
            formatetc: self.formatetc,
            index: Cell::new(self.index.get()),
        }
        .into())
    }
}

#[implement(IDropSource)]
struct DropSource;

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, fescapepressed: BOOL, grfkeystate: MODIFIERKEYS_FLAGS) -> HRESULT {
        if fescapepressed.as_bool() {
            DRAGDROP_S_CANCEL
        } else if grfkeystate.0 & MK_LBUTTON.0 == 0 {
            // Left button released while dragging -> perform the drop.
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}
