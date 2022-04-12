use windows::{
	core::*,
    Win32::System::Diagnostics::Etw::*
};
pub use strum::*;


pub const EVENTS_DESC: &'static[EventsDescribe] = &[
	EventsDescribe{
		major: MajorDescribe{name: "Process", flag: Major::Process as u64},
		minors: &[
			MinorDescribe{name: "Process Start", op_code: 1},
			MinorDescribe{name: "Process End", op_code: 2},
			MinorDescribe{name: "Process Terminate", op_code: 11},
			MinorDescribe{name: "Data Collection Start", op_code: 3},
			MinorDescribe{name: "Data Collection Stop", op_code: 4},
			MinorDescribe{name: "Defunct", op_code: 39},
			MinorDescribe{name: "Perf Counter", op_code: 32},
			MinorDescribe{name: "Perf Counter Rundown", op_code: 33}
		],
		guid: PROCESS_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Thread", flag: Major::Thread as u64},
		minors: &[
			MinorDescribe{name: "SendMessage", op_code: 0x21},
			MinorDescribe{name: "ReceiveMessage", op_code: 0x22},
			MinorDescribe{name: "WaitForReply", op_code: 0x23},
			MinorDescribe{name: "WaitForNewMessage", op_code: 0x24},
			MinorDescribe{name: "Unwait", op_code: 0x25},
			MinorDescribe{name: "ConnectRequest", op_code: 0x26},
			MinorDescribe{name: "ConnectSuccess", op_code: 0x27},
			MinorDescribe{name: "ConnectFail", op_code: 0x28},
			MinorDescribe{name: "ClosePort", op_code: 0x29}
		],
		guid: THREAD_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "ImageLoad", flag: Major::ImageLoad as u64},
		minors: &[
			MinorDescribe{name: "Load", op_code: 10},
			MinorDescribe{name: "Unload", op_code: 2},
			MinorDescribe{name: "Relocation", op_code: 0x20},
			MinorDescribe{name: "Data Collection Start", op_code: 3},
			MinorDescribe{name: "Data Collection Stop", op_code: 4},
			MinorDescribe{name: "Kernel Base", op_code: 0x21},
			MinorDescribe{name: "Hypercall Page", op_code: 0x22}
		],
		guid: IMAGE_LOAD_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Registry", flag: Major::Registry as u64},
		minors: &[
			MinorDescribe{name: "Create Key", op_code: EVENT_TRACE_TYPE_REGCREATE},
			MinorDescribe{name: "Open Key", op_code: EVENT_TRACE_TYPE_REGOPEN},
			MinorDescribe{name: "Delete Key", op_code: EVENT_TRACE_TYPE_REGDELETE},
			MinorDescribe{name: "Query Key", op_code: EVENT_TRACE_TYPE_REGQUERY},
			MinorDescribe{name: "Set Value", op_code: EVENT_TRACE_TYPE_REGSETVALUE},
			MinorDescribe{name: "Delete Value", op_code: EVENT_TRACE_TYPE_REGDELETEVALUE},
			MinorDescribe{name: "Query Value", op_code: EVENT_TRACE_TYPE_REGQUERYVALUE},
			MinorDescribe{name: "Enum Key", op_code: EVENT_TRACE_TYPE_REGENUMERATEKEY},
			MinorDescribe{name: "Enum Value", op_code: EVENT_TRACE_TYPE_REGENUMERATEVALUEKEY},
			MinorDescribe{name: "Query Multiple Values", op_code: EVENT_TRACE_TYPE_REGQUERYMULTIPLEVALUE},
			MinorDescribe{name: "Set Key Information", op_code: EVENT_TRACE_TYPE_REGSETINFORMATION},
			MinorDescribe{name: "Flush Key", op_code: EVENT_TRACE_TYPE_REGFLUSH},
			MinorDescribe{name: "KCB Create", op_code: EVENT_TRACE_TYPE_REGKCBCREATE},
			MinorDescribe{name: "KCB Delete", op_code: EVENT_TRACE_TYPE_REGKCBDELETE},
			MinorDescribe{name: "KCB Rundown Begin", op_code: EVENT_TRACE_TYPE_REGKCBRUNDOWNBEGIN},
			MinorDescribe{name: "KCB Rundown End", op_code: EVENT_TRACE_TYPE_REGKCBRUNDOWNEND},
			MinorDescribe{name: "Virtualize Key", op_code: EVENT_TRACE_TYPE_REGVIRTUALIZE},
			MinorDescribe{name: "Close Key", op_code: EVENT_TRACE_TYPE_REGCLOSE},
			MinorDescribe{name: "Set Security Descriptor", op_code: EVENT_TRACE_TYPE_REGSETSECURITY},
			MinorDescribe{name: "Query Security Descriptor", op_code: EVENT_TRACE_TYPE_REGQUERYSECURITY},
			MinorDescribe{name: "Commit Tx", op_code: EVENT_TRACE_TYPE_REGCOMMIT},
			MinorDescribe{name: "Prepare Tx", op_code: EVENT_TRACE_TYPE_REGPREPARE},
			MinorDescribe{name: "Rollback Tx", op_code: EVENT_TRACE_TYPE_REGROLLBACK},
			MinorDescribe{name: "Load Key", op_code: EVENT_TRACE_TYPE_REGMOUNTHIVE}
		],
		guid: REGISTRY_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "FileIO", flag: Major::FileIO as u64},
		minors: &[
			MinorDescribe{name: "Name", op_code: 0},
			MinorDescribe{name: "File Create", op_code: 32},
			MinorDescribe{name: "File Delete", op_code: 35},
			MinorDescribe{name: "File Rundown", op_code: 36},
			MinorDescribe{name: "Create", op_code: 64},
			MinorDescribe{name: "Dir Enum", op_code: 72},
			MinorDescribe{name: "Dir Notify", op_code: 77},
			MinorDescribe{name: "Set Info", op_code: 69},
			MinorDescribe{name: "Delete", op_code: 70},
			MinorDescribe{name: "Rename", op_code: 71},
			MinorDescribe{name: "Query Info", op_code: 74},
			MinorDescribe{name: "FS Control", op_code: 75},
			MinorDescribe{name: "Operation End", op_code: 76},
			MinorDescribe{name: "Read", op_code: 67},
			MinorDescribe{name: "Write", op_code: 68},
			MinorDescribe{name: "Cleanup", op_code: 65},
			MinorDescribe{name: "Close", op_code: 66},
			MinorDescribe{name: "Flush", op_code: 73},
		],
		guid: FILE_IO_GUID
	},

	EventsDescribe{
		major: MajorDescribe{name: "DiskIO", flag: Major::DiskIO as u64},
		minors: &[
			MinorDescribe{name: "Read", op_code: 10},
			MinorDescribe{name: "Write", op_code: 11},
			MinorDescribe{name: "Read Init", op_code: 12},
			MinorDescribe{name: "Write Init", op_code: 13},
			MinorDescribe{name: "Flush Init", op_code: 15},
			MinorDescribe{name: "Flush Buffers", op_code: 14}
		],
		guid: DISK_IO_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PageFaults", flag: Major::PageFaults as u64},
		minors: &[
			MinorDescribe{name: "Hard Fault", op_code: 0x20},
			MinorDescribe{name: "Transition Fault", op_code: 10},
			MinorDescribe{name: "Demand Zero Fault", op_code: 11},
			MinorDescribe{name: "Copy on Write", op_code: 12},
			MinorDescribe{name: "Guard Page Fault", op_code: 13},
			MinorDescribe{name: "Hard Page Fault", op_code: 14},
			MinorDescribe{name: "Access Violation", op_code: 15},
			MinorDescribe{name: "Image Load Backed", op_code: 105}
		],
		guid: PAGE_FAULT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Tcp", flag: Major::Network as u64},
		minors: &[
			MinorDescribe{name: "TCP Send IPv4", op_code: EVENT_TRACE_TYPE_SEND},
			MinorDescribe{name: "TCP Receive IPv4", op_code: EVENT_TRACE_TYPE_RECEIVE},
			MinorDescribe{name: "TCP Connect IPv4", op_code: EVENT_TRACE_TYPE_CONNECT},
			MinorDescribe{name: "TCP Disconnect IPv4", op_code: EVENT_TRACE_TYPE_DISCONNECT},
			MinorDescribe{name: "TCP Retransmit IPv4", op_code: EVENT_TRACE_TYPE_RETRANSMIT},
			MinorDescribe{name: "TCP Accept IPv4", op_code: EVENT_TRACE_TYPE_ACCEPT},
			MinorDescribe{name: "TCP Reconnect IPv4", op_code: EVENT_TRACE_TYPE_RECONNECT},
			MinorDescribe{name: "TCP Fail", op_code: EVENT_TRACE_TYPE_CONNFAIL},
			MinorDescribe{name: "TCP Copy IPv4", op_code: 18},
			MinorDescribe{name: "TCP Send IPv6", op_code: 26},
			MinorDescribe{name: "TCP Receive IPv6", op_code: 27},
			MinorDescribe{name: "TCP Disconnect IPv6", op_code: 29},
			MinorDescribe{name: "TCP Retransmit IPv6", op_code: 30},
			MinorDescribe{name: "TCP Reconnect IPv6", op_code: 32},
			MinorDescribe{name: "TCP Copy IPv6", op_code: 34},
			MinorDescribe{name: "TCP Connect IPv6", op_code: 28},
			MinorDescribe{name: "TCP Accept IPv6", op_code: 31},
			MinorDescribe{name: "Copy ARP", op_code: EVENT_TRACE_TYPE_COPY_ARP},
			MinorDescribe{name: "Full Ack", op_code: EVENT_TRACE_TYPE_ACKFULL},
			MinorDescribe{name: "Partial Ack", op_code: EVENT_TRACE_TYPE_ACKPART},
			MinorDescribe{name: "Duplicate Ack", op_code: EVENT_TRACE_TYPE_ACKDUP}
		],
		guid: TCP_IP_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Udp", flag: Major::Network as u64},
		minors: &[
			MinorDescribe{name: "UDP Send IPv4", op_code: 10},
			MinorDescribe{name: "UDP Receive IPv4", op_code: 11},
			MinorDescribe{name: "UDP Send IPv6", op_code: 26},
			MinorDescribe{name: "UDP Receive IPv6", op_code: 27},
			MinorDescribe{name: "UDP Send IPv4", op_code: 10},
			MinorDescribe{name: "UDP Receive IPv4", op_code: 11},
			MinorDescribe{name: "UDP Send IPv6", op_code: 26},
			MinorDescribe{name: "UDP Receive IPv6", op_code: 27}
		],
		guid: UDP_IP_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "DebugPrint", flag: Major::DebugPrint as u64},
		minors: &[
			MinorDescribe{name: "Debug Print", op_code: 0x20}
		],
		guid: DBG_PRINT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PerfHandles", flag: Major::PerfHandles as u64},
		minors: &[
			MinorDescribe{name: "Create Handle", op_code: 32},
			MinorDescribe{name: "Close Handle", op_code: 33},
			MinorDescribe{name: "Duplicate Handle", op_code: 34}
		],
		guid: OBJECT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PerfObjects", flag: Major::PerfObjects as u64},
		minors: &[
			MinorDescribe{name: "Create Object", op_code: 48},
			MinorDescribe{name: "Delete Object", op_code: 49},
			MinorDescribe{name: "Reference Object", op_code: 50},
			MinorDescribe{name: "Dereference Object", op_code: 51}
		],
		guid: OBJECT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PerfObjects", flag: Major::PerfObjects as u64},
		minors: &[
			MinorDescribe{name: "Create Object", op_code: 48},
			MinorDescribe{name: "Delete Object", op_code: 49},
			MinorDescribe{name: "Reference Object", op_code: 50},
			MinorDescribe{name: "Dereference Object", op_code: 51}
		],
		guid: OBJECT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PerfPool", flag: Major::PerfPool as u64},
		minors: &[
			MinorDescribe{name: "Pool Alloc", op_code: 0x20},
			MinorDescribe{name: "Pool Session Alloc", op_code: 0x21},
			MinorDescribe{name: "Pool Free", op_code: 0x22},
			MinorDescribe{name: "Pool (Session) Free", op_code: 0x23},
			MinorDescribe{name: "Add Pool Page", op_code: 0x24},
			MinorDescribe{name: "Add Session Pool Page", op_code: 0x25},
			MinorDescribe{name: "Big Pool Page", op_code: 0x26},
			MinorDescribe{name: "Big Session Pool Page", op_code: 0x27}
		],
		guid: POOL_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "VirtualMemory", flag: Major::VirtualMemory as u64},
		minors: &[
			MinorDescribe{name: "Virtual Alloc", op_code: 98},
			MinorDescribe{name: "Virtual Free", op_code: 99}
		],
		guid: PAGE_FAULT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PerfHeap", flag: Major::PerfHeap as u64},
		minors: &[
			MinorDescribe{name: "Heap Create", op_code: 0x20},
			MinorDescribe{name: "Heap Alloc", op_code: 0x21},
			MinorDescribe{name: "Heap ReAlloc", op_code: 0x22},
			MinorDescribe{name: "Heap Destroy", op_code: 0x22},
			MinorDescribe{name: "Heap Free", op_code: 0x24},
			MinorDescribe{name: "Heap Extend", op_code: 0x25},
			MinorDescribe{name: "Heap Snapshot", op_code: 0x26},
			MinorDescribe{name: "Heap Create Snapshot", op_code: 0x27},
			MinorDescribe{name: "Heap Destroy Snapshot", op_code: 0x28},
			MinorDescribe{name: "Heap Extend Snapshot", op_code: 0x29},
			MinorDescribe{name: "Heap Contract", op_code: 0x2a},
			MinorDescribe{name: "Heap Lock", op_code: 0x2b},
			MinorDescribe{name: "Heap Unlock", op_code: 0x2c},
			MinorDescribe{name: "Heap Validate", op_code: 0x2d},
			MinorDescribe{name: "Heap Walk", op_code: 0x2e}
		],
		guid: HEAP_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Job", flag: Major::Job as u64},
		minors: &[
			MinorDescribe{name: "Create", op_code: 0x20},
			MinorDescribe{name: "Terminate", op_code: 0x21},
			MinorDescribe{name: "Open", op_code: 0x22},
			MinorDescribe{name: "Assign Process", op_code: 0x23},
			MinorDescribe{name: "Remove Process", op_code: 0x24},
			MinorDescribe{name: "Set", op_code: 0x25},
			MinorDescribe{name: "Query", op_code: 0x26}
		],
		guid: JOB_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "WorkerThread", flag: Major::WorkerThread as u64},
		minors: &[
			MinorDescribe{name: "Create", op_code: 1},
			MinorDescribe{name: "Delete", op_code: 2}
		],
		guid: THREAD_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "PerfFlt", flag: Major::PerfFlt as u64},
		minors: &[
			MinorDescribe{name: "Pre Operation Init", op_code: 0x60},
			MinorDescribe{name: "Post Operation Init", op_code: 0x61},
			MinorDescribe{name: "Pre Operation Completion", op_code: 0x62},
			MinorDescribe{name: "Post Operation Completion", op_code: 0x63},
			MinorDescribe{name: "Pre Operation Failure", op_code: 0x64},
			MinorDescribe{name: "Post Operation Failure", op_code: 0x65}
		],
		guid: FILE_IO_GUID
	},
];

