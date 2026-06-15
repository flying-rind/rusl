use test_framework::test;

test!("test_setgid_same_gid" {
    {
        let gid = super::getgid();
        let ret = super::setgid(gid);
        assert_eq!(ret, 0);
    }
});
