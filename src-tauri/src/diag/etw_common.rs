use core::ffi::c_void;
use std::ptr;
use std::thread::JoinHandle;
use windows::core::GUID;

pub const INVALID_PROCESSTRACE_HANDLE: u64 = !0u64;

pub const EVENT_TRACE_CONTROL_QUERY: u32 = 0;
pub const EVENT_TRACE_CONTROL_STOP: u32 = 1;
pub const EVENT_TRACE_CONTROL_UPDATE: u32 = 2;
pub const EVENT_TRACE_CONTROL_FLUSH: u32 = 3;

pub const EVENT_TRACE_REAL_TIME_MODE: u32 = 0x00000100;
pub const PROCESS_TRACE_MODE_REAL_TIME: u32 = 0x00000100;
pub const PROCESS_TRACE_MODE_EVENT_RECORD: u32 = 0x10000000;

pub const WNODE_FLAG_TRACED_GUID: u32 = 0x00020000;

pub const EVENT_CONTROL_CODE_DISABLE_PROVIDER: u32 = 0;
pub const EVENT_CONTROL_CODE_ENABLE_PROVIDER: u32 = 1;

pub const TRACE_LEVEL_INFORMATION: u8 = 4;
pub const TRACE_LEVEL_VERBOSE: u8 = 5;

// Kernel Logger flags for DPC / Interrupt / Driver
pub const EVENT_TRACE_FLAG_DPC: u32 = 0x00000020;
pub const EVENT_TRACE_FLAG_INTERRUPT: u32 = 0x00000040;
pub const EVENT_TRACE_FLAG_DRIVER: u32 = 0x00800000;

pub const KERNEL_LOGGER_NAME: &str = "NT Kernel Logger";

