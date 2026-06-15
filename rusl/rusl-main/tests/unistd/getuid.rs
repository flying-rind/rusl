use test_framework::test;

test!("test_getuid_basic" {
    {
        let uid = super::getuid();
        // getuid always succeeds; uid is a u32
        let _ = uid;
    }
});

test!("test_getuid_idempotent" {
    {
        let uid1 = super::getuid();
        let uid2 = super::getuid();
        assert_eq!(uid1, uid2);
    }
});
