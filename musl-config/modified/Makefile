#
# Makefile for musl (requires GNU make)
#
# This is how simple every makefile should be...
# No, I take that back - actually most should be less than half this size.
#
# Use config.mak to override any of the following variables.
# Do not make changes here.
#

# test

srcdir = .
exec_prefix = /usr/local
bindir = $(exec_prefix)/bin

prefix = /usr/local/musl
includedir = $(prefix)/include
libdir = $(prefix)/lib
syslibdir = /lib

# --- RUSL crate replacements ---
# 可配置的模块替换: make replace string malloc ctype prng env
# 或: make REPLACE="string malloc"
# 不指定 REPLACE 时，构建原版 musl（不替换任何模块）
#
# 每个模块的注册信息:
RUSL_A_malloc = $(abspath $(srcdir)/../rusl/target/release/librusl_malloc.a)
RUSL_A_string = $(abspath $(srcdir)/../rusl/target/release/librusl_string.a)
RUSL_A_ctype  = $(abspath $(srcdir)/../rusl/target/release/librusl_ctype.a)
RUSL_A_prng   = $(abspath $(srcdir)/../rusl/target/release/librusl_prng.a)
RUSL_A_env      = $(abspath $(srcdir)/../rusl/target/release/librusl_env.a)
RUSL_A_internal = $(abspath $(srcdir)/../rusl/target/release/librusl_internal.a)

RUSL_FEATURES_malloc   = --no-default-features
RUSL_FEATURES_string   = --no-default-features
RUSL_FEATURES_ctype    = --no-default-features
RUSL_FEATURES_prng     = --no-default-features
RUSL_FEATURES_env      = --no-default-features
RUSL_FEATURES_internal = --no-default-features

# 有效的可替换模块列表
RUSL_MODULES = malloc string ctype prng env internal

# ---- REPLACE 解析 ----
# 支持 make replace mod1 mod2 ... (通过 MAKECMDGOALS 解析)
ifneq ($(findstring replace,$(MAKECMDGOALS)),)
  REPLACE := $(filter $(RUSL_MODULES),$(MAKECMDGOALS))
endif
# 也支持 make REPLACE="mod1 mod2"
REPLACE ?=

RUSL_WORKSPACE = $(abspath $(srcdir)/../rusl)
RUSL_OBJDIR    = obj/rusl
RUSL_STAMP     = $(RUSL_OBJDIR)/.extracted

