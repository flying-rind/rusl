use test_framework::test;

test!("test_setreuid_no_change" {
    {
        // -1u32 means "don't change" for both ruid and euid
        let ret = super::setreuid(!0u32, !0u32);
        assert_eq!(ret, 0);
    }
});

test!("test_setreuid_same_ids" {
    {
        let uid = super::getuid();
        let euid = super::geteuid();
        let ret = super::setreuid(uid, euid);
        assert_eq!(ret, 0);
    }
});
