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
		guid: ProcessGuid
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
		guid: ThreadGuid
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
		guid: ImageLoadGuid
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
		guid: RegistryGuid
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
		guid: FileIoGuid
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
		guid: DiskIoGuid
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
		guid: PageFaultGuid
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
		guid: TcpIpGuid
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
		guid: UdpIpGuid
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
		guid: PageFaultGuid
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
		guid: ThreadGuid
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
		guid: FileIoGuid
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

/* 89497f50-effe-4440-8cf2-ce6b1cdcaca7 */
const OBJECT_GUID: GUID = GUID{ data1: 0x89497f50, data2: 0xeffe, data3: 0x4440, data4: [0x8c, 0xf2, 0xce, 0x6b, 0x1c, 0xdc, 0xac, 0xa7] };

/* 0268a8b6-74fd-4302-9dd0-6e8f1795c0cf */
const POOL_GUID: GUID = GUID{ data1: 0x0268a8b6, data2: 0x74fd, data3: 0x4302, data4: [0x9d, 0xd0, 0x6e, 0x8f, 0x17, 0x95, 0xc0, 0xcf] };

/* 222962ab-6180-4b88-a825-346b75f2a24a */
const HEAP_GUID: GUID = GUID{ data1: 0x222962ab, data2: 0x6180, data3: 0x4b88, data4: [0xa8, 0x25, 0x34, 0x6b, 0x75, 0xf2, 0xa2, 0x4a] };

/* 13976D09-A327-438c-950B-7F03192815C7  */
const DBG_PRINT_GUID: GUID = GUID{ data1: 0x13976d09, data2: 0xa327, data3: 0x438c, data4: [0x95, 0xb, 0x7f, 0x3, 0x19, 0x28, 0x15, 0xc7] };

/* 3282fc76-feed-498e-8aa7-e70f459d430e */
const JOB_GUID: GUID = GUID{ data1: 0x3282fc76, data2: 0xfeed, data3: 0x498e, data4: [0x8a, 0xa7, 0xe7, 0x0f, 0x45, 0x9d, 0x43, 0x0e] };

