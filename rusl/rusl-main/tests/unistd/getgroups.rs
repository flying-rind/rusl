use test_framework::test;

test!("test_getgroups_count_zero" {
    {
        // count=0 returns the number of supplementary group IDs without modifying list
        let count = super::getgroups(0, core::ptr::null_mut());
        assert!(count >= 0);
    }
});

test!("test_getgroups_with_buffer" {
    {
        let mut list: [u32; 64] = [0u32; 64];
        let count = super::getgroups(64, list.as_mut_ptr());
        assert!(count >= 0);
        assert!(count <= 64);
    }
});

test!("test_getgroups_zero_count_list" {
    {
        // count=0 with a non-null buffer is also valid
        let mut list: [u32; 1] = [0u32; 1];
        let prev = list[0];
        let count = super::getgroups(0, list.as_mut_ptr());
        assert!(count >= 0);
        // list should not be modified when count=0
        assert_eq!(list[0], prev);
    }
});
