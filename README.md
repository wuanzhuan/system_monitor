# system_monitor
A windows system tool, development in rust. A replacement of [procmon](https://learn.microsoft.com/zh-cn/sysinternals/downloads/procmon), more events and useful filter. Typically can check handle leak for a long time(i.e. a week). because can remove the closed handle.
![image](https://github.com/wuanzhuan/system_monitor/assets/11628049/a1cbd86e-eeb7-4edb-9898-ce2bf2c74959)

# features
- [x] more events
  - [x] public and unpublished. refer to [`monitor events`](#monitor-events)
- [ ] more useful filter
  - [ ] filter one event with some filter condition
  - [ ] filter two events by match some condition. i.e. handle create and close
- [ ] find for events
  - [x] easy query language
  - [ ] mark result of query at scroll bar of TableView
- [ ] call stack view
  - [x] record original module and monitor change
  - [x] convert the virtual address to the offset of module
  - [ ] translate a module offset to the code location
- easy of use
  - [ ] syntax highlight for filter expression
  - [ ] tips

# supported os version
- [x] windows11 x64
- [ ] windows10 x64
- [ ] windows10 x32

## monitor events
![image](https://github.com/wuanzhuan/system_monitor/assets/11628049/8956c35a-031e-4045-92db-aa4d906a004d)