pub struct EventsDescribe{
	pub major: MajorDescribe,
	pub minors: &'static [MinorDescribe],
	pub guid: GUID

}
pub struct MajorDescribe {
	pub name: &'static str,
	pub flag: u64
}

pub struct MinorDescribe {
	pub name: &'static str,
	pub op_code: u32
}


#[derive(Clone, Copy, EnumIter, FromRepr, AsRefStr)]
#[repr(u64)]
pub enum Major{
    None,

    Process =               EVENT_TRACE_FLAG_PROCESS.0 as u64,
    Thread =	            EVENT_TRACE_FLAG_THREAD.0 as u64,
	ImageLoad =			    EVENT_TRACE_FLAG_IMAGE_LOAD.0  as u64,
	Registry =			    EVENT_TRACE_FLAG_REGISTRY.0 as u64,
	DiskIO =			    EVENT_TRACE_FLAG_DISK_IO.0 as u64,
	DiskFileIO =		    EVENT_TRACE_FLAG_DISK_IO.0 as u64 | EVENT_TRACE_FLAG_DISK_FILE_IO.0 as u64,
	PageFaults =		    EVENT_TRACE_FLAG_MEMORY_PAGE_FAULTS.0 as u64,
	HardPageFaults =	    EVENT_TRACE_FLAG_MEMORY_HARD_FAULTS.0 as u64,
	Network =			    EVENT_TRACE_FLAG_NETWORK_TCPIP.0 as u64,
	DebugPrint =		    EVENT_TRACE_FLAG_DBGPRINT.0 as u64,

