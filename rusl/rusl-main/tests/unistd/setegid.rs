use test_framework::test;

test!("test_setegid_same_egid" {
    {
        let egid = super::getegid();
        let ret = super::setegid(egid);
        assert_eq!(ret, 0);
    }
});
