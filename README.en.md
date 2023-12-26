# system_monitor

## Description
### 功能介绍
监控windows系统的事件，包含内核事件（进程，线程，文件，网络，句柄，内核对象，内存，ALPC等）、用户态事件（RPC等）。相比procmon可以监控更多的事件，过滤功能更强大，更实用。
如监控句柄泄露，目前市面上很少有实用的工具。procmon没有提供句柄监控的能力。提供句柄监控的windbg，通过中断（int3）监控，性能很低，事件一多就会丢弃事件，不适用。
### 实现
基于etw监控事件，通过windows-rs操作etw来记录事件。ui用slint。过滤规则自己设计实现。
### 进度
目前事件记录已经封装好。正在整合ui。slint因为要支持嵌入式，所以所有ui操作工作在mainthread中，多线程交互的能力有限。而我这个工具很注重性能。记录的事件在一个双向链表中，操作主要是push_back，remove，和查询。
另外根据需要对这个链表匹配索引，比如过滤句柄时，当监控到一个句柄关闭时，需要查找到打开这个句柄的操作，把这两个记录一起从链表中删除。所以当前正在为slint实现一个多线程中显示数组的model。

## Software Architecture
Software architecture description

## Installation

1.  xxxx
2.  xxxx
3.  xxxx

## Instructions

1.  xxxx
2.  xxxx
3.  xxxx

## Contribution

1.  Fork the repository
2.  Create Feat_xxx branch
3.  Commit your code
4.  Create Pull Request


## Gitee Feature

1.  You can use Readme\_XXX.md to support different languages, such as Readme\_en.md, Readme\_zh.md
2.  Gitee blog [blog.gitee.com](https://blog.gitee.com)
3.  Explore open source project [https://gitee.com/explore](https://gitee.com/explore)
4.  The most valuable open source project [GVP](https://gitee.com/gvp)
5.  The manual of Gitee [https://gitee.com/help](https://gitee.com/help)
6.  The most popular members  [https://gitee.com/gitee-stars/](https://gitee.com/gitee-stars/)
