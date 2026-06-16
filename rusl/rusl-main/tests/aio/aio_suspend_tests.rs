//! aio_suspend 集成测试
//!
//! 测试函数: aio_suspend

use core::ffi::c_void;
use test_framework::test;

// ============================================================================
// aio_suspend 错误处理测试
// ============================================================================

test!("test_aio_suspend_invalid_cnt" {
    // aio_suspend 错误处理: cnt < 0 应返回 -1 (EINVAL)
    {
        let ret = super::aio_suspend(core::ptr::null(), -1, core::ptr::null());
        assert_eq!(ret, -1, "aio_suspend with cnt=-1 should return -1 (EINVAL)");
    }
});

test!("test_aio_suspend_invalid_cnt_negative" {
    // aio_suspend 错误处理: cnt = -100 应返回 -1
    {
        let ret = super::aio_suspend(core::ptr::null(), -100, core::ptr::null());
        assert_eq!(ret, -1, "aio_suspend with cnt=-100 should return -1 (EINVAL)");
    }
});

test!("test_aio_suspend_zero_timeout" {
    // aio_suspend: 空列表 + 零超时应返回 -1 (EAGAIN, 超时)
    {
        let ts = super::Timespec { tv_sec: 0, tv_nsec: 0 };
        let ret = super::aio_suspend(
            core::ptr::null(),
            0,
            &ts as *const super::Timespec as *const _,
        );
        // 空列表且零超时 → 应超时返回 EAGAIN
        assert_eq!(ret, -1, "aio_suspend with zero timeout and empty list should return -1 (EAGAIN)");
    }
});

test!("test_aio_suspend_null_cbs_zero_cnt" {
    // aio_suspend: NULL cbs + cnt=0 + 小超时应超时返回 -1
    {
        let ts = super::Timespec { tv_sec: 0, tv_nsec: 1000000 }; // 1ms
        let ret = super::aio_suspend(
            core::ptr::null(),
            0,
            &ts as *const super::Timespec as *const _,
        );
        assert_eq!(ret, -1, "aio_suspend with 1ms timeout on empty list should time out");
    }
});

// ============================================================================
// aio_suspend 基本功能测试
// ============================================================================

