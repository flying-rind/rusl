use test_framework::test;

test!("test_isatty_invalid_fd" {
    {
        // -1 is an invalid fd, so isatty should return 0 (false)
        let ret = super::isatty(-1);
        assert_eq!(ret, 0);
    }
});

test!("test_isatty_std_fds" {
    {
        // STDIN, STDOUT, STDERR may or may not be ttys
        // depending on whether the test is run interactively or piped.
        // Just verify the functions don't crash and return 0 or 1.
        let stdin_tty = super::isatty(super::STDIN_FILENO);
        let stdout_tty = super::isatty(super::STDOUT_FILENO);
        let stderr_tty = super::isatty(super::STDERR_FILENO);
        assert!(stdin_tty == 0 || stdin_tty == 1);
        assert!(stdout_tty == 0 || stdout_tty == 1);
        assert!(stderr_tty == 0 || stderr_tty == 1);
    }
});