	ProcessCounters =	    EVENT_TRACE_FLAG_PROCESS_COUNTERS.0 as u64,
	ContextSwitch =		    EVENT_TRACE_FLAG_CSWITCH.0 as u64,
	DPC =				    EVENT_TRACE_FLAG_DPC.0 as u64,
	Interrupt =			    EVENT_TRACE_FLAG_INTERRUPT.0 as u64,
	SystemCall =		    EVENT_TRACE_FLAG_SYSTEMCALL.0 as u64,
	DiskIoInit =		    EVENT_TRACE_FLAG_DISK_IO_INIT.0 as u64,
	ALPC =				    EVENT_TRACE_FLAG_ALPC.0 as u64,
	SplitIO =			    EVENT_TRACE_FLAG_SPLIT_IO.0 as u64,
	Driver =			    EVENT_TRACE_FLAG_DRIVER.0 as u64,
	Profile =			    EVENT_TRACE_FLAG_PROFILE.0 as u64,
    FileIOInit =		    EVENT_TRACE_FLAG_FILE_IO_INIT.0 as u64,
	FileIO =			    EVENT_TRACE_FLAG_FILE_IO_INIT.0 as u64 | EVENT_TRACE_FLAG_FILE_IO.0 as u64,

	Dispatcher =		    EVENT_TRACE_FLAG_DISPATCHER.0 as u64,
	VirtualAlloc =		    EVENT_TRACE_FLAG_VIRTUAL_ALLOC.0 as u64,
	VAMap =				    EVENT_TRACE_FLAG_VAMAP.0 as u64,
	VirtualMemory =		    EVENT_TRACE_FLAG_VIRTUAL_ALLOC.0 as u64 | EVENT_TRACE_FLAG_VAMAP.0 as u64,
	NoSysConfig =		    EVENT_TRACE_FLAG_NO_SYSCONFIG.0 as u64,