# ---- 动态派生 ----
# 被替换模块对应的 musl 源码排除模式
RUSL_SRC_FILTER = $(foreach m,$(REPLACE),$(srcdir)/src/$(m)/%)
# 仅 string 有 arch 特定文件需要排除
RUSL_ARCH_FILTER = $(if $(findstring string,$(REPLACE)),$(srcdir)/src/string/%,)
# 需要提取的 .a 列表
RUSL_STAMP_DEPS = $(foreach m,$(REPLACE),$(RUSL_A_$(m)))
# 链接时添加的 .o 文件
RUSL_LINK_OBJS  = $(if $(REPLACE),$(RUSL_OBJDIR)/*.o,)

# floatscan_wrap.c: C ABI 包装层 (f64→long double), 替换 internal 时保留编译
RUSL_WRAP_OBJ = $(if $(findstring internal,$(REPLACE)),obj/src/internal/floatscan_wrap.o,)

MALLOC_DIR = mallocng
SRC_DIRS = $(addprefix $(srcdir)/,src/* src/malloc/$(MALLOC_DIR) crt ldso $(COMPAT_SRC_DIRS))
BASE_GLOBS = $(addsuffix /*.c,$(SRC_DIRS))
ARCH_GLOBS = $(addsuffix /$(ARCH)/*.[csS],$(SRC_DIRS))
BASE_SRCS = $(sort $(filter-out $(RUSL_SRC_FILTER),$(wildcard $(BASE_GLOBS))))
ARCH_SRCS = $(sort $(filter-out $(RUSL_ARCH_FILTER),$(wildcard $(ARCH_GLOBS))))
BASE_OBJS = $(patsubst $(srcdir)/%,%.o,$(basename $(BASE_SRCS)))
ARCH_OBJS = $(patsubst $(srcdir)/%,%.o,$(basename $(ARCH_SRCS)))
REPLACED_OBJS = $(sort $(subst /$(ARCH)/,/,$(ARCH_OBJS)))
ALL_OBJS = $(addprefix obj/, $(filter-out $(REPLACED_OBJS), $(sort $(BASE_OBJS) $(ARCH_OBJS))))
ALL_OBJS += $(RUSL_WRAP_OBJ)

LIBC_OBJS = $(filter obj/src/%,$(ALL_OBJS)) $(filter obj/compat/%,$(ALL_OBJS))
LDSO_OBJS = $(filter obj/ldso/%,$(ALL_OBJS:%.o=%.lo))
CRT_OBJS = $(filter obj/crt/%,$(ALL_OBJS))

AOBJS = $(LIBC_OBJS)
LOBJS = $(LIBC_OBJS:.o=.lo)
GENH = obj/include/bits/alltypes.h obj/include/bits/syscall.h
GENH_INT = obj/src/internal/version.h
IMPH = $(addprefix $(srcdir)/, src/internal/stdio_impl.h src/internal/pthread_impl.h src/internal/locale_impl.h src/internal/libc.h)

LDFLAGS =
LDFLAGS_AUTO =
LIBCC = -lgcc
CPPFLAGS =
CFLAGS =
CFLAGS_AUTO = -Os -pipe
CFLAGS_C99FSE = -std=c99 -ffreestanding -nostdinc 

CFLAGS_ALL = $(CFLAGS_C99FSE)
CFLAGS_ALL += -D_XOPEN_SOURCE=700 -I$(srcdir)/arch/$(ARCH) -I$(srcdir)/arch/generic -Iobj/src/internal -I$(srcdir)/src/include -I$(srcdir)/src/internal -Iobj/include -I$(srcdir)/include
CFLAGS_ALL += $(CPPFLAGS) $(CFLAGS_AUTO) $(CFLAGS)

LDFLAGS_ALL = $(LDFLAGS_AUTO) $(LDFLAGS)

AR      = $(CROSS_COMPILE)ar
RANLIB  = $(CROSS_COMPILE)ranlib
INSTALL = $(srcdir)/tools/install.sh

ARCH_INCLUDES = $(wildcard $(srcdir)/arch/$(ARCH)/bits/*.h)
GENERIC_INCLUDES = $(wildcard $(srcdir)/arch/generic/bits/*.h)
INCLUDES = $(wildcard $(srcdir)/include/*.h $(srcdir)/include/*/*.h)
ALL_INCLUDES = $(sort $(INCLUDES:$(srcdir)/%=%) $(GENH:obj/%=%) $(ARCH_INCLUDES:$(srcdir)/arch/$(ARCH)/%=include/%) $(GENERIC_INCLUDES:$(srcdir)/arch/generic/%=include/%))

EMPTY_LIB_NAMES = m rt pthread crypt util xnet resolv dl
EMPTY_LIBS = $(EMPTY_LIB_NAMES:%=lib/lib%.a)
CRT_LIBS = $(addprefix lib/,$(notdir $(CRT_OBJS)))
STATIC_LIBS = lib/libc.a
SHARED_LIBS = lib/libc.so
TOOL_LIBS = lib/musl-gcc.specs
ALL_LIBS = $(CRT_LIBS) $(STATIC_LIBS) $(SHARED_LIBS) $(EMPTY_LIBS) $(TOOL_LIBS)
ALL_TOOLS = obj/musl-gcc

WRAPCC_GCC = gcc
WRAPCC_CLANG = clang

LDSO_PATHNAME = $(syslibdir)/ld-musl-$(ARCH)$(SUBARCH).so.1

-include config.mak
-include $(srcdir)/arch/$(ARCH)/arch.mak

ifeq ($(ARCH),)

all:
	@echo "Please set ARCH in config.mak before running make."
	@exit 1

else

