use std::{cell::Cell, rc::Rc, sync::atomic::AtomicUsize};

pub struct Store {
    root: Node,
    generation: usize,
    store_id: usize,
}

impl Store {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static STORE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        Store {
            root: Node(Rc::new(Cell::new(NodeData::Mem))),
            generation: 0,
            store_id: STORE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }

    pub fn set<T: 'static + Clone>(&mut self, r: &Ref<T>, value: T) {
        if self.generation == r.0.generation.get() {
            r.0.value.set(value);
        } else {
            let new_root = Node(Rc::new(Cell::new(NodeData::Mem)));
            let old_root = std::mem::replace(&mut self.root, new_root.clone());
            old_root.0.replace(NodeData::Diff(Box::new(Diff {
                r: r.clone(),
                value: r.0.value.replace(value),
                generation: r.0.generation.replace(self.generation),
                parent: new_root,
            })));
        }
    }

    pub fn capture(&mut self) -> Snapshot {
        let snap = Snapshot {
            root: self.root.clone(),
            generation: self.generation,
            store_id: self.store_id,
        };
        self.generation += 1;
        snap
    }

    pub fn restore(&mut self, snap: Snapshot) {
        if snap.store_id != self.store_id {
            panic!("Cannot restore from a snapshot from a different store");
        }
        if let NodeData::Mem = unsafe { &*snap.root.0.as_ptr() } {
            return;
        }
        reroot(&snap.root);
        self.root = snap.root;
        self.generation = snap.generation + 1;
    }
}

pub struct Ref<T>(Rc<RefInner<T>>);

impl<T> Ref<T> {
    pub fn new(value: T) -> Self {
        Ref(Rc::new(RefInner {
            value: Cell::new(value),
            generation: Cell::new(0),
        }))
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        unsafe { &*self.0.value.as_ptr() }.clone()
    }

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

#[derive(Clone)]
pub struct Snapshot {
    root: Node,
    generation: usize,
    store_id: usize,
}

#[derive(Clone)]
struct Node(Rc<Cell<NodeData>>);

enum NodeData {
    Mem,
    Diff(Box<dyn ReRoot>),
}

struct Diff<T> {
    r: Ref<T>,
    value: T,
    generation: usize,
    parent: Node,
}

trait ReRoot {
    fn reroot(&self, this: Node, parent: &Node);
    fn parent(&self) -> &Node;
}

impl<T: 'static + Clone> ReRoot for Diff<T> {
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

fn reroot(mut n: &Node) {
    let mut stack = vec![];
    loop {
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