	Job =				    EVENT_TRACE_FLAG_JOB.0 as u64,
	Debug =				    EVENT_TRACE_FLAG_DEBUG_EVENTS as u64,

	// Mask[1]
	PerfMemory =			0x20000001 | (1u64 << 32),
	PerfProfile =			0x20000002 | (1u64 << 32),
	PerfContextSwitch =		0x20000004 | (1u64 << 32),
	PerfDrivers =			0x20000010 | (1u64 << 32),
	PerfPool =				0x20000040 | (1u64 << 32),
	PerfSyncObjects =		0x20020000 | (1u64 << 32),
	PerfVirtualAu64oc =		0x20008000 | (1u64 << 32),
	PerfSession =			0x20400000 | (1u64 << 32),
	PerfMemInfo =			0x20080000 | (1u64 << 32),

	// Mask[2]
	PerfHeap =			0x40000020 | (2u64 << 32),
	PerfSysCalls =		0x40000040 | (2u64 << 32),
	WorkerThread =		0x48000000 | (2u64 << 32),
	ProcessFreeze =		0x40000002 | (2u64 << 32),
	PerfEvents =		0x40000800 | (2u64 << 32),
	PerfWSDetail =		0x40000008 | (2u64 << 32),
	PerfTimer =			0x40020000 | (2u64 << 32),
	PerfIPI =			0x40400000 | (2u64 << 32),
	PerfClockIntr =		0x40040000 | (2u64 << 32),
	Mask2All =			0x4fffffff | (2u64 << 32),