all: $(ALL_LIBS) $(ALL_TOOLS)

OBJ_DIRS = $(sort $(patsubst %/,%,$(dir $(ALL_LIBS) $(ALL_TOOLS) $(ALL_OBJS) $(ALL_OBJS:%.o=%.lo) $(GENH) $(GENH_INT))) obj/include $(RUSL_OBJDIR))

$(ALL_LIBS) $(ALL_TOOLS) $(ALL_OBJS) $(ALL_OBJS:%.o=%.lo) $(GENH) $(GENH_INT): | $(OBJ_DIRS)

$(OBJ_DIRS):
	mkdir -p $@

obj/include/bits/alltypes.h: $(srcdir)/arch/$(ARCH)/bits/alltypes.h.in $(srcdir)/include/alltypes.h.in $(srcdir)/tools/mkalltypes.sed
	sed -f $(srcdir)/tools/mkalltypes.sed $(srcdir)/arch/$(ARCH)/bits/alltypes.h.in $(srcdir)/include/alltypes.h.in > $@

obj/include/bits/syscall.h: $(srcdir)/arch/$(ARCH)/bits/syscall.h.in
	cp $< $@
	sed -n -e s/__NR_/SYS_/p < $< >> $@

obj/src/internal/version.h: $(wildcard $(srcdir)/VERSION $(srcdir)/.git)
	printf '#define VERSION "%s"\n' "$$(cd $(srcdir); sh tools/version.sh)" > $@

obj/src/internal/version.o obj/src/internal/version.lo: obj/src/internal/version.h

obj/crt/rcrt1.o obj/ldso/dlstart.lo obj/ldso/dynlink.lo: $(srcdir)/src/internal/dynlink.h $(srcdir)/arch/$(ARCH)/reloc.h

obj/crt/crt1.o obj/crt/Scrt1.o obj/crt/rcrt1.o obj/ldso/dlstart.lo: $(srcdir)/arch/$(ARCH)/crt_arch.h

obj/crt/rcrt1.o: $(srcdir)/ldso/dlstart.c

obj/crt/Scrt1.o obj/crt/rcrt1.o: CFLAGS_ALL += -fPIC

OPTIMIZE_SRCS = $(wildcard $(OPTIMIZE_GLOBS:%=$(srcdir)/src/%))
$(OPTIMIZE_SRCS:$(srcdir)/%.c=obj/%.o) $(OPTIMIZE_SRCS:$(srcdir)/%.c=obj/%.lo): CFLAGS += -O3

MEMOPS_OBJS = $(filter %/memcpy.o %/memmove.o %/memcmp.o %/memset.o, $(LIBC_OBJS))
$(MEMOPS_OBJS) $(MEMOPS_OBJS:%.o=%.lo): CFLAGS_ALL += $(CFLAGS_MEMOPS)

NOSSP_OBJS = $(CRT_OBJS) $(LDSO_OBJS) $(filter \
	%/__libc_start_main.o %/__init_tls.o %/__stack_chk_fail.o \
	%/__set_thread_area.o %/memset.o %/memcpy.o \
	, $(LIBC_OBJS))
$(NOSSP_OBJS) $(NOSSP_OBJS:%.o=%.lo): CFLAGS_ALL += $(CFLAGS_NOSSP)

$(CRT_OBJS): CFLAGS_ALL += -DCRT

$(LOBJS) $(LDSO_OBJS): CFLAGS_ALL += -fPIC

CC_CMD = $(CC) $(CFLAGS_ALL) -c -o $@ $<

# Choose invocation of assembler to be used
ifeq ($(ADD_CFI),yes)
	AS_CMD = LC_ALL=C awk -f $(srcdir)/tools/add-cfi.common.awk -f $(srcdir)/tools/add-cfi.$(ARCH).awk $< | $(CC) $(CFLAGS_ALL) -x assembler -c -o $@ -
else
	AS_CMD = $(CC_CMD)
endif

obj/%.o: $(srcdir)/%.s
	$(AS_CMD)

obj/%.o: $(srcdir)/%.S
	$(CC_CMD)

