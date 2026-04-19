// Thanks to @hanna-kruppe for finding this unsoundness in the original implementation of `snapshottable` and providing a test case for it.

use snapshottable::{Ref, Store, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Weird {
    store: Weak<RefCell<Store>>,
    r: Option<WeakRef<Weird>>,
}

impl Clone for Weird {
    fn clone(&self) -> Self {
        let make_clone = || Weird {
            store: self.store.clone(),
            r: self.r.clone(),
        };
        // if we have the self-reference...
        if let Some(r) = &self.r {
            let store = self.store.upgrade().unwrap();
            let mut store = store.borrow_mut();
            // ... use it to trigger mutation of the cell contents that self refers to
            store.set(&r.upgrade().unwrap(), make_clone());
        }
        make_clone()
    }
}

#[test]
#[should_panic = "RefCell already borrowed"]
fn clone_safe() {
    let store = Rc::new(RefCell::new(Store::new()));
    let r = Ref::new(Weird {
        store: Rc::downgrade(&store),
        r: None,
    });
    // tie the knot: value behind `r` gets access to `r`
    let cycle = Weird {
        store: Rc::downgrade(&store),
        r: Some(Ref::downgrade(&r)),
    };
    store.borrow_mut().set(&r, cycle);
    // trigger Weird::clone -> mutation of Cell in store while it's aliased
    r.get();
}