	// Mask[4]
	PerfHandles =		0x80000040 | (4u64 << 32),
	PerfObjects =		0x80000080 | (4u64 << 32),
	PerfDebugger =		0x80000800 | (4u64 << 32),
	PerfPower =			0x80008000 | (4u64 << 32),
	PerfDllInfo =		0x80000008 | (4u64 << 32),
	PerfFltIoInit =		0x80080000 | (4u64 << 32),
	PerfFltIO =			0x80100000 | (4u64 << 32),
	PerfFltFastIO =		0x80200000 | (4u64 << 32),
	PerfFltFail =		0x80400000 | (4u64 << 32),
	PerfFlt =			Major::PerfFltIoInit as u64 | Major::PerfFltIO as u64 | Major::PerfFltFastIO as u64 | Major::PerfFltFail as u64,
	PerfHvProfile =		0x80800000 | (4u64 << 32),
	Mask4All =			0x8fffffff | (4u64 << 32),

	// Mask[6]
	PerfConfigSystem =	0xC0000001 | (6u64 << 32),
	PerfConfigGraphics = 0xC0000002 | (6u64 << 32),
}

#[derive(Clone, Copy, EnumIter, FromRepr, IntoStaticStr)]
#[repr(u32)]
pub enum MinorProcess{
	None,
    Start = 1,
    End = 2,
    Terminate = 11,
    DcStart = 3,
    DcStop = 4,
    PerfCounter = 32,   
    PerfCounterRundown = 33,
    Defunct = 39
}

