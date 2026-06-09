use test_framework::test;

test!("framework_assert_true" {
    assert!(true);
});

test!("framework_arithmetic" {
    assert_eq!(2 + 2, 4);
    assert_ne!(3 * 7, 20);
});