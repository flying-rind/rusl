use test_framework::test;

test!("test_seteuid_same_euid" {
    {
        let euid = super::geteuid();
        let ret = super::seteuid(euid);
        assert_eq!(ret, 0);
    }
});
