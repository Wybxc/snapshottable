#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec};
use core::{cell::Cell, sync::atomic::AtomicUsize};

/// A bag of mutable objects (references) with snapshot and restore capabilities.
pub struct Store {
    root: Node,
    generation: usize,
    store_id: usize,
}

impl Store {
    /// Creates a new `Store` with an initial empty mapping.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static STORE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        Store {
            root: Node(Rc::new(Cell::new(NodeData::Mem))),
            generation: 0,
            store_id: STORE_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
        }
    }

    /// Sets the value of a reference `r` inside this store.
    pub fn set<T: 'static + Clone>(&mut self, r: &Ref<T>, value: T) {
        if self.generation == r.0.generation.get() {
            r.0.value.set(value);
        } else {
            let new_root = Node(Rc::new(Cell::new(NodeData::Mem)));
            let old_root = core::mem::replace(&mut self.root, new_root.clone());
            old_root.0.replace(NodeData::Diff(Box::new(Diff {
                r: r.clone(),
                value: r.0.value.replace(value),
                generation: r.0.generation.replace(self.generation),
                parent: new_root,
            })));
        }
    }

    /// Captures the current state of all references in a `Snapshot`.
    pub fn capture(&mut self) -> Snapshot {
        let snap = Snapshot {
            root: self.root.clone(),
            generation: self.generation,
            store_id: self.store_id,
        };
        self.generation += 1;
        snap
    }

    /// Restores the references to the exact state they were in when this
    /// `Snapshot` was taken.
    ///
    /// Restoring panics if you try to restore a snapshot spanning from a
    /// different store instance.
    pub fn restore(&mut self, snap: Snapshot) {
        if snap.store_id != self.store_id {
            panic!("Cannot restore from a snapshot from a different store");
        }
        // SAFETY: There are no mutable references to the store's internal graph.
        if let NodeData::Mem = unsafe { &*snap.root.0.as_ptr() } {
            return;
        }
        reroot(&snap.root);
        self.root = snap.root;
        self.generation = snap.generation + 1;
    }
}

/// A snapshot-aware mutable reference to a value.
pub struct Ref<T>(Rc<RefInner<T>>);

impl<T> Ref<T> {
    /// Creates a new, detached reference wrapping the given `value`.
    pub fn new(value: T) -> Self {
        Ref(Rc::new(RefInner {
            value: Cell::new(value),
            generation: Cell::new(0),
        }))
    }

    /// Fetches and clones the current value of this reference.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        // SAFETY: There are no mutable references to Ref's internal cell.
        unsafe { &*self.0.value.as_ptr() }.clone()
    }

    /// Sets the value of this reference in the provided `Store`.
    pub fn set(&self, store: &mut Store, value: T)
    where
        T: Clone + 'static,
    {
        store.set(self, value);
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Ref(Rc::clone(&self.0))
    }
}

struct RefInner<T> {
    value: Cell<T>,
    generation: Cell<usize>,
}

/// An opaque handle recording a captured version of the store references.
#[derive(Clone)]
pub struct Snapshot {
    root: Node,
    generation: usize,
    store_id: usize,
}

// Internal Representation of history graph. The `Node` tree structure maps state
// generations recursively. A `Mem` node represents the "current" memory baseline.
// When traversing a `Diff` back, the old values update the current references.
#[derive(Clone)]
struct Node(Rc<Cell<NodeData>>);

enum NodeData {
    // Defines the current branch point. Exactly ONE `Mem` node always exists as
    // the globally "tracked" active graph head inside the store.
    Mem,
    // Holds the dynamic boxed callback trait back-linking graph layers.
    Diff(Box<dyn ReRoot>),
}

struct Diff<T> {
    r: Ref<T>,
    // The previous generation's baseline value.
    value: T,
    // The previously tracked generator epoch.
    generation: usize,
    // Ascends via reversed pointers towards the older ancestor mapping.
    parent: Node,
}

// Dynamic dispatch fallback enforcing type-erasure mapping so single node histories
// can backtrack values universally without storing type signatures throughout Node traversal.
trait ReRoot {
    fn reroot(&self, this: Node, parent: &Node);
    fn parent(&self) -> &Node;
}

impl<T: 'static + Clone> ReRoot for Diff<T> {
    // Unwinds the captured specific mutation into the references underlying cell
    // and updates reverse graph edges mapping it to its newer successor snapshot tree.
    fn reroot(&self, this: Node, parent: &Node) {
        assert!(Rc::ptr_eq(&self.parent.0, &parent.0));
        parent.0.replace(NodeData::Diff(Box::new(Diff {
            r: self.r.clone(),
            value: self.r.0.value.replace(self.value.clone()),
            generation: self.r.0.generation.replace(self.generation),
            parent: this,
        })));
    }

    fn parent(&self) -> &Node {
        &self.parent
    }
}

// Crawls from the arbitrary snapshot `n` up the parent chains, collecting
// diff inversions into `Mem`. Flushes the inverted stack pushing values back to
// the active globally-viewed variables and rotating the `Mem` node backwards.
fn reroot(mut n: &Node) {
    let mut stack = vec![];
    loop {
        // SAFETY: There are no mutable references to the store's internal graph.
        match unsafe { &*n.0.as_ptr() } {
            NodeData::Mem => break,
            NodeData::Diff(diff) => {
                stack.push((diff, n));
                n = diff.parent();
            }
        }
    }
    while let Some((diff, node)) = stack.pop() {
        diff.reroot(node.clone(), n);
        n = node;
    }
    n.0.replace(NodeData::Mem);
}
