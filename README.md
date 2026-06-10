# rusl-use claude code to rewrite musl libc in Rust

## 目录结构

```
.
├── CLAUDE.md               
├── libc-test                   // musl官方测试套件
├── musl-1.2.6                  // musl源码
├── musl-config                 // 修改的和原版的musl配置
├── README.md
├── rusl                        // rusl源码
├── rust-spec                   // rust spec
├── spec                        // C spec
└── tests-plan                  // 测试分类说明
```


## 对musl的修改
修改了musl的Makefile以部分替换为rusl实现，修改后的Makefile放在[Makefile-modified](musl-config/modified/Makefile)，原版的Makefile备份在![Makefile-orig](musl-config/origin/Makefile.bak)，[lib-origin](musl-config/origin/lib/)存放原版musl的编译产物。[REPORT-orig](musl-config/origin/REPORT.original)是使用原版musl测试的测试报告。

## 测试说明

### 部分实现替换musl并一同链接测试libc-test

libc-test是musl的官方测试套件之一，总的测试思路是将musl源码中的部分替换为rusl实现，链接为一个libc后使用libc-test测试。rusl workspace包括多个crate，每个crate对应musl/src中的一个目录（模块），每个crate可以单独编译，通过修改musl Makefile，可以单独与musl源码中其余部分编译链接组成一个libc并测试，此时需要关闭默认feature。

在musl目录下使用make replace xxx命令替换掉musl的xxx模块并使用rusl。

```shell
cargo build -p rusl-xxx --no-default-features
cd musl-1.2.6 && make clean && make replace xxx -j16
```

### Rusl单元测试

rusl源码中已经包含了一些单元测试，使用cargo测试

```shell
cd rusl
make lib-test
```

### Rusl集成测试
rusl源码包含了集成测试，集成测试只测试libc应当直接提供给用户的api接口

```shell
cd rusl
make inte-test
```
由于rusl应当与musl abi兼容，所以集成测试也可以直接测试musl实现，通过链接整个musl libc来直接测试musl的C实现

```shell
cd rusl
make test-c
```