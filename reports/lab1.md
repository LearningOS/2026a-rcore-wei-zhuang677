# 完成功能总结
- 扩充了TaskControlBlock,新增`syscall_num: BTreeMap<usize, usize>`用于存储每个任务系统调用的次数
- 修改TASK_MANAGER的初始化
- 为TaskManager实现系统调用次数的记录与查询
- 在`os/src/syscall/mod.rs`中的`syscall`的`match`前进行当前任务系统调用的记录
- 在`os/src/syscall/process.rs`中实现sys_trace要求的逻辑
# 简答作业
## 1
- RustSBI version 0.4.1
- 运行`make -C os TEST=ch2 run`将`/home/wang/2026a-rcore-wei-zhuang677/user/src/bin`下`ch2b*.rs`(包含ch2b_bad_*.rs)全部编译打包运行
- 运行报错  
    1. `ch2b_bad_address.rs` 往0x0地址写数据，触发报错。这是触发了CPU 硬件的StorePageFault,被`os/src/trap/mod.rs`中的`trap_handler`处理输出
        >[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
    2. `ch2b_bad_instructions.rs`在用户态执行`sret`触发IllegalInstruction异常，同样在`os/src/trap/mod.rs`中处理输出
        >[kernel] IllegalInstruction in application, kernel killed it.
    3. `ch2b_bad_register.rs`在用户态试图访问sstatus，硬件也直接报 非法指令异常 IllegalInstruction，在`os/src/trap/mod.rs`中处理输出
        >[kernel] IllegalInstruction in application, kernel killed it.

## 2
1. 刚进入__restore时sp 代表的是当前陷阱帧的起始地址，场景： 
    1. 用户态陷入处理完成，恢复上下文，返回用户程序  
    2. 首次启动一个任务，作为任务的入口

2. 
    - sstatus:S态状态寄存器,决定 sret 之后切换到哪个特权级
    - sepc:S态异常程序计数器,sret 指令会跳转到 sepc 指向的地址执行
    - sscratch:S态临时暂存寄存器,保存用户态栈指针 sp

3. 因为x2和x4分别是sp和tp,需要用到sp进行寻址，而tp 是线程指针不能随意覆盖
4. sp用户态栈指针 sscratch内核栈指针
5. sret，它会让sstatus.SPP = 0.同时跳转到sepc 地址继续执行用户代码
6. sp：内核栈指针 sscratch保存了用户栈指针，暂存用户sp
7. ecall

#  荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    >https://learningos.cn/rCore-Tutorial-Guide/chapter3/5exercise.html
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。