use std::panic::{AssertUnwindSafe, catch_unwind};

use snapshottable::{Ref, Store};

#[test]
fn ref_set_and_store_set_update_value() {
    let mut store = Store::new();
    let r = Ref::new(1_i32);

    assert_eq!(r.get(), 1);

    r.set(&mut store, 2);
    assert_eq!(r.get(), 2);

    store.set(&r, 3);
    assert_eq!(r.get(), 3);
}

#[test]
fn restore_current_snapshot_is_noop() {
    let mut store = Store::new();
    let r = Ref::new(String::from("v0"));

    let snap = store.capture();
    store.restore(snap);

    assert_eq!(r.get(), "v0");
    r.set(&mut store, String::from("v1"));
    assert_eq!(r.get(), "v1");
}

#[test]
fn restore_from_other_store_panics() {
    let mut store1 = Store::new();
    let mut store2 = Store::new();

    let snap = store1.capture();

    let result = catch_unwind(AssertUnwindSafe(|| {
        store2.restore(snap);
    }));

    assert!(result.is_err());
}

#[test]
fn cloned_ref_points_to_same_cell() {
    let mut store = Store::new();
    let r1 = Ref::new(10_usize);
    let r2 = r1.clone();

    r2.set(&mut store, 20);
    assert_eq!(r1.get(), 20);

    store.set(&r1, 30);
    assert_eq!(r2.get(), 30);
}

#[test]
fn multiple_snapshots_can_be_restored_without_changes() {
    let mut store = Store::new();
    let r = Ref::new(0_i32);

    let s0 = store.capture();
    let s1 = store.capture();

    store.restore(s1);
    assert_eq!(r.get(), 0);

    store.restore(s0);
    assert_eq!(r.get(), 0);
}

#[test]
fn restore_after_mutation_reverts_value() {
    let mut store = Store::new();
    let r = Ref::new(1_i32);

    let s0 = store.capture();
    r.set(&mut store, 2);
    assert_eq!(r.get(), 2);

    store.restore(s0);
    assert_eq!(r.get(), 1);
}

#[test]
fn restore_old_snapshot_after_multiple_mutations_reverts() {
    let mut store = Store::new();
    let r = Ref::new(10_i32);

    let s0 = store.capture();
    r.set(&mut store, 11);
    let s1 = store.capture();
    r.set(&mut store, 12);

    assert_eq!(r.get(), 12);

    store.restore(s1);
    assert_eq!(r.get(), 11);

    store.restore(s0);
    assert_eq!(r.get(), 10);
}

#[test]
fn restore_reverts_multiple_refs_together() {
    let mut store = Store::new();
    let a = Ref::new(String::from("A0"));
    let b = Ref::new(100_i32);

    let s0 = store.capture();
    a.set(&mut store, String::from("A1"));
    b.set(&mut store, 101);

    assert_eq!(a.get(), "A1");
    assert_eq!(b.get(), 101);

    store.restore(s0);

    assert_eq!(a.get(), "A0");
    assert_eq!(b.get(), 100);
}
