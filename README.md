# system_monitor
A windows system tool, development in rust. A replacement of [procmon](https://learn.microsoft.com/zh-cn/sysinternals/downloads/procmon), more events and useful filter. Typically can check handle leak for a long time. because can remove the closed handle.
![image](https://github.com/wuanzhuan/system_monitor/assets/11628049/a1cbd86e-eeb7-4edb-9898-ce2bf2c74959)

# features
- [x] more events
  - [x] public and unpublished. refer to [`monitor events`](#monitor-events)
- [ ] more useful filter
  - [ ] filter one event with some filter condition
  - [ ] filter two events by match some condition. i.e. handle create and close
- [ ] find for events
  - [ ] easy query language
  - [ ] mark result of query at scroll bar of TableView
- [ ] stack trace
  - [ ] record original module and monitor change
  - [ ] translate a address to code location

## monitor events
![image](https://github.com/wuanzhuan/system_monitor/assets/11628049/8956c35a-031e-4045-92db-aa4d906a004d)