#[derive(Clone, Copy, EnumIter, FromRepr, IntoStaticStr)]
#[repr(u32)]
pub enum MinorThread{
	None,
    SendMessage =  0x21,
    ReceiveMessage = 0x22,
    WaitForReply = 0x23,
    WaitForNewMessage = 0x24,
    Unwait = 0x25,
    ConnectRequest = 0x26,
    ConnectSuccess = 0x27,
    ConnectFail = 0x28,
    ClosePort = 0x29,
}

/* 54849625-5478-4994-a5ba-3e3b0328c30d */
const SECURITY_PROVIDER_GUID: GUID = GUID{ data1: 0x54849625, data2: 0x5478, data3: 0x4994, data4: [0xa5, 0xba, 0x3e, 0x3b, 0x03, 0x28, 0xc3, 0x0d] };

/* 3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c */
const PROCESS_GUID: GUID = GUID{ data1: 0x3d6fa8d0, data2: 0xfe05, data3: 0x11d0, data4: [0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c] };

/* 45d8cccd-539f-4b72-a8b7-5c683142609a */
const ALPC_GUID: GUID = GUID{ data1: 0x45d8cccd, data2: 0x539f, data3: 0x4b72, data4: [0xa8, 0xb7, 0x5c, 0x68, 0x31, 0x42, 0x60, 0x9a] };

/* 3d6fa8d4-fe05-11d0-9dda-00c04fd7ba7c */
const DISK_IO_GUID: GUID = GUID{ data1: 0x3d6fa8d4, data2: 0xfe05, data3: 0x11d0, data4: [0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c] };

/* 90cbdc39-4a3e-11d1-84f4-0000f80464e3 */
const FILE_IO_GUID: GUID = GUID{ data1: 0x90cbdc39, data2: 0x4a3e, data3: 0x11d1, data4: [0x84, 0xf4, 0x00, 0x00, 0xf8, 0x04, 0x64, 0xe3] };

