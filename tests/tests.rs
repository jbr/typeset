use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    process::Termination,
};
use test_harness::test;
use type_set::{entry::Entry, TypeSet};

fn harness<T: Termination>(f: impl FnOnce() -> T) -> T {
    let _ = env_logger::builder().is_test(true).try_init();
    f()
}

struct MyCustomStruct;

#[test(harness)]
fn debug() {
    let mut set = TypeSet::new()
        .with("hello")
        .with(1usize)
        .with(MyCustomStruct);

    assert_eq!(
        r#"TypeSet({"&str", "tests::MyCustomStruct", "usize"})"#,
        format!("{set:?}")
    );

    assert_eq!(
        "Occupied(OccupiedEntry<&str>(\"hello\"))",
        format!("{:?}", set.entry::<&'static str>())
    );

    assert_eq!(
        "Vacant(VacantEntry<alloc::string::String>)",
        format!("{:?}", set.entry::<String>())
    );
}

#[test(harness)]
fn smoke() {
    let mut set = TypeSet::new();
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    assert!(!set.contains::<bool>());
    set.insert(true);
    assert!(set.contains::<bool>());
    assert!(!set.is_empty());
    assert_eq!(set.len(), 1);
    assert!(set.get::<bool>().unwrap());
    set.insert(false);
    assert_eq!(set.len(), 1);
    assert!(!set.get::<bool>().unwrap());

    assert_eq!(*set.entry().or_insert("hello"), "hello");
    set.insert(String::from("hello"));
    assert_eq!(
        *set.entry()
            .and_modify(|h: &mut String| h.push_str(" world"))
            .or_default(),
        "hello world"
    );

    set.get_mut::<String>().unwrap().make_ascii_uppercase();
    assert_eq!(*set.get_or_insert(String::from("unused")), "HELLO WORLD");
    assert_eq!(
        *set.get_or_insert_with(|| String::from("unused")),
        "HELLO WORLD"
    );
    assert_eq!(*set.get_or_insert_default::<String>(), "HELLO WORLD");
    assert_eq!(set.take::<String>().unwrap(), "HELLO WORLD");
    assert_eq!(set.take::<String>(), None);
}

#[test(harness)]
fn merge() {
    let mut set_a = TypeSet::new().with(8u8).with("hello");
    let set_b = TypeSet::new().with(32u32).with("world");
    set_a.merge(set_b);
    assert_eq!(set_a.get::<u8>(), Some(&8));
    assert_eq!(set_a.get::<u32>(), Some(&32));
    assert_eq!(set_a.get::<&'static str>(), Some(&"world"));
    assert_eq!(set_a.len(), 3);
}

#[test(harness)]
fn entry() {
    let mut set = TypeSet::new();
    let entry = set.entry::<String>();
    assert!(entry.is_empty());
    let vacant_entry = entry.unwrap_vacant();
    vacant_entry.insert("hello".into());

    let vacant = set.entry::<usize>().unwrap_vacant();
    assert!(Entry::from(vacant).is_empty()); // sure it's a bit contrived

    let mut occupied_entry = set.entry::<String>().unwrap_occupied();
    assert_eq!(&**occupied_entry, "hello"); //deref
    assert_eq!(occupied_entry.get(), "hello");
    occupied_entry.get_mut().push_str(" world");
    occupied_entry.make_ascii_uppercase(); //deref mut

    set.entry::<String>().into_mut().unwrap().push('!');

    assert_eq!(set.entry::<String>().take().unwrap(), "HELLO WORLD!");

    assert!(set.entry::<String>().into_occupied().is_none());
    let vacant = set.entry::<String>();

    assert_eq!(
        *catch_unwind(AssertUnwindSafe(move || { vacant.unwrap_occupied() }))
            .unwrap_err()
            .downcast::<String>()
            .unwrap(),
        "expected an occupied type-set entry for alloc::string::String, but was vacant"
    );

    assert_eq!(*set.entry::<usize>().or_insert(10), 10);
    assert_eq!(
        *set.entry()
            .and_modify(|x: &mut usize| *x += 10)
            .or_default(),
        20
    );

    let occupied = set.entry::<usize>();
    assert_eq!(
        *catch_unwind(AssertUnwindSafe(move || { occupied.unwrap_vacant() }))
            .unwrap_err()
            .downcast::<String>()
            .unwrap(),
        "expected a vacant type-set entry for usize, but was occupied"
    );

    assert_eq!(
        *set.entry::<String>()
            .and_modify(|_| panic!("never called"))
            .or_insert_with(|| String::from("hello")),
        "hello"
    );

    assert!(!Entry::from(set.entry::<String>().unwrap_occupied()).is_empty())
}