pub const SYSTEM_TRACE_CONTROL_GUID: GUID = GUID {
    data1: 0x9e814aad,
    data2: 0x3204,
    data3: 0x11d2,
    data4: [0x9a, 0x82, 0x00, 0x60, 0x08, 0xa8, 0x69, 0x39],
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WNODE_HEADER {
    pub BufferSize: u32,
    pub ProviderId: u32,
    pub HistoricalContext: u64,
    pub KernelHandle: u64,
    pub Guid: GUID,
    pub ClientContext: u32,
    pub Flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EVENT_TRACE_PROPERTIES {
    pub Wnode: WNODE_HEADER,
    pub BufferSize: u32,
    pub MinimumBuffers: u32,
    pub MaximumBuffers: u32,
    pub MaximumFileSize: u32,
    pub LogFileMode: u32,
    pub FlushTimer: u32,
    pub EnableFlags: u32,
    pub AgeLimit: i32,
    pub NumberOfBuffers: u32,
    pub FreeBuffers: u32,
    pub EventsLost: u32,
    pub BuffersLost: u32,
    pub LogBuffersLost: u32,
    pub RealTimeBuffersLost: u32,
    pub LoggerThreadId: *mut c_void,
    pub LogFileNameOffset: u32,
    pub LoggerNameOffset: u32,
}

pub const ETW_PROPS_BUFFER_SIZE: usize =
    std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 2048;

pub struct TracePropertiesBuffer {
    pub raw: Vec<u8>,
}

impl TracePropertiesBuffer {
    pub fn new(session_name: &str) -> Self {
        let mut raw = vec![0u8; ETW_PROPS_BUFFER_SIZE];
        let p_props = raw.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        let name_offset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();

        unsafe {
            (*p_props).Wnode.BufferSize = ETW_PROPS_BUFFER_SIZE as u32;
            (*p_props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
            (*p_props).Wnode.ClientContext = 1; // 1 = QPC
            (*p_props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
            (*p_props).LoggerNameOffset = name_offset as u32;
            (*p_props).LogFileNameOffset = 0;

            let wide_name: Vec<u16> =
                session_name.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_len = wide_name.len() * 2;
            if name_offset + byte_len <= ETW_PROPS_BUFFER_SIZE {
                ptr::copy_nonoverlapping(
                    wide_name.as_ptr() as *const u8,
                    raw.as_mut_ptr().add(name_offset),
                    byte_len,
                );
            }
        }

        Self { raw }
    }

    pub fn as_mut_ptr(&mut self) -> *mut EVENT_TRACE_PROPERTIES {
        self.raw.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TIME_ZONE_INFORMATION {
    pub Bias: i32,
    pub StandardName: [u16; 32],
    pub StandardDate: [u16; 8],
    pub StandardBias: i32,
    pub DaylightName: [u16; 32],
    pub DaylightDate: [u16; 8],
    pub DaylightBias: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TRACE_LOGFILE_HEADER {
    pub BufferSize: u32,
    pub Version: u32,
    pub ProviderVersion: u32,
    pub NumberOfProcessors: u32,
    pub EndTime: i64,
    pub TimerResolution: u32,
    pub MaximumFileSize: u32,
    pub LogFileMode: u32,
    pub BuffersWritten: u32,
    pub StartBuffers: u32,
    pub PointerSize: u32,
    pub EventsLost: u32,
    pub CpuSpeedInMHz: u32,
    pub LoggerName: *mut u16,
    pub LogFileName: *mut u16,
    pub TimeZone: TIME_ZONE_INFORMATION,
    pub BootTime: i64,
    pub PerfFreq: i64,
    pub StartTime: i64,
    pub ReservedFlags: u32,
    pub BuffersLost: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EVENT_TRACE_HEADER {
    pub Size: u16,
    pub FieldTypeFlags: u16,
    pub Version: u32,
    pub ThreadId: u32,
    pub ProcessId: u32,
    pub TimeStamp: i64,
    pub Guid: GUID,
    pub KernelTime: u32,
    pub UserTime: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ETW_BUFFER_CONTEXT {
    pub ProcessorNumber: u8,
    pub Alignment: u8,
    pub LoggerId: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EVENT_TRACE {
    pub Header: EVENT_TRACE_HEADER,
    pub InstanceId: u32,
    pub ParentInstanceId: u32,
    pub ParentGuid: GUID,
    pub MofData: *mut c_void,
    pub MofLength: u32,
    pub BufferContext: ETW_BUFFER_CONTEXT,
}

#[repr(C)]
pub struct EVENT_DESCRIPTOR {
    pub Id: u16,
    pub Version: u8,
    pub Channel: u8,
    pub Level: u8,
    pub Opcode: u8,
    pub Task: u16,
    pub Keyword: u64,
}

#[repr(C)]
pub struct EVENT_HEADER {
    pub Size: u16,
    pub HeaderType: u16,
    pub Flags: u16,
    pub EventProperty: u16,
    pub ThreadId: u32,
    pub ProcessId: u32,
    pub TimeStamp: i64,
    pub ProviderId: GUID,
    pub EventDescriptor: EVENT_DESCRIPTOR,
    pub KernelTime: u32,
    pub UserTime: u32,
    pub ActivityId: GUID,
}

#[repr(C)]
pub struct EVENT_HEADER_EXTENDED_DATA_ITEM {
    pub Reserved1: u16,
    pub ExtType: u16,
    pub Linkage: u16,
    pub DataSize: u16,
    pub DataPtr: u64,
}

#[repr(C)]
pub struct EVENT_RECORD {
    pub EventHeader: EVENT_HEADER,
    pub BufferContext: ETW_BUFFER_CONTEXT,
    pub ExtendedDataCount: u16,
    pub UserDataLength: u16,
    pub ExtendedData: *mut EVENT_HEADER_EXTENDED_DATA_ITEM,
    pub UserData: *mut c_void,
    pub UserContext: *mut c_void,
}

#[repr(C)]
pub struct EVENT_TRACE_LOGFILEW {
    pub LogFileName: *mut u16,
    pub LoggerName: *mut u16,
    pub CurrentTime: i64,
    pub BuffersRead: u32,
    pub ProcessTraceMode: u32,
    pub CurrentEvent: EVENT_TRACE,
    pub LogfileHeader: TRACE_LOGFILE_HEADER,
    pub BufferCallback: Option<unsafe extern "system" fn(*mut EVENT_TRACE_LOGFILEW) -> u32>,
    pub BufferSize: u32,
    pub Filled: u32,
    pub EventsLost: u32,
    pub EventRecordCallback: Option<unsafe extern "system" fn(*mut EVENT_RECORD)>,
    pub IsKernelTrace: u32,
    pub Context: *mut c_void,
}

#[link(name = "advapi32")]
extern "system" {
    pub fn StartTraceW(
        trace_handle: *mut u64,
        instance_name: *const u16,
        properties: *mut EVENT_TRACE_PROPERTIES,
    ) -> u32;

    pub fn ControlTraceW(
        trace_handle: u64,
        instance_name: *const u16,
        properties: *mut EVENT_TRACE_PROPERTIES,
        control_code: u32,
    ) -> u32;

    pub fn EnableTraceEx2(
        trace_handle: u64,
        provider_id: *const GUID,
        control_code: u32,
        level: u8,
        match_any_keyword: u64,
        match_all_keyword: u64,
        timeout: u32,
        enable_parameters: *const c_void,
    ) -> u32;

    pub fn OpenTraceW(logfile: *mut EVENT_TRACE_LOGFILEW) -> u64;

    pub fn ProcessTrace(
        handle_array: *const u64,
        handle_count: u32,
        start_time: *const c_void,
        end_time: *const c_void,
    ) -> u32;

    pub fn CloseTrace(trace_handle: u64) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn QueryPerformanceCounter(lp_performance_count: *mut i64) -> i32;
    pub fn QueryPerformanceFrequency(lp_frequency: *mut i64) -> i32;
}

#[link(name = "psapi")]
extern "system" {
    pub fn EnumDeviceDrivers(
        lp_image_base: *mut *mut c_void,
        cb: u32,
        lpcb_needed: *mut u32,
    ) -> i32;

    pub fn GetDeviceDriverBaseNameW(
        image_base: *mut c_void,
        lp_filename: *mut u16,
        n_size: u32,
    ) -> u32;
}

/// Closes any leftover session with the same name before starting a new trace.
pub fn stop_stale_session(session_name: &str) {
    let wide: Vec<u16> =
        session_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf = vec![0u8; ETW_PROPS_BUFFER_SIZE];
    let p_props = buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
    unsafe {
        (*p_props).Wnode.BufferSize = ETW_PROPS_BUFFER_SIZE as u32;
        (*p_props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*p_props).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        let _ = ControlTraceW(0, wide.as_ptr(), p_props, EVENT_TRACE_CONTROL_STOP);
    }
}

/// RAII Guard ensuring ETW trace sessions, consumer handles, and worker threads
/// are always stopped, closed, and joined cleanly — even if a panic unwind occurs.
pub struct EtwSessionGuard {
    session_handle: u64,
    session_name_wide: Vec<u16>,
    consumer_handle: u64,
    worker_thread: Option<JoinHandle<()>>,
    is_stopped: bool,
}

impl EtwSessionGuard {
    pub fn new(session_handle: u64, session_name: &str) -> Self {
        let wide: Vec<u16> =
            session_name.encode_utf16().chain(std::iter::once(0)).collect();
        Self {
            session_handle,
            session_name_wide: wide,
            consumer_handle: 0,
            worker_thread: None,
            is_stopped: false,
        }
    }

    pub fn attach_consumer(&mut self, consumer_handle: u64, worker_thread: JoinHandle<()>) {
        self.consumer_handle = consumer_handle;
        self.worker_thread = Some(worker_thread);
    }

    pub fn stop_and_join(&mut self) {
        if !self.is_stopped {
            self.is_stopped = true;

            // 1. Close consumer trace handle so ProcessTrace unblocks
            if self.consumer_handle != 0 && self.consumer_handle != INVALID_PROCESSTRACE_HANDLE {
                unsafe {
                    let _ = CloseTrace(self.consumer_handle);
                }
            }

            // 2. Stop ETW session via handle + fallback by session name
            let mut stop_buf = vec![0u8; ETW_PROPS_BUFFER_SIZE];
            let p_props = stop_buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
            unsafe {
                (*p_props).Wnode.BufferSize = ETW_PROPS_BUFFER_SIZE as u32;
                (*p_props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
                (*p_props).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

                if self.session_handle != 0 {
                    let _ = ControlTraceW(
                        self.session_handle,
                        self.session_name_wide.as_ptr(),
                        p_props,
                        EVENT_TRACE_CONTROL_STOP,
                    );
                }

                let _ = ControlTraceW(
                    0,
                    self.session_name_wide.as_ptr(),
                    p_props,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
        }

        // 3. Join worker thread BEFORE returning so EventRecordCallback can never touch freed memory
        if let Some(worker) = self.worker_thread.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for EtwSessionGuard {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}