obj/%.o: $(srcdir)/%.c $(GENH) $(IMPH)
	$(CC_CMD)

obj/%.lo: $(srcdir)/%.s
	$(AS_CMD)

obj/%.lo: $(srcdir)/%.S
	$(CC_CMD)

obj/%.lo: $(srcdir)/%.c $(GENH) $(IMPH)
	$(CC_CMD)

# --- rusl object extraction ---
# 条件化提取: 仅当 REPLACE 非空时执行
# 所有 staticlib 共享依赖 (rusl_core, core)，提取到同一目录避免重复符号。
#
ifeq ($(REPLACE),)

$(RUSL_STAMP):
	@mkdir -p $(RUSL_OBJDIR)
	@touch $@

else

$(RUSL_STAMP): $(RUSL_STAMP_DEPS)
	@mkdir -p $(RUSL_OBJDIR)
	@cd $(RUSL_OBJDIR); for a in $(RUSL_STAMP_DEPS); do $(AR) x $$a; done
	rm -f $(RUSL_OBJDIR)/test_framework-*.o
	rm -f $(RUSL_OBJDIR)/rustc_std_workspace_core-*.o
	rm -f $(RUSL_OBJDIR)/compiler_builtins-*.o
	@touch $@

endif

lib/libc.so: $(LOBJS) $(LDSO_OBJS) $(RUSL_STAMP)
	$(CC) $(CFLAGS_ALL) $(LDFLAGS_ALL) -nostdlib -shared \
	-Wl,-e,_dlstart -o $@ $(LOBJS) $(LDSO_OBJS) \
	$(RUSL_LINK_OBJS) $(LIBCC)

lib/libc.a: $(AOBJS) $(RUSL_STAMP)
	rm -f $@
	$(AR) rc $@ $(AOBJS)
	$(if $(REPLACE),$(AR) r $@ $(RUSL_LINK_OBJS))
	$(RANLIB) $@

$(EMPTY_LIBS):
	rm -f $@
	$(AR) rc $@

lib/%.o: obj/crt/$(ARCH)/%.o
	cp $< $@

lib/%.o: obj/crt/%.o
	cp $< $@

lib/musl-gcc.specs: $(srcdir)/tools/musl-gcc.specs.sh config.mak
	sh $< "$(includedir)" "$(libdir)" "$(LDSO_PATHNAME)" > $@

obj/musl-gcc: config.mak
	printf '#!/bin/sh\nexec "$${REALGCC:-$(WRAPCC_GCC)}" "$$@" -specs "%s/musl-gcc.specs"\n' "$(libdir)" > $@
	chmod +x $@

obj/%-clang: $(srcdir)/tools/%-clang.in config.mak
	sed -e 's!@CC@!$(WRAPCC_CLANG)!g' -e 's!@PREFIX@!$(prefix)!g' -e 's!@INCDIR@!$(includedir)!g' -e 's!@LIBDIR@!$(libdir)!g' -e 's!@LDSO@!$(LDSO_PATHNAME)!g' $< > $@
	chmod +x $@

$(DESTDIR)$(bindir)/%: obj/%
	$(INSTALL) -D $< $@

$(DESTDIR)$(libdir)/%.so: lib/%.so
	$(INSTALL) -D -m 755 $< $@

$(DESTDIR)$(libdir)/%: lib/%
	$(INSTALL) -D -m 644 $< $@

$(DESTDIR)$(includedir)/bits/%: $(srcdir)/arch/$(ARCH)/bits/%
	$(INSTALL) -D -m 644 $< $@

$(DESTDIR)$(includedir)/bits/%: $(srcdir)/arch/generic/bits/%
	$(INSTALL) -D -m 644 $< $@

$(DESTDIR)$(includedir)/bits/%: obj/include/bits/%
	$(INSTALL) -D -m 644 $< $@

$(DESTDIR)$(includedir)/%: $(srcdir)/include/%
	$(INSTALL) -D -m 644 $< $@

$(DESTDIR)$(LDSO_PATHNAME): $(DESTDIR)$(libdir)/libc.so
	$(INSTALL) -D -l $(libdir)/libc.so $@ || true

