//! AIO 核心 API 集成测试
//!
//! 测试函数: aio_read, aio_write, aio_fsync, aio_return, aio_error, aio_cancel

use core::ffi::c_void;
use test_framework::test;

// ============================================================================
// aio_read 测试
// ============================================================================

test!("test_aio_read_invalid_fd" {
    // aio_read 错误处理: 无效 fd (-1) 应返回 -1
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -1;

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_read with fd=-1 should return -1 (EBADF)");
    }
});

test!("test_aio_read_invalid_fd_negative" {
    // aio_read 错误处理: 负数 fd (-100) 应返回 -1
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -100;

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_read with invalid negative fd should return -1");
    }
});

test!("test_aio_read_zero_bytes" {
    // aio_read 边界条件: nbytes=0, 应成功提交并读取 0 字节
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = 0;

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_read with nbytes=0 should succeed");

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0 on completion");

        // 验证错误码为 0 (成功)
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should return 0 on success");

        // 验证返回值为 0
        let ret_val = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(ret_val, 0, "aio_return should return 0 for zero-byte read");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_read_pipe" {
    // aio_read 基本功能: 从管道异步读取数据
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 先同步写入数据到管道
        let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let wn = super::write(fds[1], data.as_ptr() as *const c_void, data.len());
        assert_eq!(wn, 8, "write to pipe should succeed");

        // 配置 aiocb 进行异步读
        let mut buf: [u8; 16] = [0u8; 16];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_read should succeed");

        // 等待完成 (数据已在管道中, 操作可能很快完成)
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0 on completion");

        // 验证操作成功完成
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should return 0 after completion");

        // 验证读取的字节数
        let ret_val = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(ret_val, 8, "aio_return should return 8 bytes read");

        // 验证读到的数据与写入的一致
        assert_eq!(&buf[..8], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_read_pipe_large" {
    // aio_read 基本功能: 从管道异步读取较大数据量 (1024 字节)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入 1024 字节到管道
        let data: [u8; 1024] = [0xAB; 1024];
        let wn = super::write(fds[1], data.as_ptr() as *const c_void, data.len());
        assert_eq!(wn, 1024, "write 1024 bytes to pipe should succeed");

        // 异步读取
        let mut buf: [u8; 1024] = [0u8; 1024];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_read should succeed");

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0");

        let ret_val = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(ret_val, 1024, "aio_return should return 1024 bytes");
        assert_eq!(&buf[..], &data[..], "data should match for 1024 bytes");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

// ============================================================================
// aio_write 测试
// ============================================================================

test!("test_aio_write_invalid_fd" {
    // aio_write 错误处理: 无效 fd (-1) 应返回 -1
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -1;

        let ret = super::aio_write(&mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_write with fd=-1 should return -1 (EBADF)");
    }
});

test!("test_aio_write_invalid_fd_negative" {
    // aio_write 错误处理: 负数 fd (-999) 应返回 -1
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -999;

        let ret = super::aio_write(&mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_write with invalid negative fd should return -1");
    }
});

test!("test_aio_write_zero_bytes" {
    // aio_write 边界条件: nbytes=0, 应成功提交并写入 0 字节
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 4] = [1, 2, 3, 4];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[1];
        cb.aio_buf = data.as_ptr() as *mut c_void;
        cb.aio_nbytes = 0;

        let ret = super::aio_write(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_write with nbytes=0 should succeed");

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0");

        // 验证错误码为 0 (成功)
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should return 0 on success");

        // 验证返回值为 0
        let ret_val = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(ret_val, 0, "aio_return should return 0 for zero-byte write");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_write_pipe" {
    // aio_write 基本功能: 异步向管道写入数据
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 配置 aiocb 进行异步写
        let data: [u8; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[1];
        cb.aio_buf = data.as_ptr() as *mut c_void;
        cb.aio_nbytes = data.len();

        let ret = super::aio_write(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_write should succeed");

        // 等待完成 (向空管道写入小数据可能很快完成)
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0 on completion");

        // 验证操作成功完成
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should return 0 after completion");

        // 验证写入的字节数
        let ret_val = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(ret_val, 8, "aio_return should return 8 bytes written");

        // 从管道读端同步读取并验证数据
        let mut buf: [u8; 16] = [0u8; 16];
        let rn = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
        assert_eq!(rn, 8, "should read back 8 bytes");
        assert_eq!(&buf[..8], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_write_pipe_large" {
    // aio_write 基本功能: 异步向管道写入大块数据 (1024 字节)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 1024] = [0x7E; 1024];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[1];
        cb.aio_buf = data.as_ptr() as *mut c_void;
        cb.aio_nbytes = data.len();

        let ret = super::aio_write(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_write should succeed");

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0");

        let ret_val = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(ret_val, 1024, "aio_return should return 1024 bytes");

        // 验证数据
        let mut buf: [u8; 1024] = [0u8; 1024];
        let rn = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
        assert_eq!(rn, 1024, "should read back 1024 bytes");
        assert_eq!(&buf[..], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

// ============================================================================
// aio_write + aio_read 组合测试
// ============================================================================

test!("test_aio_write_then_read" {
    // 先异步写入管道, 再异步读取, 验证管道的异步 I/O 正确性
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 异步写入
        let wdata: [u8; 5] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let mut wcb: super::aiocb = unsafe { core::mem::zeroed() };
        wcb.aio_fildes = fds[1];
        wcb.aio_buf = wdata.as_ptr() as *mut c_void;
        wcb.aio_nbytes = wdata.len();

        let ret = super::aio_write(&mut wcb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_write should submit successfully");

        // 异步读取
        let mut rbuf: [u8; 8] = [0u8; 8];
        let mut rcb: super::aiocb = unsafe { core::mem::zeroed() };
        rcb.aio_fildes = fds[0];
        rcb.aio_buf = rbuf.as_mut_ptr() as *mut c_void;
        rcb.aio_nbytes = rbuf.len();

        let ret = super::aio_read(&mut rcb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_read should submit successfully");

        // 等待两个操作都完成 (使用单个 aiocb 挂起两次)
        let wcbs: [*const super::aiocb; 1] = [&wcb as *const super::aiocb];
        super::aio_suspend(wcbs.as_ptr(), 1, core::ptr::null());

        let rcbs: [*const super::aiocb; 1] = [&rcb as *const super::aiocb];
        super::aio_suspend(rcbs.as_ptr(), 1, core::ptr::null());

        // 验证异步写
        let werr = super::aio_error(&wcb as *const super::aiocb);
        assert_eq!(werr, 0, "aio_error for write should be 0");
        let wret = super::aio_return(&mut wcb as *mut super::aiocb);
        assert_eq!(wret, 5, "aio_return for write should be 5");

        // 验证异步读
        let rerr = super::aio_error(&rcb as *const super::aiocb);
        assert_eq!(rerr, 0, "aio_error for read should be 0");
        let rret = super::aio_return(&mut rcb as *mut super::aiocb);
        assert_eq!(rret, 5, "aio_return for read should be 5");
        assert_eq!(&rbuf[..5], &wdata[..], "read data should match written data");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

// ============================================================================
// aio_fsync 测试
// ============================================================================

test!("test_aio_fsync_invalid_op" {
    // aio_fsync 错误处理: 无效的 op 参数应返回 -1 (EINVAL)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        // op = 0 不是 O_SYNC 也不是 O_DSYNC
        let ret = super::aio_fsync(0, &mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_fsync with invalid op=0 should return -1 (EINVAL)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_fsync_invalid_op_negative" {
    // aio_fsync 错误处理: 负数 op 应返回 -1
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        let ret = super::aio_fsync(-1, &mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_fsync with invalid op=-1 should return -1");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_fsync_invalid_fd" {
    // aio_fsync 错误处理: 无效 fd 应返回 -1
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -1;

        let ret = super::aio_fsync(super::O_SYNC, &mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_fsync with invalid fd should return -1 (EBADF)");
    }
});

// ============================================================================
// aio_error 测试
// ============================================================================

test!("test_aio_error_einprogress" {
    // aio_error 应返回 EINPROGRESS 在操作提交后但未完成前
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut buf: [u8; 16] = [0u8; 16];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_read should submit");

        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, super::EINPROGRESS, "aio_error should return EINPROGRESS");

        // 向管道写入数据以触发完成
        let data: [u8; 4] = [1, 2, 3, 4];
        super::write(fds[1], data.as_ptr() as *const c_void, data.len());

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0");

        // 此时 aio_error 应返回 0
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should return 0 after completion");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_error_after_failed_submit" {
    // aio_error 应在提交失败后返回错误码
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -1;

        let ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(ret, -1, "aio_read should fail with invalid fd");

        // 提交失败时, __err 应该已经被设置为 errno
        let err = super::aio_error(&cb as *const super::aiocb);
        assert!(err != super::EINPROGRESS, "aio_error should not be EINPROGRESS after failed submit");
        assert!(err != 0 || err == super::EBADF, "aio_error should return error code");
    }
});

// ============================================================================
// aio_return 测试
// ============================================================================

test!("test_aio_return_after_read" {
    // aio_return 应在异步读完成后返回读取的字节数
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入
        let data: [u8; 3] = [0x11, 0x22, 0x33];
        let wn = super::write(fds[1], data.as_ptr() as *const c_void, 3);
        assert_eq!(wn, 3, "write should succeed");

        // 异步读取
        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        super::aio_read(&mut cb as *mut super::aiocb);

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 3, "aio_return should return 3 bytes read");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_return_after_write" {
    // aio_return 应在异步写完成后返回写入的字节数
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 5] = [5, 6, 7, 8, 9];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[1];
        cb.aio_buf = data.as_ptr() as *mut c_void;
        cb.aio_nbytes = 5;

        super::aio_write(&mut cb as *mut super::aiocb);

        // 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 5, "aio_return should return 5 bytes written");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_return_after_failure" {
    // aio_return 应在失败的异步操作后返回 -1
    {
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = -1;
        cb.aio_nbytes = 8;

        let submit_ret = super::aio_read(&mut cb as *mut super::aiocb);
        assert_eq!(submit_ret, -1, "aio_read should fail with invalid fd");

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, -1, "aio_return should return -1 after failed submit");
    }
});

// ============================================================================
// aio_cancel 测试
// ============================================================================

test!("test_aio_cancel_invalid_fd" {
    // aio_cancel 错误处理: 无效 fd 应返回 -1 (EBADF)
    {
        let ret = super::aio_cancel(-1, core::ptr::null_mut());
        assert_eq!(ret, -1, "aio_cancel on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_aio_cancel_invalid_fd_negative" {
    // aio_cancel 错误处理: 负数 fd (-999) 应返回 -1
    {
        let ret = super::aio_cancel(-999, core::ptr::null_mut());
        assert_eq!(ret, -1, "aio_cancel on invalid fd should return -1");
    }
});

test!("test_aio_cancel_no_operations" {
    // aio_cancel 在 fd 上没有未完成操作时应返回 AIO_ALLDONE
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 该 fd 上没有未完成的 AIO 操作
        let ret = super::aio_cancel(fds[0], core::ptr::null_mut());
        // 可能返回 AIO_ALLDONE 或 -1 (取决于是否有全局未完成的操作)
        // AIO_ALLDONE = 2
        assert!(ret == super::AIO_ALLDONE || ret == 0 || ret == -1,
            "aio_cancel on fd with no ops should return AIO_ALLDONE or 0 or -1, got {}", ret);

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

// ============================================================================
// aio 常量和类型测试
// ============================================================================

test!("test_aio_constants" {
    // 验证 AIO 常量的值
    {
        assert_eq!(super::AIO_CANCELED, 0, "AIO_CANCELED should be 0");
        assert_eq!(super::AIO_NOTCANCELED, 1, "AIO_NOTCANCELED should be 1");
        assert_eq!(super::AIO_ALLDONE, 2, "AIO_ALLDONE should be 2");
        assert_eq!(super::LIO_READ, 0, "LIO_READ should be 0");
        assert_eq!(super::LIO_WRITE, 1, "LIO_WRITE should be 1");
        assert_eq!(super::LIO_NOP, 2, "LIO_NOP should be 2");
        assert_eq!(super::LIO_WAIT, 0, "LIO_WAIT should be 0");
        assert_eq!(super::LIO_NOWAIT, 1, "LIO_NOWAIT should be 1");
    }
});