/* 2cb15d1d-5fc1-11d2-abe1-00a0c911f518 */
const IMAGE_LOAD_GUID: GUID = GUID{ data1: 0x2cb15d1d, data2: 0x5fc1, data3: 0x11d2, data4: [0xab, 0xe1, 0x00, 0xa0, 0xc9, 0x11, 0xf5, 0x18] };

/* 3d6fa8d3-fe05-11d0-9dda-00c04fd7ba7c */
const PAGE_FAULT_GUID: GUID = GUID{ data1: 0x3d6fa8d3, data2: 0xfe05, data3: 0x11d0, data4: [0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c] };

/* ce1dbfb4-137e-4da6-87b0-3f59aa102cbc */
const PERF_INFO_GUID: GUID = GUID{ data1: 0xce1dbfb4, data2: 0x137e, data3: 0x4da6, data4: [0x87, 0xb0, 0x3f, 0x59, 0xaa, 0x10, 0x2c, 0xbc] };

/* AE53722E-C863-11d2-8659-00C04FA321A1 */
const REGISTRY_GUID: GUID = GUID{ data1: 0xae53722e, data2: 0xc863, data3: 0x11d2, data4: [0x86, 0x59, 0x0, 0xc0, 0x4f, 0xa3, 0x21, 0xa1] };

/* 9a280ac0-c8e0-11d1-84e2-00c04fb998a2 */
const TCP_IP_GUID: GUID = GUID{ data1: 0x9a280ac0, data2: 0xc8e0, data3: 0x11d1, data4: [0x84, 0xe2, 0x00, 0xc0, 0x4f, 0xb9, 0x98, 0xa2] };

/* 3d6fa8d1-fe05-11d0-9dda-00c04fd7ba7c */
const THREAD_GUID: GUID = GUID{ data1: 0x3d6fa8d1, data2: 0xfe05, data3: 0x11d0, data4: [0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c] };

/* bf3a50c5-a9c9-4988-a005-2df0b7c80f80 */
const UDP_IP_GUID: GUID = GUID{ data1: 0xbf3a50c5, data2: 0xa9c9, data3: 0x4988, data4: [0xa0, 0x05, 0x2d, 0xf0, 0xb7, 0xc8, 0x0f, 0x80] };

/* DEF2FE46-7BD6-4b80-bd94-F57FE20D0CE3 */
const STACK_WALK_GUID: GUID = GUID{ data1: 0xdef2fe46, data2: 0x7bd6, data3: 0x4b80, data4: [0xbd, 0x94, 0xf5, 0x7f, 0xe2, 0xd, 0xc, 0xe3] };

/* 89497f50-effe-4440-8cf2-ce6b1cdcaca7 */
const OBJECT_GUID: GUID = GUID{ data1: 0x89497f50, data2: 0xeffe, data3: 0x4440, data4: [0x8c, 0xf2, 0xce, 0x6b, 0x1c, 0xdc, 0xac, 0xa7] };

/* E43445E0-0903-48c3-B878-FF0FCCEBDD04 */
const POWER_GUID: GUID = GUID{ data1: 0xe43445e0, data2: 0x903, data3: 0x48c3, data4: [0xb8, 0x78, 0xff, 0xf, 0xcc, 0xeb, 0xdd, 0x4] };

/* F8F10121-B617-4A56-868B-9dF1B27FE32C */
const MMCSS_GUID: GUID = GUID{ data1: 0xf8f10121, data2: 0xb617, data3: 0x4a56, data4: [0x86, 0x8b, 0x9d, 0xf1, 0xb2, 0x7f, 0xe3, 0x2c] };

/* b2d14872-7c5b-463d-8419-ee9bf7d23e04 */
const DPC_GUID: GUID = GUID{ data1: 0xb2d14872, data2: 0x7c5b, data3: 0x463d, data4: [0x84, 0x19, 0xee, 0x9b, 0xf7, 0xd2, 0x3e, 0x04] };