install-libs: $(ALL_LIBS:lib/%=$(DESTDIR)$(libdir)/%) $(if $(SHARED_LIBS),$(DESTDIR)$(LDSO_PATHNAME),)

install-headers: $(ALL_INCLUDES:include/%=$(DESTDIR)$(includedir)/%)

install-tools: $(ALL_TOOLS:obj/%=$(DESTDIR)$(bindir)/%)

install: install-libs install-headers install-tools

musl-git-%.tar.gz: .git
	 git --git-dir=$(srcdir)/.git archive --format=tar.gz --prefix=$(patsubst %.tar.gz,%,$@)/ -o $@ $(patsubst musl-git-%.tar.gz,%,$@)

musl-%.tar.gz: .git
	 git --git-dir=$(srcdir)/.git archive --format=tar.gz --prefix=$(patsubst %.tar.gz,%,$@)/ -o $@ v$(patsubst musl-%.tar.gz,%,$@)

endif

clean:
	rm -rf obj lib

distclean: clean
	rm -f config.mak

# --- RUSL crate auto-build ---
# FORCE ensures cargo build runs every make invocation.
FORCE:

# 各模块的构建规则（仅在 REPLACE 包含该模块时触发）
$(RUSL_A_malloc): FORCE
	cd $(RUSL_WORKSPACE) && cargo build --package rusl-malloc $(RUSL_FEATURES_malloc) --release

$(RUSL_A_string): FORCE
	cd $(RUSL_WORKSPACE) && cargo build --package rusl-string $(RUSL_FEATURES_string) --release

$(RUSL_A_ctype): FORCE
	cd $(RUSL_WORKSPACE) && cargo build --package rusl-ctype $(RUSL_FEATURES_ctype) --release

$(RUSL_A_prng): FORCE
	cd $(RUSL_WORKSPACE) && cargo build --package rusl-prng $(RUSL_FEATURES_prng) --release

$(RUSL_A_env): FORCE
	cd $(RUSL_WORKSPACE) && cargo build --package rusl-env $(RUSL_FEATURES_env) --release

$(RUSL_A_internal): FORCE
	cd $(RUSL_WORKSPACE) && cargo build --package rusl-internal $(RUSL_FEATURES_internal) --release

# ---- replace 目标 ----
# 用法: make replace string malloc ctype prng env
# 也可以用: make REPLACE="string malloc"
replace: all
	@# Patch musl-gcc & specs to use local build paths (no sudo make install needed)
	@sed -i 's|/usr/local/musl/lib/musl-gcc.specs|$(abspath lib/musl-gcc.specs)|' obj/musl-gcc
	@sed -i 's|-L/usr/local/musl/lib |-L$(abspath lib) |g' lib/musl-gcc.specs
	@sed -i 's|/usr/local/musl/lib/Scrt1.o|$(abspath lib/Scrt1.o)|g' lib/musl-gcc.specs
	@sed -i 's|-dynamic-linker /lib/ld-musl-x86_64\.so\.1|-dynamic-linker $(abspath lib/libc.so)|g' lib/musl-gcc.specs
	@sed -i 's|/usr/local/musl/lib/crti.o|$(abspath lib/crti.o)|g' lib/musl-gcc.specs
	@sed -i 's|/usr/local/musl/lib/crtn.o|$(abspath lib/crtn.o)|g' lib/musl-gcc.specs

# 帮助信息
replace-help:
	@echo "Usage: make replace [module ...]"
	@echo "       make REPLACE=\"module ...\""
	@echo "Available modules: $(RUSL_MODULES)"
	@echo ""
	@echo "Examples:"
	@echo "  make replace string malloc          # 替换 string 和 malloc"
	@echo "  make REPLACE=\"string ctype prng\"   # 等价变量写法"

# 占位目标, 防止模块名被当作目标报错
$(RUSL_MODULES): ;

.PHONY: all clean install install-libs install-headers install-tools FORCE replace replace-help $(RUSL_MODULES)
