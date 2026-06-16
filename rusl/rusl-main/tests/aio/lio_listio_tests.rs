//! lio_listio 集成测试
//!
//! 测试函数: lio_listio

use core::ffi::c_void;
use test_framework::test;

// ============================================================================
// lio_listio 错误处理测试
// ============================================================================

test!("test_lio_listio_invalid_cnt" {
    // lio_listio 错误处理: cnt < 0 应返回 -1 (EINVAL)
    {
        let ret = super::lio_listio(
            super::LIO_WAIT,
            core::ptr::null(),
            -1,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, -1, "lio_listio with cnt=-1 should return -1 (EINVAL)");
    }
});

test!("test_lio_listio_invalid_cnt_negative" {
    // lio_listio 错误处理: cnt = -5 应返回 -1
    {
        let ret = super::lio_listio(
            super::LIO_NOWAIT,
            core::ptr::null(),
            -5,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, -1, "lio_listio with cnt=-5 should return -1 (EINVAL)");
    }
});

test!("test_lio_listio_zero_cnt" {
    // lio_listio: cnt = 0 应返回 0 (空操作)
    {
        let ret = super::lio_listio(
            super::LIO_WAIT,
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio with cnt=0 should return 0");
    }
});

test!("test_lio_listio_zero_cnt_nowait" {
    // lio_listio: cnt = 0, LIO_NOWAIT 应返回 0
    {
        let ret = super::lio_listio(
            super::LIO_NOWAIT,
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio with cnt=0 and LIO_NOWAIT should return 0");
    }
});

// ============================================================================
// lio_listio LIO_WAIT (同步阻塞) 测试
// ============================================================================

test!("test_lio_listio_wait_read" {
    // lio_listio: LIO_WAIT 模式, 单个异步读操作
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入数据到管道
        let data: [u8; 5] = [0x50, 0x51, 0x52, 0x53, 0x54];
        let wn = super::write(fds[1], data.as_ptr() as *const c_void, 5);
        assert_eq!(wn, 5, "write to pipe should succeed");

        // 配置 aiocb
        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_lio_opcode = super::LIO_READ;
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        // LIO_WAIT 模式
        let cbs: [*mut super::aiocb; 1] = [&mut cb as *mut super::aiocb];
        let ret = super::lio_listio(
            super::LIO_WAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio with LIO_WAIT should return 0");

        // 验证数据读取正确
        assert_eq!(&buf[..5], &data[..], "read data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lio_listio_wait_write" {
    // lio_listio: LIO_WAIT 模式, 单个异步写操作
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 5] = [0xA0, 0xA1, 0xA2, 0xA3, 0xA4];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[1];
        cb.aio_lio_opcode = super::LIO_WRITE;
        cb.aio_buf = data.as_ptr() as *mut c_void;
        cb.aio_nbytes = data.len();

        let cbs: [*mut super::aiocb; 1] = [&mut cb as *mut super::aiocb];
        let ret = super::lio_listio(
            super::LIO_WAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio LIO_WAIT write should return 0");

        // 从管道读端验证
        let mut rbuf: [u8; 8] = [0u8; 8];
        let rn = super::read(fds[0], rbuf.as_mut_ptr() as *mut c_void, rbuf.len());
        assert_eq!(rn, 5, "should read back 5 bytes");
        assert_eq!(&rbuf[..5], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lio_listio_wait_nop" {
    // lio_listio: LIO_WAIT 模式, LIO_NOP 操作 (空操作应被跳过)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_lio_opcode = super::LIO_NOP;
        cb.aio_nbytes = 0;

        let cbs: [*mut super::aiocb; 1] = [&mut cb as *mut super::aiocb];
        let ret = super::lio_listio(
            super::LIO_WAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio with LIO_NOP should return 0");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lio_listio_wait_read_write" {
    // lio_listio: LIO_WAIT 模式, 同时异步读写两个管道
    {
        let mut fds1: [super::c_int; 2] = [-1, -1];
        let mut fds2: [super::c_int; 2] = [-1, -1];
        super::pipe(fds1.as_mut_ptr());
        super::pipe(fds2.as_mut_ptr());

        // 向管道1写入数据 (供读取)
        let data1: [u8; 4] = [1, 2, 3, 4];
        super::write(fds1[1], data1.as_ptr() as *const c_void, 4);

        // 配置两个 aiocb: 一个读取管道1, 一个写入管道2
        let mut rbuf: [u8; 8] = [0u8; 8];
        let mut rcb: super::aiocb = unsafe { core::mem::zeroed() };
        rcb.aio_fildes = fds1[0];
        rcb.aio_lio_opcode = super::LIO_READ;
        rcb.aio_buf = rbuf.as_mut_ptr() as *mut c_void;
        rcb.aio_nbytes = rbuf.len();

        let wdata: [u8; 3] = [0xDD, 0xEE, 0xFF];
        let mut wcb: super::aiocb = unsafe { core::mem::zeroed() };
        wcb.aio_fildes = fds2[1];
        wcb.aio_lio_opcode = super::LIO_WRITE;
        wcb.aio_buf = wdata.as_ptr() as *mut c_void;
        wcb.aio_nbytes = wdata.len();

        let cbs: [*mut super::aiocb; 2] = [
            &mut rcb as *mut super::aiocb,
            &mut wcb as *mut super::aiocb,
        ];
        let ret = super::lio_listio(
            super::LIO_WAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            2,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio with read+write should return 0");

        // 验证读取结果
        assert_eq!(&rbuf[..4], &data1[..], "read data should match");

        // 验证写入结果
        let mut verify_buf: [u8; 8] = [0u8; 8];
        let rn = super::read(fds2[0], verify_buf.as_mut_ptr() as *mut c_void, verify_buf.len());
        assert_eq!(rn, 3, "should read back 3 bytes from pipe2");
        assert_eq!(&verify_buf[..3], &wdata[..], "written data should match");

        super::close(fds1[0]); super::close(fds1[1]);
        super::close(fds2[0]); super::close(fds2[1]);
    }
});

// ============================================================================
// lio_listio LIO_NOWAIT (异步) 测试
// ============================================================================

test!("test_lio_listio_nowait_read" {
    // lio_listio: LIO_NOWAIT 模式, 单个异步读 (立即返回 0)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入数据
        let data: [u8; 4] = [0x61, 0x62, 0x63, 0x64];
        super::write(fds[1], data.as_ptr() as *const c_void, 4);

        let mut buf: [u8; 8] = [0u8; 8];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[0];
        cb.aio_lio_opcode = super::LIO_READ;
        cb.aio_buf = buf.as_mut_ptr() as *mut c_void;
        cb.aio_nbytes = buf.len();

        let cbs: [*mut super::aiocb; 1] = [&mut cb as *mut super::aiocb];
        let ret = super::lio_listio(
            super::LIO_NOWAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio LIO_NOWAIT should return 0 immediately");

        // 等待异步读完成
        let wait_cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(wait_cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0 when read completes");

        // 验证读取结果
        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should be 0 after read completes");

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 4, "should have read 4 bytes");
        assert_eq!(&buf[..4], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lio_listio_nowait_write" {
    // lio_listio: LIO_NOWAIT 模式, 单个异步写 (立即返回 0)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 4] = [0xF1, 0xF2, 0xF3, 0xF4];
        let mut cb: super::aiocb = unsafe { core::mem::zeroed() };
        cb.aio_fildes = fds[1];
        cb.aio_lio_opcode = super::LIO_WRITE;
        cb.aio_buf = data.as_ptr() as *mut c_void;
        cb.aio_nbytes = data.len();

        let cbs: [*mut super::aiocb; 1] = [&mut cb as *mut super::aiocb];
        let ret = super::lio_listio(
            super::LIO_NOWAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio LIO_NOWAIT write should return 0 immediately");

        // 等待写完成
        let wait_cbs: [*const super::aiocb; 1] = [&cb as *const super::aiocb];
        let suspend_ret = super::aio_suspend(wait_cbs.as_ptr(), 1, core::ptr::null());
        assert_eq!(suspend_ret, 0, "aio_suspend should return 0");

        let err = super::aio_error(&cb as *const super::aiocb);
        assert_eq!(err, 0, "aio_error should be 0 after write completes");

        let n = super::aio_return(&mut cb as *mut super::aiocb);
        assert_eq!(n, 4, "should have written 4 bytes");

        // 验证数据
        let mut rbuf: [u8; 8] = [0u8; 8];
        let rn = super::read(fds[0], rbuf.as_mut_ptr() as *mut c_void, rbuf.len());
        assert_eq!(rn, 4, "should read back 4 bytes");
        assert_eq!(&rbuf[..4], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lio_listio_nowait_mix" {
    // lio_listio: LIO_NOWAIT 模式, 混合 LIO_READ、LIO_WRITE、LIO_NOP
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 同步写入数据 (供后面异步读取)
        let data: [u8; 3] = [0x10, 0x20, 0x30];
        super::write(fds[1], data.as_ptr() as *const c_void, 3);

        // 配置三个 aiocb: NOP, READ, WRITE
        let mut nop_cb: super::aiocb = unsafe { core::mem::zeroed() };
        nop_cb.aio_fildes = fds[0];
        nop_cb.aio_lio_opcode = super::LIO_NOP;

        let mut rbuf: [u8; 4] = [0u8; 4];
        let mut read_cb: super::aiocb = unsafe { core::mem::zeroed() };
        read_cb.aio_fildes = fds[0];
        read_cb.aio_lio_opcode = super::LIO_READ;
        read_cb.aio_buf = rbuf.as_mut_ptr() as *mut c_void;
        read_cb.aio_nbytes = rbuf.len();

        let wdata: [u8; 2] = [0xAA, 0xBB];
        let mut write_cb: super::aiocb = unsafe { core::mem::zeroed() };
        write_cb.aio_fildes = fds[1];
        write_cb.aio_lio_opcode = super::LIO_WRITE;
        write_cb.aio_buf = wdata.as_ptr() as *mut c_void;
        write_cb.aio_nbytes = wdata.len();

        let cbs: [*mut super::aiocb; 3] = [
            &mut nop_cb as *mut super::aiocb,
            &mut read_cb as *mut super::aiocb,
            &mut write_cb as *mut super::aiocb,
        ];
        let ret = super::lio_listio(
            super::LIO_NOWAIT,
            cbs.as_ptr() as *const *mut super::aiocb,
            3,
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "lio_listio LIO_NOWAIT with mixed ops should return 0");

        // 等待读和写都完成
        let wait_cbs: [*const super::aiocb; 2] = [
            &read_cb as *const super::aiocb,
            &write_cb as *const super::aiocb,
        ];

        // 等待第一个完成
        super::aio_suspend(wait_cbs.as_ptr(), 2, core::ptr::null());
        // 等待第二个完成 (如果第一个 suspend 后另一个还没完成)
        let rerr = super::aio_error(&read_cb as *const super::aiocb);
        let werr = super::aio_error(&write_cb as *const super::aiocb);
        if rerr == super::EINPROGRESS {
            let scbs: [*const super::aiocb; 1] = [&read_cb as *const super::aiocb];
            super::aio_suspend(scbs.as_ptr(), 1, core::ptr::null());
        }
        if werr == super::EINPROGRESS {
            let scbs: [*const super::aiocb; 1] = [&write_cb as *const super::aiocb];
            super::aio_suspend(scbs.as_ptr(), 1, core::ptr::null());
        }

        // 验证读取
        let rn = super::aio_return(&mut read_cb as *mut super::aiocb);
        assert_eq!(rn, 3, "should have read 3 bytes");
        assert_eq!(&rbuf[..3], &data[..], "read data should match");

        // 验证写入
        let wn = super::aio_return(&mut write_cb as *mut super::aiocb);
        assert_eq!(wn, 2, "should have written 2 bytes");

        // 验证写入的数据可从管道读出 (注意顺序: 先写入的 data(3字节), 再写入的 wdata(2字节))
        let mut verify_buf: [u8; 8] = [0u8; 8];
        // 先读掉之前同步写入的 3 字节 (已被异步读掉), 再读异步写的 2 字节
        let vn = super::read(fds[0], verify_buf.as_mut_ptr() as *mut c_void, verify_buf.len());
        assert_eq!(vn, 2, "should read back 2 bytes (async write data)");
        assert_eq!(&verify_buf[..2], &wdata[..], "written data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