/* d837ca92-12b9-44a5-ad6a-3a65b3578aa8 */
const SPLIT_IO_GUID: GUID = GUID{ data1: 0xd837ca92, data2: 0x12b9, data3: 0x44a5, data4: [0xad, 0x6a, 0x3a, 0x65, 0xb3, 0x57, 0x8a, 0xa8] };

/* c861d0e2-a2c1-4d36-9f9c-970bab943a12 */
const THREAD_POOL_GUID: GUID = GUID{ data1: 0xc861d0e2, data2: 0xa2c1, data3: 0x4d36, data4: [0xa5, 0xba, 0x3e, 0x3b, 0x03, 0x28, 0xc3, 0x0d] };

/* 0268a8b6-74fd-4302-9dd0-6e8f1795c0cf */
const POOL_GUID: GUID = GUID{ data1: 0x0268a8b6, data2: 0x74fd, data3: 0x4302, data4: [0x9d, 0xd0, 0x6e, 0x8f, 0x17, 0x95, 0xc0, 0xcf] };

/* 222962ab-6180-4b88-a825-346b75f2a24a */
const HEAP_GUID: GUID = GUID{ data1: 0x222962ab, data2: 0x6180, data3: 0x4b88, data4: [0xa8, 0x25, 0x34, 0x6b, 0x75, 0xf2, 0xa2, 0x4a] };

/* d781ca11-61c0-4387-b83d-af52d3d2dd6a */
const HEAP_RANGE_GUID: GUID = GUID{ data1: 0xd781ca11, data2: 0x61c0, data3: 0x4387, data4: [0xb8, 0x3d, 0xaf, 0x52, 0xd3, 0xd2, 0xdd, 0x6a] };

/* 05867806-c246-43ef-a147-e17d2bdb1496 */
const HEAP_SUMMARY_GUID: GUID = GUID{ data1: 0x05867806, data2: 0xc246, data3: 0x43ef, data4: [0xa1, 0x47, 0xe1, 0x7d, 0x2b, 0xdb, 0x14, 0x96] };

/* 3AC66736-CC59-4cff-8115-8DF50E39816B */
const CRIT_SEC_GUID: GUID = GUID{ data1: 0x3ac66736, data2: 0xcc59, data3: 0x4cff, data4: [0x81, 0x15, 0x8d, 0xf5, 0xe, 0x39, 0x81, 0x6b] };

/* 13976D09-A327-438c-950B-7F03192815C7  */
const DBG_PRINT_GUID: GUID = GUID{ data1: 0x13976d09, data2: 0xa327, data3: 0x438c, data4: [0x95, 0xb, 0x7f, 0x3, 0x19, 0x28, 0x15, 0xc7] };

/* D56CA431-61BF-4904-A621-00E0381E4DDE */
const DRIVER_VERIFIER_GUID: GUID = GUID{ data1: 0xd56ca431, data2: 0x61bf, data3: 0x4904, data4: [0xa6, 0x21, 0x0, 0xe0, 0x38, 0x1e, 0x4d, 0xde] };

/* E21D2142-DF90-4d93-BBD9-30E63D5A4AD6 */
const NTDLL_TRACE_GUID: GUID = GUID{ data1: 0xe21d2142, data2: 0xdf90, data3: 0x4d93, data4: [0xbb, 0xd9, 0x30, 0xe6, 0x3d, 0x5a, 0x4a, 0xd6] };

/* d3de60b2-a663-45d5-9826-a0a5949d2cb0 */
const LOAD_MUIDLL_GUID: GUID = GUID{ data1: 0xd3de60b2, data2: 0xa663, data3: 0x45d5, data4: [0x98, 0x26, 0xa0, 0xa5, 0x94, 0x9d, 0x2c, 0xb0] };

/* 3282fc76-feed-498e-8aa7-e70f459d430e */
const JOB_GUID: GUID = GUID{ data1: 0x3282fc76, data2: 0xfeed, data3: 0x498e, data4: [0x8a, 0xa7, 0xe7, 0x0f, 0x45, 0x9d, 0x43, 0x0e] };