test!("test_aio_suspend_with_completed_op" {
    // aio_suspend: 挂起等待已完成的操作应立即返回 0
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入数据
        let data: [u8; 4] = [1, 2, 3, 4];
        super::write(fds[1], data.as_ptr() as *const c_void, 4);

        // 异步读取
        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        super::aio_read(&mut cb as *mut super::aiocb);

        // 使用 aio_suspend 等待
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(ret, 0, "aio_suspend should return 0 when operation completes");

        // 验证操作已完成
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should be 0 after aio_suspend returns");

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 4, "should have read 4 bytes");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_suspend_wait_for_progress" {
    // aio_suspend: 挂起等待进行中的操作
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 先异步写入 (此时管道为空, 读端无数据)
        let wdata: [u8; 6] = [0xA, 0xB, 0xC, 0xD, 0xE, 0xF];
        let mut wcb: super::aiocb = unsafe { core::mem::zeroed() };
        wcb.aio_fildes = fds[1];
        wcb.aio_buf = wdata.as_ptr() as *mut c_void;
        wcb.aio_nbytes = wdata.len();

        let ret = super::aio_write(&mut wcb as *mut super::aiocb);
        assert_eq!(ret, 0, "aio_write should submit");

        // 使用 aio_suspend 等待写操作完成
        let cbs: [*const super::aiocb; 1] = [&wcb as *const super::aiocb];
        let ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(ret, 0, "aio_suspend should return 0 when write completes");

        // 验证写入完成
        let werr = super::aio_error(&wcb as *const super::aiocb);
        assert_eq!(werr, 0, "write should have completed successfully");
        let wret = super::aio_return(&mut wcb as *mut super::aiocb);
        assert_eq!(wret, 6, "should have written 6 bytes");

        // 验证数据可从管读端读出
        let mut rbuf: [u8; 8] = [0u8; 8];
        let rn = super::read(fds[0], rbuf.as_mut_ptr() as *mut c_void, rbuf.len());
        assert_eq!(rn, 6, "should read back 6 bytes");
        assert_eq!(&rbuf[..6], &wdata[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_suspend_with_timeout" {
    // aio_suspend: 使用超时参数等待 (带足够长的超时)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入
        let data: [u8; 5] = [0x10, 0x20, 0x30, 0x40, 0x50];
        super::write(fds[1], data.as_ptr() as *const c_void, 5);

        // 异步读取
        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        super::aio_read(&mut cb as *mut super::aiocb);

        // 使用 1 秒超时
        let ts = super::Timespec { tv_sec: 1, tv_nsec: 0 };
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let ret = super::aio_suspend(
            cbs.as_ptr(),
            1,
            &ts as *const super::Timespec as *const _,
        );
        // 管道中已有数据, 应快速完成
        assert_eq!(ret, 0, "aio_suspend should return 0 within timeout");

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 5, "should have read 5 bytes");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_suspend_multiple_ops" {
    // aio_suspend: 等待多个操作, 任意一个完成即返回
    {
        let mut fds1: [super::c_int; 2] = [-1, -1];
        let mut fds2: [super::c_int; 2] = [-1, -1];
        super::pipe(fds1.as_mut_ptr());
        super::pipe(fds2.as_mut_ptr());

        // 向两个管道各写入数据
        let data1: [u8; 3] = [1, 1, 1];
        let data2: [u8; 3] = [2, 2, 2];
        super::write(fds1[1], data1.as_ptr() as *const c_void, 3);
        super::write(fds2[1], data2.as_ptr() as *const c_void, 3);

        // 异步读两个管道
        let mut buf1: [u8; 4] = [0u8; 4];
        let mut buf2: [u8; 4] = [0u8; 4];
        let mut cb1: super::aiocb = unsafe { core::mem::zeroed() };
        let mut cb2: super::aiocb = unsafe { core::mem::zeroed() };
        cb1.aio_fildes = fds1[0];
        cb1.aio_buf = buf1.as_mut_ptr() as *mut c_void;
        cb1.aio_nbytes = buf1.len();
        cb2.aio_fildes = fds2[0];
        cb2.aio_buf = buf2.as_mut_ptr() as *mut c_void;
        cb2.aio_nbytes = buf2.len();

        super::aio_read(&mut cb1 as *mut super::aiocb);
        super::aio_read(&mut cb2 as *mut super::aiocb);

        // 等待任意一个完成
        let cbs: [*const super::aiocb; 2] = [
            &cb1 as *const super::aiocb,
            &cb2 as *const super::aiocb,
        ];
        let ret = super::aio_suspend(cbs.as_ptr(), 2, core::ptr::null());
        assert_eq!(ret, 0, "aio_suspend should return 0 when at least one op completes");

        // 至少一个应该已完成
        let err1 = super::aio_error(&cb1 as *const super::aiocb);
        let err2 = super::aio_error(&cb2 as *const super::aiocb);
        assert!(err1 == 0 || err2 == 0, "at least one operation should be complete");

        super::close(fds1[0]); super::close(fds1[1]);
        super::close(fds2[0]); super::close(fds2[1]);
    }
});

test!("test_aio_suspend_with_null_entries" {
    // aio_suspend: cbs 数组含 NULL 条目应被忽略
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 4] = [7, 7, 7, 7];
        super::write(fds[1], data.as_ptr() as *const c_void, 4);

        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        super::aio_read(&mut cb as *mut super::aiocb);

        // cbs 数组, 第 0 个为 NULL, 第 1 个为有效 cb
        let cbs: [*const super::aiocb; 2] = [
            core::ptr::null(),
            &cb as *const super::aiocb,
        ];
        let ret = super::aio_suspend(cbs.as_ptr(), 2, core::ptr::null());
        assert_eq!(ret, 0, "aio_suspend should ignore NULL entries and return 0");

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 4, "should have read 4 bytes");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_aio_suspend_after_aio_error_not_einprogress" {
    // aio_suspend: 当 aio_error 已不是 EINPROGRESS 时应立即返回 0
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 写入数据使异步读操作快速完成
        let data: [u8; 3] = [0x99; 3];
        super::write(fds[1], data.as_ptr() as *const c_void, 3);

        let mut buf: [u8; 4] = [0u8; 4];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        super::aio_read(&mut cb as *mut super::aiocb);

        // 先调用 aio_suspend 等待完成
        let cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let ret = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(ret, 0, "first aio_suspend should return 0");

        // 再次调用 aio_suspend, 操作已完成, 应立即返回
        let ret2 = super::aio_suspend(cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(ret2, 0, "second aio_suspend should also return 0 immediately");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
