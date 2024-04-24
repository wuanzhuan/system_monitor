use windows::{
	core::*,
    Win32::System::Diagnostics::Etw::*
};
pub use strum::*;

/*
   reference:
   nt-kernel-logger-constants: https://learn.microsoft.com/zh-cn/windows/win32/etw/nt-kernel-logger-constants
   EnableFlags: https://learn.microsoft.com/zh-cn/windows/win32/api/evntrace/ns-evntrace-event_trace_properties_v2
   system-providers: https://learn.microsoft.com/zh-cn/windows/win32/etw/system-providers
*/
pub const EVENTS_DESC: &'static[EventsDescribe] = &[
	// Masks[0]
	EventsDescribe{
		major: MajorDescribe{name: "Process", flag: Major::Process as u32},
		minors: &[
			MinorDescribe{name: "Start", op_code: 1},
			MinorDescribe{name: "End", op_code: 2},
			MinorDescribe{name: "DCStart", op_code: 3},
			MinorDescribe{name: "DCEnd", op_code: 4},
			MinorDescribe{name: "Terminate", op_code: 11},
			MinorDescribe{name: "Defunct", op_code: 39},
		],
		guid: ProcessGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "Thread", flag: Major::Thread as u32},
		minors: &[
			MinorDescribe{name: "Start", op_code: 1},
			MinorDescribe{name: "End", op_code: 2},
			MinorDescribe{name: "DCStart", op_code: 3},
			MinorDescribe{name: "DCEnd", op_code: 4},
			MinorDescribe{name: "SetName", op_code: 72},
		],
		guid: ThreadGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "Image", flag: Major::ImageLoad as u32},
		minors: &[
			MinorDescribe{name: "Load", op_code: 10},
			MinorDescribe{name: "UnLoad", op_code: 2},
			MinorDescribe{name: "KernelBase", op_code: 33},
			MinorDescribe{name: "HypercallPage", op_code: 34},
			MinorDescribe{name: "DCStart", op_code: 3},
			MinorDescribe{name: "DCEnd", op_code: 4}
		],
		guid: ImageLoadGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "ProcessCounters", flag: Major::ProcessCounters as u32},
		minors: &[
			MinorDescribe{name: "PerfCounter", op_code: 32},
			MinorDescribe{name: "PerfCounterRundown", op_code: 33}
		],
		guid: ProcessGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "DiskIo", flag: Major::DiskIo as u32},
		minors: &[
			MinorDescribe{name: "Read", op_code: 10},
			MinorDescribe{name: "Write", op_code: 11},
			MinorDescribe{name: "FlushBuffers", op_code: 14}
		],
		guid: DiskIoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "FileIoName", flag: Major::FileIoName as u32},
		minors: &[
			MinorDescribe{name: "Name", op_code: 0},
			MinorDescribe{name: "FileCreate", op_code: 32},
			MinorDescribe{name: "FileDelete", op_code: 35},
			MinorDescribe{name: "FileRundown", op_code: 36},
			MinorDescribe{name: "Read", op_code: 10},
			MinorDescribe{name: "Write", op_code: 11},
			MinorDescribe{name: "FlushBuffers", op_code: 14},
		],
		guid: FileIoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "DiskIoInit", flag: Major::DiskIoInit as u32},
		minors: &[
			MinorDescribe{name: "ReadInit", op_code: 12},
			MinorDescribe{name: "WriteInit", op_code: 13},
			MinorDescribe{name: "FlushInit", op_code: 15}
		],
		guid: DiskIoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "MemoryPageFaults", flag: Major::MemoryPageFaults as u32},
		minors: &[
			MinorDescribe{name: "TransitionFault", op_code: 10},
			MinorDescribe{name: "DemandZeroFault", op_code: 11},
			MinorDescribe{name: "CopyOnWrite", op_code: 12},
			MinorDescribe{name: "GuardPageFault", op_code: 13},
			MinorDescribe{name: "HardPageFault", op_code: 14},
			MinorDescribe{name: "AccessViolation", op_code: 15},
		],
		guid: PageFaultGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "MemoryHardFaults", flag: Major::MemoryHardFaults as u32},
		minors: &[
			MinorDescribe{name: "HardFault", op_code: 32}
		],
		guid: PageFaultGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "VaMap", flag: Major::VaMap as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "TcpIp", flag: Major::Network as u32},
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
		],
		guid: TcpIpGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "UdpIp", flag: Major::Network as u32},
		minors: &[
			MinorDescribe{name: "UDP Fail", op_code: 17},
			MinorDescribe{name: "UDP Send IPv4", op_code: 10},
			MinorDescribe{name: "UDP Receive IPv4", op_code: 11},
			MinorDescribe{name: "UDP Send IPv6", op_code: 26},
			MinorDescribe{name: "UDP Receive IPv6", op_code: 27}
		],
		guid: UdpIpGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "Registry", flag: Major::Registry as u32},
		minors: &[
			MinorDescribe{name: "CreateKey", op_code: EVENT_TRACE_TYPE_REGCREATE},
			MinorDescribe{name: "OpenKey", op_code: EVENT_TRACE_TYPE_REGOPEN},
			MinorDescribe{name: "DeleteKey", op_code: EVENT_TRACE_TYPE_REGDELETE},
			MinorDescribe{name: "QueryKey", op_code: EVENT_TRACE_TYPE_REGQUERY},
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
			MinorDescribe{name: "Close Key", op_code: EVENT_TRACE_TYPE_REGCLOSE}
		],
		guid: RegistryGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "DebugPrint", flag: Major::DbgPrint as u32},
		minors: &[
			MinorDescribe{name: "Debug Print", op_code: 0x20}
		],
		guid: DBG_PRINT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Job", flag: Major::Job as u32},
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
		major: MajorDescribe{name: "Alpc", flag: Major::Alpc as u32},
		minors: &[
			MinorDescribe{name: "ALPC-Send-Message", op_code: 33},
			MinorDescribe{name: "ALPC-Receive-Message", op_code: 34},
			MinorDescribe{name: "ALPC-Wait-For-Reply", op_code: 35},
			MinorDescribe{name: "ALPC-Wait-For-New-Message", op_code: 36},
			MinorDescribe{name: "ALPC-Unwait", op_code: 37},
		],
		guid: ALPCGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "SplitIo", flag: Major::SplitIo as u32},
		minors: &[
			MinorDescribe{name: "VolMgr", op_code: 32},
		],
		guid: SplitIoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "DebugEvents", flag: Major::DebugEvents as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "FileIo", flag: Major::FileIo as u32},
		minors: &[
			MinorDescribe{name: "OperationEnd", op_code: 76},
		],
		guid: FileIoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "FileIoInit", flag: Major::FileIoInit as u32},
		minors: &[
			MinorDescribe{name: "Create", op_code: 64},
			MinorDescribe{name: "DirEnum", op_code: 72},
			MinorDescribe{name: "DirNotify", op_code: 77},
			MinorDescribe{name: "SetInfo", op_code: 69},
			MinorDescribe{name: "Delete", op_code: 70},
			MinorDescribe{name: "Rename", op_code: 71},
			MinorDescribe{name: "QueryInfo", op_code: 74},
			MinorDescribe{name: "FSControl", op_code: 75},
			MinorDescribe{name: "Read", op_code: 67},
			MinorDescribe{name: "Write", op_code: 68},
			MinorDescribe{name: "Cleanup", op_code: 65},
			MinorDescribe{name: "Close", op_code: 66},
			MinorDescribe{name: "Flush", op_code: 73},
		],
		guid: FileIoGuid
	},
	// Don't use, replace by masks[6] of Major
	// EventsDescribe{
	// 	major: MajorDescribe{name: "NoSysConfig", flag: Major::NoSysConfig as u32},
	// 	minors: &[],
	// 	guid: GUID::zeroed() //?
	// },

	// Mask[1]
	EventsDescribe{
		major: MajorDescribe{name: "Memory", flag: Major::Memory as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Profile", flag: Major::Profile as u32},
		minors: &[
			MinorDescribe{name: "SampleProfile", op_code: 46},
		],
		guid: PerfInfoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "ContextSwitch", flag: Major::ContextSwitch as u32},
		minors: &[
			MinorDescribe{name: "CSwitch", op_code: 36}
		],
		guid: ThreadGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "FootPrint", flag: Major::FootPrint as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "DiskIoDriver", flag: Major::Driver as u32},
		minors: &[
			MinorDescribe{name: "DrvMjFnCall", op_code: 34},
			MinorDescribe{name: "DrvMjFnRet", op_code: 35},
			MinorDescribe{name: "DrvComplRout", op_code: 37},
			MinorDescribe{name: "DrvComplReq", op_code: 52},
			MinorDescribe{name: "DrvComplReqRet", op_code: 53},
		],
		guid: DiskIoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "Refset", flag: Major::Refset as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Pool", flag: Major::Pool as u32},
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
		major: MajorDescribe{name: "PoolTrace", flag: Major::PoolTrace as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Dpc", flag: Major::Dpc as u32},
		minors: &[
			MinorDescribe{name: "ThreadDPC", op_code: 66},
			MinorDescribe{name: "DPC", op_code: 68},
			MinorDescribe{name: "TimerDPC", op_code: 69}
		],
		guid: PerfInfoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "CompactContextSwitch", flag: Major::CompactCSwitch as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Dispatcher", flag: Major::Dispatcher as u32},
		minors: &[
			MinorDescribe{name: "ReadyThread", op_code: 50}
		],
		guid: ThreadGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "PmcProfile", flag: Major::PmcProfile as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ProFiling", flag: Major::ProFiling as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ProcessInSwap", flag: Major::ProcessInSwap as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Affinity", flag: Major::Affinity as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Priority", flag: Major::Priority as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Interrupt", flag: Major::Interrupt as u32},
		minors: &[
			MinorDescribe{name: "ISR", op_code: 67}
		],
		guid: PerfInfoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "VirtualAlloc", flag: Major::VirtualAlloc as u32},
		minors: &[
			MinorDescribe{name: "VirtualAlloc", op_code: 98},
			MinorDescribe{name: "VirtualFree", op_code: 99}
		],
		guid: PageFaultGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "SpinLock", flag: Major::SpinLock as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SyncObjects", flag: Major::SyncObjects as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "DpcQueue", flag: Major::DpcQueue as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "MemInfo", flag: Major::MemInfo as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ContMemGen", flag: Major::ContMemGen as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SpinLockCounts", flag: Major::SpinLockCounts as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SpinInstr", flag: Major::SpinInstr as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SessionOrPfSection", flag: Major::SessionOrPfSection as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "MemInfoWs", flag: Major::MemInfoWs as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "KernelQueue", flag: Major::KernelQueue as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "InterruptSteer", flag: Major::InterruptSteer as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ShouldYield", flag: Major::ShouldYield as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Ws", flag: Major::Ws as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},

	// Mask[2]
	EventsDescribe{
		major: MajorDescribe{name: "AntiStarvation", flag: Major::AntiStarvation as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ProcessFreeze", flag: Major::ProcessFreeze as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "PfnList", flag: Major::PfnList as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WsDeTail", flag: Major::WsDeTail as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WsEntry", flag: Major::WsEntry as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Heap", flag: Major::Heap as u32},
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
		major: MajorDescribe{name: "SystemCall", flag: Major::SystemCall as u32},
		minors: &[
			MinorDescribe{name: "SysClEnter", op_code: 51},
			MinorDescribe{name: "SysClExit", op_code: 52}
		],
		guid: PerfInfoGuid
	},
	EventsDescribe{
		major: MajorDescribe{name: "Ums", flag: Major::Ums as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "BackTrace", flag: Major::BackTrace as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Vulcan", flag: Major::Vulcan as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "EventTrace", flag: Major::Events as u32},
		minors: &[
			MinorDescribe{name: "Extension", op_code: 5},
			MinorDescribe{name: "RDComplete", op_code: 8},
			MinorDescribe{name: "EndExtension", op_code: 32},
		],
		guid: EventTraceGuid  // https://learn.microsoft.com/zh-cn/windows/win32/api/evntrace/nc-evntrace-pevent_record_callback#remarks
	},
	EventsDescribe{
		major: MajorDescribe{name: "FullTrace", flag: Major::FullTrace as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Dfss", flag: Major::Dfss as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "PreFetch", flag: Major::PreFetch as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ProcessorIdle", flag: Major::ProcessorIdle as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "CpuConfig", flag: Major::CpuConfig as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Timer", flag: Major::Timer as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ClockInterrupt", flag: Major::ClockInterrupt as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "LoadBalancer", flag: Major::LoadBalancer as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ClockTimer", flag: Major::ClockTimer as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "IdleSelection", flag: Major::IdleSelection as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Ipi", flag: Major::Ipi as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "IoTimer", flag: Major::IoTimer as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "RegHive", flag: Major::RegHive as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "RegNotIf", flag: Major::RegNotIf as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "PpmExitLatency", flag: Major::PpmExitLatency as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WorkerThread", flag: Major::WorkerThread as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},

	// Mask[4]
	EventsDescribe{
		major: MajorDescribe{name: "OpticalIo", flag: Major::OpticalIo as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "OpticalIoInit", flag: Major::OpticalIoInit as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "DllInfo", flag: Major::DllInfo as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "DllFlushWs", flag: Major::DllFlushWs as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Object", flag: Major::ObHandle as u32},
		minors: &[
			MinorDescribe{name: "CreateHandle", op_code: 32},
			MinorDescribe{name: "CloseHandle", op_code: 33},
			MinorDescribe{name: "DuplicateHandle", op_code: 34},
			MinorDescribe{name: "TypeDCStart", op_code: 36},
			MinorDescribe{name: "TypeDCEnd", op_code: 37},
			MinorDescribe{name: "HandleDCStart", op_code: 38},
			MinorDescribe{name: "HandleDCEnd", op_code: 39}
		],
		guid: OBJECT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "Object", flag: Major::ObObject as u32},
		minors: &[
			MinorDescribe{name: "CreateObject", op_code: 48},
			MinorDescribe{name: "DeleteObject", op_code: 49},
			MinorDescribe{name: "ReferenceObject", op_code: 50},
			MinorDescribe{name: "DereferenceObject", op_code: 51}
		],
		guid: OBJECT_GUID
	},
	EventsDescribe{
		major: MajorDescribe{name: "WakeDrop", flag: Major::WakeDrop as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WakeEvent", flag: Major::WakeEvent as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Debugger", flag: Major::Debugger as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "ProcAttach", flag: Major::ProcAttach as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WakeCounter", flag: Major::WakeCounter as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Power", flag: Major::Power as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SoftTrim", flag: Major::SoftTrim as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "Cc", flag: Major::Cc as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "FltIoInit", flag: Major::FltIoInit as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "FltIo", flag: Major::FltIo as u32},
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
	EventsDescribe{
		major: MajorDescribe{name: "FltFastIo", flag: Major::FltFastIo as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "FltIoFailure", flag: Major::FltIoFailure as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "HvProfile", flag: Major::HvProfile as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WdfDpc", flag: Major::WdfDpc as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "WdfInterrupt", flag: Major::WdfInterrupt as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "CacheFlush", flag: Major::CacheFlush as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},

	// Masks[5]
	EventsDescribe{
		major: MajorDescribe{name: "HiberRundown", flag: Major::HiberRundown as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},

	// Masks[6]
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigSystem", flag: Major::SysConfigSystem as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigGraphics", flag: Major::SysConfigGraphics as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigStorge", flag: Major::SysConfigStorge as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigNetwork", flag: Major::SysConfigNetwork as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigServices", flag: Major::SysConfigServices as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigPnp", flag: Major::SysConfigPnp as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigOptical", flag: Major::SysConfigOptical as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "SysConfigAll", flag: Major::SysConfigAll as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},

	// Masks[7]
	EventsDescribe{
		major: MajorDescribe{name: "ClusterOff", flag: Major::ClusterOff as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
	EventsDescribe{
		major: MajorDescribe{name: "MemoryControl", flag: Major::MemoryControl as u32},
		minors: &[],
		guid: GUID::zeroed() //?
	},
];

pub struct EventsDescribe{
	pub major: MajorDescribe,
	pub minors: &'static [MinorDescribe],
	pub guid: GUID

}
pub struct MajorDescribe {
	pub name: &'static str,
	pub flag: u32
}

pub struct MinorDescribe {
	pub name: &'static str,
	pub op_code: u32
}

// reference: https://geoffchappell.com/studies/windows/km/ntoskrnl/inc/api/ntwmi/perfinfo_groupmask.htm?ts=0,235
#[derive(Clone, Copy, EnumIter, FromRepr, AsRefStr)]
#[repr(u32)]
pub enum Major {
	// Masks[0]
    None = 0,
	Process =               EVENT_TRACE_FLAG_PROCESS.0,
    Thread     =            EVENT_TRACE_FLAG_THREAD.0,
    ImageLoad =             EVENT_TRACE_FLAG_IMAGE_LOAD.0,
    ProcessCounters =       EVENT_TRACE_FLAG_PROCESS_COUNTERS.0,
    DiskIo  =               EVENT_TRACE_FLAG_DISK_IO.0,
	FileIoName  =           EVENT_TRACE_FLAG_DISK_FILE_IO.0 | EVENT_TRACE_FLAG_DISK_IO.0,
    DiskIoInit     =        EVENT_TRACE_FLAG_DISK_IO_INIT.0,
    MemoryPageFaults =      EVENT_TRACE_FLAG_MEMORY_PAGE_FAULTS.0,
    MemoryHardFaults  =     EVENT_TRACE_FLAG_MEMORY_HARD_FAULTS.0,
    VaMap     =             EVENT_TRACE_FLAG_VAMAP.0,
    Network    =            EVENT_TRACE_FLAG_NETWORK_TCPIP.0,
    Registry  =             EVENT_TRACE_FLAG_REGISTRY.0,
    DbgPrint    =           EVENT_TRACE_FLAG_DBGPRINT.0,
    Job      =              EVENT_TRACE_FLAG_JOB.0,
    Alpc      =             EVENT_TRACE_FLAG_ALPC.0,
    SplitIo     =           EVENT_TRACE_FLAG_SPLIT_IO.0,
    DebugEvents   =         EVENT_TRACE_FLAG_DEBUG_EVENTS,
    FileIo       =          EVENT_TRACE_FLAG_FILE_IO.0,
    FileIoInit  =           EVENT_TRACE_FLAG_FILE_IO_INIT.0,
    NoSysConfig   =         EVENT_TRACE_FLAG_NO_SYSCONFIG.0,

	// Mask[1]
    Memory              =   0x20000001u32,
    Profile             =   0x20000002u32,  // equivalent to EVENT_TRACE_FLAG_PROFILE
    ContextSwitch      =   0x20000004u32,  // equivalent to EVENT_TRACE_FLAG_CSWITCH
    FootPrint           =   0x20000008u32,
    Driver             =   0x20000010u32,  // equivalent to EVENT_TRACE_FLAG_DRIVER
    Refset              =   0x20000020u32,
    Pool                =   0x20000040u32,
    PoolTrace           =   0x20000041u32,
    Dpc                 =   0x20000080u32,  // equivalent to EVENT_TRACE_FLAG_DPC
    CompactCSwitch     =   0x20000100u32,
    Dispatcher          =   0x20000200u32,  // equivalent to EVENT_TRACE_FLAG_DISPATCHER
    PmcProfile         =   0x20000400u32,
    ProFiling           =   0x20000402u32,
    ProcessInSwap      =   0x20000800u32,
    Affinity            =   0x20001000u32,
    Priority            =   0x20002000u32,
    Interrupt           =   0x20004000u32,  // equivalent to EVENT_TRACE_FLAG_INTERRUPT
    VirtualAlloc       =   0x20008000u32,  // equivalent to EVENT_TRACE_FLAG_VIRTUAL_ALLOC
    SpinLock            =   0x20010000u32,
    SyncObjects        =   0x20020000u32,
    DpcQueue           =   0x20040000u32,
    MemInfo             =   0x20080000u32,
    ContMemGen         =   0x20100000u32,
    SpinLockCounts      =   0x20200000u32,
    SpinInstr           =   0x20210000u32,
    SessionOrPfSection  =   0x20400000u32,
    MemInfoWs          =   0x20800000u32,
    KernelQueue        =   0x21000000u32,
    InterruptSteer     =   0x22000000u32,
    ShouldYield        =   0x24000000u32,
    Ws                  =   0x28000000u32,

	// Mask[2]
	AntiStarvation  =  0x40000001u32,
	ProcessFreeze   =  0x40000002u32,
	PfnList         =  0x40000004u32,
	WsDeTail        =  0x40000008u32,
	WsEntry         =  0x40000010u32,
	Heap             =  0x40000020u32,
	SystemCall       =  0x40000040u32,  // equivalent to EVENT_TRACE_FLAG_SYSTEMCALL
	Ums              =  0x40000080u32,
	BackTrace        =  0x40000100u32,
	Vulcan           =  0x40000200u32,
	Objects          =  0x40000400u32,  // no effect in windows11 22h2
	Events           =  0x40000800u32,  // no effect in windows11 22h2 EventTrace is always enable
	FullTrace        =  0x40001000u32,
	Dfss             =  0x40002000u32,
	PreFetch         =  0x40004000u32,
	ProcessorIdle   =  0x40008000u32,
	CpuConfig       =  0x40010000u32,
	Timer            =  0x40020000u32,
	ClockInterrupt  =  0x40040000u32,
	LoadBalancer    =  0x40080000u32,
	ClockTimer      =  0x40100000u32,
	IdleSelection   =  0x40200000u32,
	Ipi              =  0x40400000u32,
	IoTimer         =  0x40800000u32,
	RegHive         =  0x41000000u32,
	RegNotIf        =  0x42000000u32,
	PpmExitLatency =  0x44000000u32,
	WorkerThread    =  0x48000000u32,

	// Mask[4]
	OpticalIo      =   0x80000001u32,
	OpticalIoInit =   0x80000002u32,
	DllInfo        =   0x80000008u32,
	DllFlushWs    =   0x80000010u32,
	ObHandle       =   0x80000040u32,
	ObObject       =   0x80000080u32,
	WakeDrop       =   0x80000200u32,
	WakeEvent      =   0x80000400u32,
	Debugger        =   0x80000800u32,
	ProcAttach     =   0x80001000u32,
	WakeCounter    =   0x80002000u32,
	Power           =   0x80008000u32,
	SoftTrim       =   0x80010000u32,
	Cc              =   0x80020000u32,
	FltIoInit     =   0x80080000u32,
	FltIo          =   0x80100000u32,
	FltFastIo      =   0x80200000u32,
	FltIoFailure  =   0x80400000u32,
	HvProfile      =   0x80800000u32,
	WdfDpc         =   0x81000000u32,
	WdfInterrupt   =   0x82000000u32,
	CacheFlush     =   0x84000000u32,

	// Masks[5]
    HiberRundown =     0xA0000001u32,

    // Masks[6]
    SysConfigSystem   =   0xC0000001u32,
    SysConfigGraphics =   0xC0000002u32,
    SysConfigStorge  =   0xC0000004u32,
    SysConfigNetwork  =   0xC0000008u32,
    SysConfigServices =   0xC0000010u32,
    SysConfigPnp      =   0xC0000020u32,
    SysConfigOptical  =   0xC0000040u32,
    SysConfigAll      =   0xDFFFFFFFu32,

	// Masks[7] - Control Mask. All flags that change system behavior go here.
    ClusterOff  =      0xE0000001u32,
    MemoryControl =    0xE0000002u32,
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


pub const PERF_MASK_INDEX: u32 = 0xe0000000;
pub const PERF_MASK_GROUP: u32 = !PERF_MASK_INDEX;
pub const PERF_NUM_MASKS: u32 =  8;

#[allow(non_camel_case_types)]
pub struct PERFINFO_GROUPMASK {
	pub masks: [u32; PERF_NUM_MASKS as usize],
}

impl PERFINFO_GROUPMASK {
	pub fn new() -> Self {
		Self { masks: [0u32; PERF_NUM_MASKS as usize] }
	}
	pub fn get_mask_index(gm: u32) -> u32 {
		return (gm & PERF_MASK_INDEX) >> 29;
	}
	
	pub fn get_mask_group(gm: u32) -> u32 {
		return gm & PERF_MASK_GROUP;
	}

	pub fn or_assign_with_groupmask(&mut self, gm: u32) {
		self.masks[PERFINFO_GROUPMASK::get_mask_index(gm) as usize] |= PERFINFO_GROUPMASK::get_mask_group(gm);
	}
}


/* 89497f50-effe-4440-8cf2-ce6b1cdcaca7 */
pub const OBJECT_GUID: GUID = GUID::from_u128(0x89497f50_effe_4440_8cf2_ce6b1cdcaca7);
/* 0268a8b6-74fd-4302-9dd0-6e8f1795c0cf */
pub const POOL_GUID: GUID = GUID::from_u128(0x0268a8b6_74fd_4302_9dd0_6e8f1795c0cf);

/* 222962ab-6180-4b88-a825-346b75f2a24a */
pub const HEAP_GUID: GUID = GUID::from_u128(0x222962ab_6180_4b88_a825_346b75f2a24a);

/* 13976D09-A327-438c-950B-7F03192815C7  */
pub const DBG_PRINT_GUID: GUID = GUID::from_u128(0x13976D09_A327_438c_950B_7F03192815C7);

/* 3282fc76-feed-498e-8aa7-e70f459d430e */
pub const JOB_GUID: GUID = GUID::from_u128(0x3282fc76_feed_498e_8aa7_e70f459d430e);

/// StackWalk: https://learn.microsoft.com/zh-cn/windows/win32/etw/stackwalk
pub const STACK_WALK_GUID: GUID = GUID::from_u128(0xdef2fe46_7bd6_4b80_bd94_f57fe20d0ce3);


pub mod event_property {
    use anyhow::{anyhow, Result};
	use tracing::error;
    use crate::event_trace::event_decoder;
	
	#[derive(Debug, Clone)]
    pub struct StackWalk {
        pub event_timestamp: i64,
        pub stack_process: u32,
        pub stack_thread: u32,
        pub stacks: Vec<(String, u64)>
    }

	impl StackWalk {
		pub fn from_event_record_decoded(erd: &event_decoder::EventRecordDecoded) -> Self {
			if let event_decoder::PropertyDecoded::Struct(map) = &erd.properties {
				let event_timestamp = map.get("EventTimeStamp").map(|property| {
					u64_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get EventTimeStamp: {e}");
						0
					})
				}).unwrap_or_default();
				let stack_process = map.get("StackProcess").map(|property| {
					u32_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get StackProcess: {e}");
						0
					})
				}).unwrap_or_default();
				let stack_thread = map.get("StackThread").map(|property| {
					u32_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get StackThread: {e}");
						0
					}) 
				}).unwrap_or_default();
				let mut stacks = vec![];
				for entry in map.iter() {
					if !entry.0.starts_with("Stack") {
						continue;
					}
					if entry.0.get("Stack".len()..).unwrap_or_default().parse::<u32>().is_err() {
						continue;
					}
					let v = u64_from_string(entry.1).unwrap_or_else(|e| {
						error!("Failed to get stack address: {e}");
						0
					});
					stacks.push((entry.0.clone(), v))
				}
				Self{event_timestamp: event_timestamp as i64, stack_process, stack_thread, stacks}
			} else {
				Self{event_timestamp: erd.timestamp.0, stack_process: 0, stack_thread: 0, stacks: vec![]}
			}
		}
	}

	#[derive(Debug, Clone, Default)]
	pub struct Image {
	    pub image_base: u64,
	    pub image_size: u32,
	    pub process_id: u32,
	    pub image_check_sum: u32,
	    pub time_date_stamp: u32,
	    pub default_base: u64,
	    pub file_name: String	
	}

	impl Image {
		pub fn from_event_record_decoded(erd: &event_decoder::EventRecordDecoded) -> Self {
			if let event_decoder::PropertyDecoded::Struct(map) = &erd.properties {
				let image_base = map.get("ImageBase").map(|property| {
					u64_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get ImageBase: {e}");
						0
					})
				}).unwrap_or_default();
				let image_size = map.get("ImageSize").map(|property| {
					u32_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get ImageSize: {e}");
						0
					})
				}).unwrap_or_default();
				let process_id = map.get("ProcessId").map(|property| {
					u32_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get ProcessId: {e}");
						0
					})
				}).unwrap_or_default();
				let image_check_sum = map.get("ImageChecksum").map(|property| {
					u32_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get ImageChecksum: {e}");
						0
					})
				}).unwrap_or_default();
				let time_date_stamp = map.get("TimeDateStamp").map(|property| {
					u32_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get TimeDateStamp: {e}");
						0
					})
				}).unwrap_or_default();
				let default_base = map.get("DefaultBase").map(|property| {
					u64_from_string(property).unwrap_or_else(|e| {
						error!("Failed to get DefaultBase: {e}");
						0
					})
				}).unwrap_or_default();
				let file_name = map.get("FileName").map(|property| {
					if let event_decoder::PropertyDecoded::String(s) = property {
						s.clone()
					} else {
						String::new()
					}
				}).unwrap_or_default();
				Self{ image_base, image_size, process_id, image_check_sum, time_date_stamp, default_base, file_name}
			} else {
				Self::default()
			}
		}
	}

	fn u64_from_string(property: &event_decoder::PropertyDecoded) -> Result<u64> {
		if let event_decoder::PropertyDecoded::String(s) = property {
			let has_0x = s.starts_with("0x") || s.starts_with("0X");
			let r = if has_0x {
				let s = s.get(2..).unwrap_or_default();
				u64::from_str_radix(s, 16)
			} else {
				u64::from_str_radix(s, 10)
			};
			match r {
				Ok(num) => Ok(num),
				Err(e) => {
					if *e.kind() == std::num::IntErrorKind::Empty {
						Ok(0)
					} else {
						Err(anyhow!("Failed to parse: {s} for EventTimeStamp, {e}"))
					}
				}
			}
		} else {
			Err(anyhow!("The property's type is not string!"))
		} 
	}

	fn u32_from_string(property: &event_decoder::PropertyDecoded) -> Result<u32> {
		if let event_decoder::PropertyDecoded::String(s) = property {
			let has_0x = s.starts_with("0x") || s.starts_with("0X");
			let r = if has_0x {
				let s = s.get(2..).unwrap_or_default();
				u32::from_str_radix(s, 16)
			} else {
				u32::from_str_radix(s, 10)
			};
			match r {
				Ok(num) => Ok(num),
				Err(e) => {
					if *e.kind() == std::num::IntErrorKind::Empty {
						Ok(0)
					} else {
						Err(anyhow!("Failed to parse: {s} for EventTimeStamp, {e}"))
					}
				}
			}
		} else {
			Err(anyhow!("The property's type is not string!"))
		} 
	}
}



