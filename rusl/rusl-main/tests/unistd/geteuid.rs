use test_framework::test;

test!("test_geteuid_basic" {
    {
        let euid = super::geteuid();
        // geteuid always succeeds; euid is a u32
        let _ = euid;
    }
});

test!("test_geteuid_idempotent" {
    {
        let euid1 = super::geteuid();
        let euid2 = super::geteuid();
        assert_eq!(euid1, euid2);
    }
});
