# system_monitor
A windows system tool, development in rust. A replacement of [procmon](https://learn.microsoft.com/zh-cn/sysinternals/downloads/procmon), more events and useful filter. Typically can check handle leak for a long time(i.e. a week). because can remove the closed handle.
![image](https://github.com/wuanzhuan/system_monitor/assets/11628049/a1cbd86e-eeb7-4edb-9898-ce2bf2c74959)

# features
- [x] more events
  - [x] public and unpublished. refer to [`monitor events`](#monitor-events)
- [x] more useful filter
  - [x] filter one event with some filter condition
    - value: any string and number. i.e. `1234567` or `"system_monitor"`.
    - key-value: key is any column. i.e. `process_id` or `properties.xxx`. value is any string or number.
    - express: can use `&& || ! ()` i.e `process_id = 4 && thread_id = 6`
  - [x] filter two events by match some condition. i.e. handle create and close
    - handle: match CreateHandle and CloseHandle and remove the tow events
    - custom(event_display_name, opcode_name_first, opcode_name_second, path_for_match, ...) : can has multi path_for_match. match the opcode_name_first and opcode_name_second, and remove the two events.
- [ ] find for events
  - [x] easy query language
    - value: any string and number. i.e. `1234567` or `"system_monitor"`.
    - key-value: key is any column. i.e. `process_id` or `properties.xxx`. value is any string or number.
    - express: can use `&& || ! ()` i.e `process_id = 4 && thread_id = 6`
  - [ ] mark result of query at scroll bar of TableView
- [x] call stack view
  - [x] record original module and monitor change
  - [x] convert the virtual address to the offset of module
  - [x] translate a module offset to the code location
- easy of use
  - [ ] syntax highlight for filter expression
  - [ ] tips

# supported os version
- [x] windows11 x64
- [ ] windows10 x64
- [ ] windows10 x32

## monitor events
![image](https://github.com/wuanzhuan/system_monitor/assets/11628049/8956c35a-031e-4045-92db-aa4d906a004d)


