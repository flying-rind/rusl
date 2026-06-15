use test_framework::test;

test!("test_setuid_same_uid" {
    {
        let uid = super::getuid();
        let ret = super::setuid(uid);
        assert_eq!(ret, 0);
    }
});
