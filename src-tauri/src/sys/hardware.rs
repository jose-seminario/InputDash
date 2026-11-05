use core::ffi::c_void;
use core::ptr;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use windows::core::GUID;
use windows::Win32::Foundation::{LUID, POINTL, RECTL};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: u64,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub resolution: String,
    pub refresh_hz: f64,
    pub supports_vrr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidDeviceInfo {
    pub name: String,
    pub vid: String,
    pub pid: String,
    pub transport: String,
    pub device_type: String,
    pub raw_path: String,
    pub usb_port_speed: String,
    pub port_recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareDiagnosticInfo {
    pub gpu: GpuInfo,
    pub monitor: MonitorInfo,
    pub mice: Vec<HidDeviceInfo>,
    pub keyboards: Vec<HidDeviceInfo>,
    pub hvci_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDeviceNode {
    pub device_name: String,
    pub instance_id: String,
    pub is_mouse: bool,
    pub is_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbHostControllerGroup {
    pub controller_id: String,
    pub controller_name: String,
    pub devices: Vec<UsbDeviceNode>,
    pub has_contention: bool,
    pub contention_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbTopologyInfo {
    pub controllers: Vec<UsbHostControllerGroup>,
    pub any_contention_detected: bool,
}

// -----------------------------------------------------------------------------
// DXGI COM FFI
// -----------------------------------------------------------------------------

#[repr(C)]
struct DXGI_ADAPTER_DESC {
    Description: [u16; 128],
    VendorId: u32,
    DeviceId: u32,
    SubSysId: u32,
    Revision: u32,
    DedicatedVideoMemory: usize,
    DedicatedSystemMemory: usize,
    SharedSystemMemory: usize,
    AdapterLuid: LUID,
}

#[link(name = "dxgi")]
extern "system" {
    fn CreateDXGIFactory(riid: *const GUID, pp_factory: *mut *mut c_void) -> i32;
    fn CreateDXGIFactory1(riid: *const GUID, pp_factory: *mut *mut c_void) -> i32;
}

// -----------------------------------------------------------------------------
// DisplayConfig FFI (user32.dll)
// -----------------------------------------------------------------------------

const QDC_ONLY_ACTIVE_PATHS: u32 = 0x00000002;
const QDC_ALL_PATHS: u32 = 0x00000001;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_RATIONAL {
    Numerator: u32,
    Denominator: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_PATH_SOURCE_INFO {
    adapterId: LUID,
    id: u32,
    modeInfoIdx: u32,
    statusFlags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_PATH_TARGET_INFO {
    adapterId: LUID,
    id: u32,
    modeInfoIdx: u32,
    outputTechnology: u32,
    rotation: u32,
    scaling: u32,
    refreshRate: DISPLAYCONFIG_RATIONAL,
    scanLineOrdering: u32,
    targetAvailable: i32,
    statusFlags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_PATH_INFO {
    sourceInfo: DISPLAYCONFIG_PATH_SOURCE_INFO,
    targetInfo: DISPLAYCONFIG_PATH_TARGET_INFO,
    flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_2DREGION {
    cx: u32,
    cy: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_VIDEO_SIGNAL_INFO {
    pixelRate: u64,
    hSyncFreq: DISPLAYCONFIG_RATIONAL,
    vSyncFreq: DISPLAYCONFIG_RATIONAL,
    activeSize: DISPLAYCONFIG_2DREGION,
    totalSize: DISPLAYCONFIG_2DREGION,
    videoStandard: u32,
    scanLineOrdering: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_TARGET_MODE {
    targetVideoSignalInfo: DISPLAYCONFIG_VIDEO_SIGNAL_INFO,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_SOURCE_MODE {
    width: u32,
    height: u32,
    pixelFormat: u32,
    position: POINTL,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_DESKTOP_IMAGE_INFO {
    PathSourceSize: POINTL,
    DesktopImageRegion: RECTL,
    DesktopImageClip: RECTL,
}

#[repr(C)]
#[derive(Clone, Copy)]
union DISPLAYCONFIG_MODE_INFO_UNION {
    targetMode: DISPLAYCONFIG_TARGET_MODE,
    sourceMode: DISPLAYCONFIG_SOURCE_MODE,
    desktopImageInfo: DISPLAYCONFIG_DESKTOP_IMAGE_INFO,
}

impl Default for DISPLAYCONFIG_MODE_INFO_UNION {
    fn default() -> Self {
        DISPLAYCONFIG_MODE_INFO_UNION {
            sourceMode: DISPLAYCONFIG_SOURCE_MODE::default(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DISPLAYCONFIG_MODE_INFO {
    infoType: u32,
    id: u32,
    adapterId: LUID,
    mode: DISPLAYCONFIG_MODE_INFO_UNION,
}

// -----------------------------------------------------------------------------
// Raw Input FFI (user32.dll)
// -----------------------------------------------------------------------------

const RIDI_DEVICENAME: u32 = 0x20000007;
const RIM_TYPEMOUSE: u32 = 0;
const RIM_TYPEKEYBOARD: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RAWINPUTDEVICELIST {
    hDevice: *mut c_void,
    dwType: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetDisplayConfigBufferSizes(
        flags: u32,
        num_path_array_elements: *mut u32,
        num_mode_info_array_elements: *mut u32,
    ) -> i32;

    fn QueryDisplayConfig(
        flags: u32,
        num_path_array_elements: *mut u32,
        path_array: *mut DISPLAYCONFIG_PATH_INFO,
        num_mode_info_array_elements: *mut u32,
        mode_info_array: *mut DISPLAYCONFIG_MODE_INFO,
        current_topology_id: *mut u32,
    ) -> i32;

    fn GetRawInputDeviceList(
        p_raw_input_device_list: *mut RAWINPUTDEVICELIST,
        pui_num_devices: *mut u32,
        cb_size: u32,
    ) -> u32;

    fn GetRawInputDeviceInfoW(
        h_device: *mut c_void,
        ui_command: u32,
        p_data: *mut c_void,
        pcb_size: *mut u32,
    ) -> u32;
}

// -----------------------------------------------------------------------------
// SetupAPI and CfgMgr32 FFI for USB Topology
// -----------------------------------------------------------------------------

const DIGCF_PRESENT: u32 = 0x00000002;
const DIGCF_ALLCLASSES: u32 = 0x00000004;
const SPDRP_DEVICEDESC: u32 = 0x00000000;
const SPDRP_FRIENDLYNAME: u32 = 0x0000000C;

#[repr(C)]
struct SP_DEVINFO_DATA {
    cbSize: u32,
    ClassGuid: GUID,
    DevInst: u32,
    Reserved: usize,
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(
        ClassGuid: *const GUID,
        Enumerator: *const u16,
        hwndParent: *mut c_void,
        Flags: u32,
    ) -> *mut c_void;
    fn SetupDiEnumDeviceInfo(
        DeviceInfoSet: *mut c_void,
        MemberIndex: u32,
        DeviceInfoData: *mut SP_DEVINFO_DATA,
    ) -> i32;
    fn SetupDiGetDeviceRegistryPropertyW(
        DeviceInfoSet: *mut c_void,
        DeviceInfoData: *const SP_DEVINFO_DATA,
        Property: u32,
        PropertyRegDataType: *mut u32,
        PropertyBuffer: *mut u8,
        PropertyBufferSize: u32,
        RequiredSize: *mut u32,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(DeviceInfoSet: *mut c_void) -> i32;
}

#[link(name = "cfgmgr32")]
extern "system" {
    fn CM_Get_Device_IDW(
        dnDevInst: u32,
        Buffer: *mut u16,
        BufferLen: u32,
        ulFlags: u32,
    ) -> u32;
    fn CM_Locate_DevNodeW(
        pdnDevInst: *mut u32,
        pDeviceID: *const u16,
        ulFlags: u32,
    ) -> u32;
    fn CM_Get_Parent(
        pdnDevInst: *mut u32,
        dnDevInst: u32,
        ulFlags: u32,
    ) -> u32;
}

// -----------------------------------------------------------------------------
#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut c_void,
    ) -> *mut c_void;
    fn DeviceIoControl(
        hDevice: *mut c_void,
        dwIoControlCode: u32,
        lpInBuffer: *const c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> i32;
    fn CloseHandle(hObject: *mut c_void) -> i32;
}

const IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX: u32 = 0x0022041C;
const IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2: u32 = 0x0022045C;

const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x00000080;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct USB_PROTOCOLS {
    ul: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct USB_NODE_CONNECTION_INFORMATION_EX_V2_FLAGS {
    ul: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct USB_NODE_CONNECTION_INFORMATION_EX_V2 {
    ConnectionIndex: u32,
    Length: u32,
    SupportedUsbProtocols: USB_PROTOCOLS,
    Flags: USB_NODE_CONNECTION_INFORMATION_EX_V2_FLAGS,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct USB_DEVICE_DESCRIPTOR {
    bLength: u8,
    bDescriptorType: u8,
    bcdUSB: u16,
    bDeviceClass: u8,
    bDeviceSubClass: u8,
    bDeviceProtocol: u8,
    bMaxPacketSize0: u8,
    idVendor: u16,
    idProduct: u16,
    bcdDevice: u16,
    iManufacturer: u8,
    iProduct: u8,
    iSerialNumber: u8,
    bNumConfigurations: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct USB_NODE_CONNECTION_INFORMATION_EX {
    ConnectionIndex: u32,
    DeviceDescriptor: USB_DEVICE_DESCRIPTOR,
    CurrentConfigurationValue: u8,
    Speed: u8,
    DeviceIsHub: u8,
    DeviceAddress: u16,
    NumberOfOpenPipes: u32,
    ConnectionStatus: u32,
}

// Implementation: GPU
// -----------------------------------------------------------------------------

pub fn detect_gpu_info() -> GpuInfo {
    let dxgi_res = detect_gpu_dxgi();

    match dxgi_res {
        Ok(mut gpu) => {
            // Note: associate driver version to the adapter with highest VRAM
            let driver_ver = query_gpu_driver_version(Some(&gpu.name));
            if !driver_ver.is_empty() {
                gpu.driver_version = driver_ver;
            }
            gpu
        }
        Err(_) => {
            let driver_ver = query_gpu_driver_version(None);
            GpuInfo {
                name: "Adaptador de Pantalla de Windows".to_string(),
                vram_mb: 0,
                driver_version: if driver_ver.is_empty() {
                    "Controlador Genérico".to_string()
                } else {
                    driver_ver
                },
            }
        }
    }
}

fn detect_gpu_dxgi() -> AppResult<GpuInfo> {
    const IID_IDXGI_FACTORY: GUID = GUID {
        data1: 0x7b7166ec,
        data2: 0x21c7,
        data3: 0x44ae,
        data4: [0xb2, 0x1a, 0xc9, 0xae, 0x32, 0x1a, 0xe3, 0x69],
    };

    let mut factory_ptr: *mut c_void = ptr::null_mut();
    let hr = unsafe {
        let hr1 = CreateDXGIFactory1(&IID_IDXGI_FACTORY, &mut factory_ptr);
        if hr1 == 0 && !factory_ptr.is_null() {
            hr1
        } else {
            CreateDXGIFactory(&IID_IDXGI_FACTORY, &mut factory_ptr)
        }
    };

    if hr != 0 || factory_ptr.is_null() {
        return Err(AppError::HardwareDiag(format!(
            "No se pudo crear fábrica DXGI (hr: {hr:#010X})"
        )));
    }

    struct FactoryGuard(*mut c_void);
    impl Drop for FactoryGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let vtbl = *(self.0 as *mut *mut usize);
                    let release_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
                        core::mem::transmute(*vtbl.add(2));
                    release_fn(self.0);
                }
            }
        }
    }
    let _factory_guard = FactoryGuard(factory_ptr);

    let vtbl = unsafe { *(factory_ptr as *mut *mut usize) };
    let enum_adapters_fn: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut *mut c_void,
    ) -> i32 = unsafe { core::mem::transmute(*vtbl.add(7)) };

    let mut selected_gpu: Option<GpuInfo> = None;
    let mut adapter_idx = 0u32;

    loop {
        let mut adapter_ptr: *mut c_void = ptr::null_mut();
        let a_hr = unsafe { enum_adapters_fn(factory_ptr, adapter_idx, &mut adapter_ptr) };
        if a_hr != 0 || adapter_ptr.is_null() {
            break;
        }

        let a_vtbl = unsafe { *(adapter_ptr as *mut *mut usize) };
        let get_desc_fn: unsafe extern "system" fn(
            *mut c_void,
            *mut DXGI_ADAPTER_DESC,
        ) -> i32 = unsafe { core::mem::transmute(*a_vtbl.add(8)) };
        let release_adapter_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
            unsafe { core::mem::transmute(*a_vtbl.add(2)) };

        let mut desc = DXGI_ADAPTER_DESC {
            Description: [0u16; 128],
            VendorId: 0,
            DeviceId: 0,
            SubSysId: 0,
            Revision: 0,
            DedicatedVideoMemory: 0,
            DedicatedSystemMemory: 0,
            SharedSystemMemory: 0,
            AdapterLuid: LUID::default(),
        };

        let d_hr = unsafe { get_desc_fn(adapter_ptr, &mut desc) };
        unsafe {
            release_adapter_fn(adapter_ptr);
        }

        if d_hr == 0 {
            let name = String::from_utf16_lossy(&desc.Description)
                .trim_matches('\0')
                .trim()
                .to_string();

            let vram_mb = (desc.DedicatedVideoMemory / (1024 * 1024)) as u64;

            let is_software = name.to_lowercase().contains("basic render")
                || name.to_lowercase().contains("software");

            let current = GpuInfo {
                name,
                vram_mb,
                driver_version: "Consultando...".to_string(),
            };

            // Pick the GPU with highest dedicated VRAM (discrete over integrated)
            if selected_gpu.is_none() || (!is_software && current.vram_mb > selected_gpu.as_ref().map(|g| g.vram_mb).unwrap_or(0)) {
                selected_gpu = Some(current);
            }
        }

        adapter_idx += 1;
    }

    selected_gpu.ok_or_else(|| AppError::HardwareDiag("No se encontró ningún adaptador DXGI compatible".to_string()))
}

fn query_gpu_driver_version(target_gpu_name: Option<&str>) -> String {
    use crate::sys::registry::{read_value, RegistryHive, RegistryState};

    const CLASS_BASE: &str = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}";

    let mut fallback_ver = String::new();

    for i in 0..15 {
        let subkey = format!("{CLASS_BASE}\\{i:04}");

        let desc = match read_value(RegistryHive::HKLM, &subkey, "DriverDesc") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(d))) => d,
            _ => continue,
        };

        let ver = match read_value(RegistryHive::HKLM, &subkey, "DriverVersion") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(v))) => v,
            _ => continue,
        };

        if fallback_ver.is_empty() && !ver.is_empty() {
            fallback_ver = ver.clone();
        }

        if let Some(target) = target_gpu_name {
            let desc_lower = desc.to_lowercase();
            let target_lower = target.to_lowercase();
            // Match vendor and name parts
            if desc_lower.contains(&target_lower) || target_lower.contains(&desc_lower) {
                return ver;
            }
        }
    }

    fallback_ver
}

// -----------------------------------------------------------------------------
// Implementation: Monitor & VRR
// -----------------------------------------------------------------------------

pub fn detect_monitor_info() -> MonitorInfo {
    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    let size_res = unsafe {
        GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            &mut num_modes,
        )
    };

    if size_res != 0 || num_paths == 0 || num_modes == 0 {
        return fallback_monitor_info();
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); num_paths as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); num_modes as usize];

    let q_res = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            ptr::null_mut(),
        )
    };

    if q_res != 0 {
        return fallback_monitor_info();
    }

    let first_path = &paths[0];
    let rr = first_path.targetInfo.refreshRate;
    let refresh_hz = if rr.Denominator > 0 {
        let raw = (rr.Numerator as f64) / (rr.Denominator as f64);
        (raw * 100.0).round() / 100.0
    } else {
        60.0
    };

    let source_idx = first_path.sourceInfo.modeInfoIdx as usize;
    let resolution = if source_idx < modes.len() && modes[source_idx].infoType == 1 {
        let src = unsafe { modes[source_idx].mode.sourceMode };
        format!("{} x {}", src.width, src.height)
    } else {
        "1920 x 1080".to_string()
    };

    let supports_vrr = check_vrr_support(first_path.targetInfo.id);

    MonitorInfo {
        resolution,
        refresh_hz,
        supports_vrr,
    }
}

fn check_vrr_support(_target_id: u32) -> bool {
    let mut all_paths_cnt: u32 = 0;
    let mut all_modes_cnt: u32 = 0;

    let sz_res = unsafe {
        GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut all_paths_cnt, &mut all_modes_cnt)
    };

    if sz_res != 0 || all_paths_cnt == 0 || all_modes_cnt == 0 {
        return false;
    }

    let mut all_paths = vec![DISPLAYCONFIG_PATH_INFO::default(); all_paths_cnt as usize];
    let mut all_modes = vec![DISPLAYCONFIG_MODE_INFO::default(); all_modes_cnt as usize];

    let q_res = unsafe {
        QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut all_paths_cnt,
            all_paths.as_mut_ptr(),
            &mut all_modes_cnt,
            all_modes.as_mut_ptr(),
            ptr::null_mut(),
        )
    };

    if q_res != 0 {
        return false;
    }

    let mut distinct_rates = HashSet::new();
    for p in &all_paths {
        let r = p.targetInfo.refreshRate;
        if r.Denominator > 0 {
            let rounded = ((r.Numerator as f64) / (r.Denominator as f64) * 10.0).round() as i64;
            if rounded > 0 {
                distinct_rates.insert(rounded);
            }
        }
    }

    distinct_rates.len() > 1
}

fn fallback_monitor_info() -> MonitorInfo {
    MonitorInfo {
        resolution: "1920 x 1080".to_string(),
        refresh_hz: 60.0,
        supports_vrr: false,
    }
}

// -----------------------------------------------------------------------------
// Implementation: HID Devices
// -----------------------------------------------------------------------------

pub fn detect_hid_devices() -> (Vec<HidDeviceInfo>, Vec<HidDeviceInfo>) {
    let mut num_devices: u32 = 0;
    let cb_size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;

    unsafe {
        GetRawInputDeviceList(ptr::null_mut(), &mut num_devices, cb_size);
    }

    if num_devices == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut raw_list = vec![RAWINPUTDEVICELIST::default(); num_devices as usize];
    let count = unsafe {
        GetRawInputDeviceList(raw_list.as_mut_ptr(), &mut num_devices, cb_size)
    };

    if count == u32::MAX {
        return (Vec::new(), Vec::new());
    }

    let mut mice = Vec::new();
    let mut keyboards = Vec::new();
    let mut seen_paths = HashSet::new();
    let has_audio_contention = get_usb_topology_info()
        .map(|t| t.any_contention_detected)
        .unwrap_or(false);

    for dev in raw_list.iter().take(count as usize) {
        if dev.hDevice.is_null() {
            continue;
        }

        let mut name_len: u32 = 0;
        unsafe {
            GetRawInputDeviceInfoW(dev.hDevice, RIDI_DEVICENAME, ptr::null_mut(), &mut name_len);
        }

        if name_len == 0 {
            continue;
        }

        let mut name_buf = vec![0u16; name_len as usize];
        let read_len = unsafe {
            GetRawInputDeviceInfoW(
                dev.hDevice,
                RIDI_DEVICENAME,
                name_buf.as_mut_ptr() as *mut c_void,
                &mut name_len,
            )
        };

        if read_len == u32::MAX {
            continue;
        }

        let path = String::from_utf16_lossy(&name_buf)
            .trim_matches('\0')
            .trim()
            .to_string();

        if path.is_empty() {
            continue;
        }

        let base_id = clean_device_path(&path);
        if !seen_paths.insert(base_id) {
            continue;
        }

        let (vid, pid) = parse_vid_pid(&path);
        let transport = detect_transport(&path);

        match dev.dwType {
            RIM_TYPEMOUSE => {
                let friendly_name = friendly_mouse_name(&path, &vid, &transport);
                let (usb_port_speed, port_recommendation) =
                    detect_usb_port_speed_and_recommendation(&path, &transport, "mouse", has_audio_contention);
                mice.push(HidDeviceInfo {
                    name: friendly_name,
                    vid,
                    pid,
                    transport,
                    device_type: "mouse".to_string(),
                    raw_path: path,
                    usb_port_speed,
                    port_recommendation,
                });
            }
            RIM_TYPEKEYBOARD => {
                let friendly_name = friendly_keyboard_name(&path, &vid, &transport);
                let (usb_port_speed, port_recommendation) =
                    detect_usb_port_speed_and_recommendation(&path, &transport, "keyboard", false);
                keyboards.push(HidDeviceInfo {
                    name: friendly_name,
                    vid,
                    pid,
                    transport,
                    device_type: "keyboard".to_string(),
                    raw_path: path,
                    usb_port_speed,
                    port_recommendation,
                });
            }
            _ => {}
        }
    }

    (mice, keyboards)
}

fn clean_device_path(raw: &str) -> String {
    let upper = raw.to_uppercase();
    if let Some(pos) = upper.find("&COL") {
        return upper[..pos].to_string();
    }
    upper
}

fn parse_vid_pid(path: &str) -> (String, String) {
    let upper = path.to_uppercase();
    let vid = if let Some(pos) = upper.find("VID_") {
        upper[pos + 4..].chars().take(4).collect()
    } else {
        "N/A".to_string()
    };

    let pid = if let Some(pos) = upper.find("PID_") {
        upper[pos + 4..].chars().take(4).collect()
    } else {
        "N/A".to_string()
    };

    (vid, pid)
}

fn detect_transport(path: &str) -> String {
    let upper = path.to_uppercase();
    if upper.contains("BTH") || upper.contains("BLUETOOTH") {
        "Bluetooth".to_string()
    } else if upper.contains("USB") {
        "USB".to_string()
    } else {
        "HID genérico".to_string()
    }
}

fn friendly_mouse_name(path: &str, vid: &str, transport: &str) -> String {
    let upper = path.to_uppercase();
    let vendor = match vid {
        "046D" => "Logitech",
        "1532" => "Razer",
        "1B1C" => "Corsair",
        "1038" => "SteelSeries",
        "28DA" => "Finalmouse",
        "2BA2" => "Vaxee",
        "045E" => "Microsoft",
        "0B05" => "ASUS ROG",
        "258A" => "SinoWealth / Pulsar",
        _ => {
            if upper.contains("ELAN") {
                return "Touchpad / Ratón Elan".to_string();
            } else if upper.contains("SYNAPTICS") || upper.contains("SYN") {
                return "Touchpad / Ratón Synaptics".to_string();
            } else {
                "Ratón"
            }
        }
    };

    if vendor == "Ratón" {
        format!("Ratón ({transport})")
    } else {
        format!("{vendor} Gaming Mouse ({transport})")
    }
}

fn friendly_keyboard_name(path: &str, vid: &str, transport: &str) -> String {
    let upper = path.to_uppercase();
    if upper.contains("ACPI") {
        return "Teclado Interno del Sistema (ACPI)".to_string();
    }

    let vendor = match vid {
        "046D" => "Logitech",
        "1532" => "Razer",
        "1B1C" => "Corsair",
        "1038" => "SteelSeries",
        "045E" => "Microsoft",
        "0B05" => "ASUS",
        "04F2" => "Chicony",
        _ => "Teclado",
    };

    if vendor == "Teclado" {
        format!("Teclado ({transport})")
    } else {
        format!("{vendor} Keyboard ({transport})")
    }
}

pub fn is_hvci_enabled() -> bool {
    use crate::sys::registry::{read_value, RegistryData, RegistryHive, RegistryState};
    match read_value(
        RegistryHive::HKLM,
        "SYSTEM\\CurrentControlSet\\Control\\DeviceGuard\\Scenarios\\HypervisorEnforcedCodeIntegrity",
        "Enabled",
    ) {
        Ok(RegistryState::Present(RegistryData::Dword(val))) => val == 1,
        _ => false,
    }
}

fn detect_usb_port_speed_and_recommendation(
    path: &str,
    transport: &str,
    device_type: &str,
    has_audio_contention: bool,
) -> (String, Option<String>) {
    if transport != "USB" {
        let speed = if transport == "Bluetooth" {
            "Bluetooth (Inalámbrico)".to_string()
        } else {
            "N/A".to_string()
        };

        let recommendation = if device_type == "mouse" && transport == "Bluetooth" {
            Some("⚠️ Ratón conectado por Bluetooth (limitado a ~125 Hz). Usa conexión por cable o dongle inalámbrico 2.4 GHz en un puerto USB 3.0 para juego competitivo.".to_string())
        } else {
            None
        };

        return (speed, recommendation);
    }

    let mut resolved_speed = "USB (No determinado)".to_string();
    let mut is_superspeed = false;
    let mut is_high_or_full_speed = false;

    let mut clean_id = path.trim();
    if let Some(stripped) = clean_id.strip_prefix("\\\\?\\") {
        clean_id = stripped;
    } else if let Some(stripped) = clean_id.strip_prefix("\\\\.\\") {
        clean_id = stripped;
    }
    if let Some(pos) = clean_id.rfind("#{") {
        clean_id = &clean_id[..pos];
    }
    let instance_id = clean_id.replace('#', "\\");
    let instance_id_w: Vec<u16> = instance_id.encode_utf16().chain(std::iter::once(0)).collect();

    let mut hid_devnode: u32 = 0;
    let cr = unsafe { CM_Locate_DevNodeW(&mut hid_devnode, instance_id_w.as_ptr(), 0) };

    if cr == 0 && hid_devnode != 0 {
        let mut curr_node = hid_devnode;
        let mut root_hub_id = String::new();
        let mut parent_hub_id = String::new();
        let mut depth = 0;

        while depth < 8 {
            let mut parent_node: u32 = 0;
            let pr = unsafe { CM_Get_Parent(&mut parent_node, curr_node, 0) };
            if pr != 0 || parent_node == 0 {
                break;
            }

            let mut id_buf = [0u16; 512];
            let id_res = unsafe { CM_Get_Device_IDW(parent_node, id_buf.as_mut_ptr(), 512, 0) };
            if id_res == 0 {
                let p_id = String::from_utf16_lossy(&id_buf)
                    .trim_matches('\0')
                    .trim()
                    .to_string();
                let upper = p_id.to_uppercase();

                if parent_hub_id.is_empty() && (upper.contains("HUB") || upper.contains("USB")) {
                    parent_hub_id = p_id.clone();
                }
                if upper.starts_with("USB\\ROOT_HUB") || upper.starts_with("PCI\\") {
                    root_hub_id = p_id;
                    break;
                }
            }

            curr_node = parent_node;
            depth += 1;
        }

        let mut ioctl_success = false;
        if !parent_hub_id.is_empty() {
            let hub_dev_path = format!("\\\\.\\{}", parent_hub_id.replace('\\', "#"));
            let hub_path_w: Vec<u16> = hub_dev_path.encode_utf16().chain(std::iter::once(0)).collect();

            let h_hub = unsafe {
                CreateFileW(
                    hub_path_w.as_ptr(),
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    ptr::null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    ptr::null_mut(),
                )
            };

            if !h_hub.is_null() && h_hub != (-1isize as *mut c_void) {
                for port_idx in 1..=32 {
                    let mut info_v2 = USB_NODE_CONNECTION_INFORMATION_EX_V2 {
                        ConnectionIndex: port_idx,
                        Length: std::mem::size_of::<USB_NODE_CONNECTION_INFORMATION_EX_V2>() as u32,
                        SupportedUsbProtocols: USB_PROTOCOLS { ul: 0 },
                        Flags: USB_NODE_CONNECTION_INFORMATION_EX_V2_FLAGS { ul: 0 },
                    };
                    let mut bytes_ret = 0u32;
                    let ok_v2 = unsafe {
                        DeviceIoControl(
                            h_hub,
                            IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2,
                            &info_v2 as *const _ as *const c_void,
                            std::mem::size_of::<USB_NODE_CONNECTION_INFORMATION_EX_V2>() as u32,
                            &mut info_v2 as *mut _ as *mut c_void,
                            std::mem::size_of::<USB_NODE_CONNECTION_INFORMATION_EX_V2>() as u32,
                            &mut bytes_ret,
                            ptr::null_mut(),
                        )
                    };

                    if ok_v2 != 0 {
                        if (info_v2.SupportedUsbProtocols.ul & (1 << 2)) != 0 {
                            resolved_speed = "USB 3.0 (SuperSpeed)".to_string();
                            is_superspeed = true;
                            ioctl_success = true;
                            break;
                        } else if (info_v2.SupportedUsbProtocols.ul & (1 << 1)) != 0 {
                            resolved_speed = "USB 2.0 (High Speed)".to_string();
                            is_high_or_full_speed = true;
                            ioctl_success = true;
                            break;
                        }
                    }

                    let mut info_ex = USB_NODE_CONNECTION_INFORMATION_EX {
                        ConnectionIndex: port_idx,
                        DeviceDescriptor: USB_DEVICE_DESCRIPTOR::default(),
                        CurrentConfigurationValue: 0,
                        Speed: 0,
                        DeviceIsHub: 0,
                        DeviceAddress: 0,
                        NumberOfOpenPipes: 0,
                        ConnectionStatus: 0,
                    };
                    let ok_ex = unsafe {
                        DeviceIoControl(
                            h_hub,
                            IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX,
                            &info_ex as *const _ as *const c_void,
                            std::mem::size_of::<USB_NODE_CONNECTION_INFORMATION_EX>() as u32,
                            &mut info_ex as *mut _ as *mut c_void,
                            std::mem::size_of::<USB_NODE_CONNECTION_INFORMATION_EX>() as u32,
                            &mut bytes_ret,
                            ptr::null_mut(),
                        )
                    };

                    if ok_ex != 0 && info_ex.ConnectionStatus == 1 {
                        match info_ex.Speed {
                            3 | 4 => {
                                resolved_speed = "USB 3.0 (SuperSpeed)".to_string();
                                is_superspeed = true;
                                ioctl_success = true;
                                break;
                            }
                            2 => {
                                resolved_speed = "USB 2.0 (High Speed)".to_string();
                                is_high_or_full_speed = true;
                                ioctl_success = true;
                                break;
                            }
                            1 => {
                                resolved_speed = "USB 1.1 (Full Speed)".to_string();
                                is_high_or_full_speed = true;
                                ioctl_success = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                unsafe {
                    CloseHandle(h_hub);
                }
            }
        }

        if !ioctl_success {
            let combined = format!("{parent_hub_id} {root_hub_id}").to_uppercase();
            if combined.contains("ROOT_HUB30") || combined.contains("USB3") || combined.contains("XHCI") {
                resolved_speed = "USB 3.0 (SuperSpeed)".to_string();
                is_superspeed = true;
            } else if combined.contains("ROOT_HUB20") || combined.contains("USB2") || combined.contains("EHCI") {
                resolved_speed = "USB 2.0 (High Speed)".to_string();
                is_high_or_full_speed = true;
            } else if combined.contains("HUB") || combined.contains("USB") {
                resolved_speed = "USB 2.0 (High Speed)".to_string();
                is_high_or_full_speed = true;
            }
        }
    }

    let recommendation = if device_type == "mouse" {
        if is_high_or_full_speed {
            Some("⚠️ Tu ratón está en un puerto USB 2.0 (High Speed). Para evitar cuellos de botella con ratones de 1000 Hz o superior, conéctalo a un puerto USB 3.0 (SuperSpeed) trasero, directo a la placa base.".to_string())
        } else if is_superspeed {
            if has_audio_contention {
                Some("⚠️ Tu ratón está en un puerto USB 3.0, pero comparte controlador raíz con un dispositivo de audio. Muévelo a otro puerto directo de la placa base para evitar jitter.".to_string())
            } else {
                Some("✅ Tu ratón está en un puerto USB 3.0 dedicado. Configuración óptima.".to_string())
            }
        } else {
            None
        }
    } else {
        None
    };

    (resolved_speed, recommendation)
}

pub fn get_hardware_diagnostic_info() -> HardwareDiagnosticInfo {
    let gpu = detect_gpu_info();
    let monitor = detect_monitor_info();
    let (mice, keyboards) = detect_hid_devices();
    let hvci_enabled = is_hvci_enabled();

    HardwareDiagnosticInfo {
        gpu,
        monitor,
        mice,
        keyboards,
        hvci_enabled,
    }
}

// -----------------------------------------------------------------------------
/// Calcula un hash liviano (en < 0.05ms) del conjunto de dispositivos Raw Input (ratones/teclados).
/// Si un dispositivo se desconecta, se conecta o se cambia de puerto USB, este hash cambia.
pub fn get_hid_topology_signature() -> u64 {
    let mut num_devices: u32 = 0;
    let cb_size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;

    unsafe {
        GetRawInputDeviceList(ptr::null_mut(), &mut num_devices, cb_size);
    }

    if num_devices == 0 {
        return 0;
    }

    let mut raw_list = vec![RAWINPUTDEVICELIST::default(); num_devices as usize];
    let count = unsafe {
        GetRawInputDeviceList(raw_list.as_mut_ptr(), &mut num_devices, cb_size)
    };

    if count == u32::MAX {
        return 0;
    }

    let mut hasher = DefaultHasher::new();
    count.hash(&mut hasher);

    for dev in raw_list.iter().take(count as usize) {
        if dev.hDevice.is_null() {
            continue;
        }
        dev.dwType.hash(&mut hasher);

        let mut name_len: u32 = 0;
        unsafe {
            GetRawInputDeviceInfoW(dev.hDevice, RIDI_DEVICENAME, ptr::null_mut(), &mut name_len);
        }

        if name_len > 0 {
            let mut name_buf = vec![0u16; name_len as usize];
            let read_len = unsafe {
                GetRawInputDeviceInfoW(
                    dev.hDevice,
                    RIDI_DEVICENAME,
                    name_buf.as_mut_ptr() as *mut c_void,
                    &mut name_len,
                )
            };
            if read_len != u32::MAX {
                name_buf.hash(&mut hasher);
            }
        }
    }

    hasher.finish()
}

// -----------------------------------------------------------------------------
// Implementation: USB Topology & Root Hub Contention (Etapa 3.2.3)
// -----------------------------------------------------------------------------

pub fn get_usb_topology_info() -> AppResult<UsbTopologyInfo> {
    let usb_wide: Vec<u16> = "USB\0".encode_utf16().collect();
    let set = unsafe {
        SetupDiGetClassDevsW(
            ptr::null(),
            usb_wide.as_ptr(),
            ptr::null_mut(),
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    };

    if set.is_null() || set == (-1isize as *mut c_void) {
        return Err(AppError::HardwareDiag(
            "Fallo al enumerar dispositivos USB vía SetupAPI".to_string(),
        ));
    }

    struct DevInfoSetGuard(*mut c_void);
    impl Drop for DevInfoSetGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = SetupDiDestroyDeviceInfoList(self.0);
            }
        }
    }
    let _set_guard = DevInfoSetGuard(set);

    let mut groups: HashMap<String, Vec<UsbDeviceNode>> = HashMap::new();
    let mut dev_info = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ClassGuid: GUID::default(),
        DevInst: 0,
        Reserved: 0,
    };

    let mut idx = 0u32;
    let mut buf = [0u16; 512];

    loop {
        let enum_res = unsafe { SetupDiEnumDeviceInfo(set, idx, &mut dev_info) };
        if enum_res == 0 {
            break;
        }

        let mut id_buf = [0u16; 512];
        let id_res = unsafe { CM_Get_Device_IDW(dev_info.DevInst, id_buf.as_mut_ptr(), 512, 0) };
        let instance_id = if id_res == 0 {
            String::from_utf16_lossy(&id_buf)
                .trim_matches('\0')
                .trim()
                .to_string()
        } else {
            String::new()
        };

        // Query FriendlyName or DeviceDesc
        let mut reg_type = 0u32;
        let mut req_size = 0u32;
        let mut got_prop = unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                set,
                &dev_info,
                SPDRP_FRIENDLYNAME,
                &mut reg_type,
                buf.as_mut_ptr() as *mut u8,
                (buf.len() * 2) as u32,
                &mut req_size,
            )
        };

        if got_prop == 0 {
            got_prop = unsafe {
                SetupDiGetDeviceRegistryPropertyW(
                    set,
                    &dev_info,
                    SPDRP_DEVICEDESC,
                    &mut reg_type,
                    buf.as_mut_ptr() as *mut u8,
                    (buf.len() * 2) as u32,
                    &mut req_size,
                )
            };
        }

        let device_name = if got_prop != 0 {
            String::from_utf16_lossy(&buf)
                .trim_matches('\0')
                .trim()
                .to_string()
        } else {
            instance_id.clone()
        };

        // Climb parents until Host Controller / Root Hub
        let mut curr_inst = dev_info.DevInst;
        let mut root_hub_id = instance_id.clone();

        loop {
            let mut parent_inst = 0u32;
            let p_res = unsafe { CM_Get_Parent(&mut parent_inst, curr_inst, 0) };
            if p_res != 0 || parent_inst == 0 {
                break;
            }

            let mut p_buf = [0u16; 512];
            let p_id_res = unsafe { CM_Get_Device_IDW(parent_inst, p_buf.as_mut_ptr(), 512, 0) };
            if p_id_res == 0 {
                let p_id = String::from_utf16_lossy(&p_buf)
                    .trim_matches('\0')
                    .trim()
                    .to_string();

                let upper = p_id.to_uppercase();
                if upper.starts_with("USB\\ROOT_HUB") || upper.starts_with("PCI\\") {
                    root_hub_id = p_id;
                }
            }

            curr_inst = parent_inst;
        }

        let lower_name = device_name.to_lowercase();
        let lower_id = instance_id.to_lowercase();

        let is_mouse = lower_name.contains("mouse")
            || lower_name.contains("ratón")
            || lower_id.contains("mouse");

        let is_audio = lower_name.contains("audio")
            || lower_name.contains("sound")
            || lower_name.contains("headset")
            || lower_name.contains("auricular")
            || lower_name.contains("micrófono")
            || lower_name.contains("microphone")
            || lower_name.contains("dac");

        // Filter root hub self-references from list of children
        if !instance_id.to_uppercase().starts_with("USB\\ROOT_HUB") && !device_name.is_empty() {
            groups.entry(root_hub_id).or_default().push(UsbDeviceNode {
                device_name,
                instance_id,
                is_mouse,
                is_audio,
            });
        }

        idx += 1;
    }

    let mut controllers = Vec::new();
    let mut any_contention_detected = false;

    let mut group_keys: Vec<String> = groups.keys().cloned().collect();
    group_keys.sort();

    for (c_idx, ctrl_id) in group_keys.into_iter().enumerate() {
        let mut devices = groups.remove(&ctrl_id).unwrap_or_default();
        // Deduplicate devices by instance_id
        let mut seen = HashSet::new();
        devices.retain(|d| seen.insert(d.instance_id.clone()));

        let has_mouse = devices.iter().any(|d| d.is_mouse);
        let has_audio = devices.iter().any(|d| d.is_audio);

        let has_contention = has_mouse && has_audio;
        if has_contention {
            any_contention_detected = true;
        }

        let contention_details = if has_contention {
            Some(
                "Contención detectada: El mouse comparte controlador raíz USB con un dispositivo de audio. Las transferencias isócronas continuas de audio pueden causar micro-stuttering y fluctuación de jitter en el polling rate.".to_string(),
            )
        } else {
            None
        };

        let controller_name = format!("Controlador Raíz USB #{}", c_idx + 1);

        controllers.push(UsbHostControllerGroup {
            controller_id: ctrl_id,
            controller_name,
            devices,
            has_contention,
            contention_details,
        });
    }

    Ok(UsbTopologyInfo {
        controllers,
        any_contention_detected,
    })
}
