use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::{Mutex, RwLock};

/// Trait for types that can contain Gc references.
/// Implementors report their child Gc IDs for the mark phase.
pub trait Traceable {
    fn trace(&self, _children: &mut Vec<usize>) {}
}

/// A garbage-collected smart pointer.
///
/// `Gc<T>` provides interior mutability through `RefCell` semantics.
/// The underlying object lives on the `GcHeap` and is accessed via
/// a heap reference and unique ID.
pub struct Gc<T: Traceable + 'static> {
    id: usize,
    heap: Arc<GcHeap>,
    _marker: PhantomData<T>,
}

impl<T: Traceable + Send + Sync + 'static> Gc<T> {
    /// Allocate a new GC-managed object. The object is initially rooted.
    pub fn new(heap: &Arc<GcHeap>, value: T) -> Self {
        let id = heap.allocate(value);
        heap.add_root(id);
        Self {
            id,
            heap: Arc::clone(heap),
            _marker: PhantomData,
        }
    }

    /// Access the inner value immutably via a closure.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let objects = self.heap.objects.read();
        let obj = objects
            .get(&self.id)
            .expect("Gc refers to collected object");
        let inner = obj.value.lock();
        let borrow = inner.borrow();
        let val = borrow.downcast_ref::<T>().expect("Gc type mismatch");
        f(val)
    }

    /// Access the inner value mutably via a closure.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let objects = self.heap.objects.read();
        let obj = objects
            .get(&self.id)
            .expect("Gc refers to collected object");
        let inner = obj.value.lock();
        let mut borrow = inner.borrow_mut();
        let val = borrow.downcast_mut::<T>().expect("Gc type mismatch");
        f(val)
    }

    /// Get the unique ID of this GC allocation.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Register another Gc as a child of this object.
    /// This is the write barrier: call this when storing a Gc inside another Gc.
    pub fn store(&self, child_id: usize) {
        self.heap.register_child(self.id, child_id);
    }

    /// Remove this object from the root set (e.g., when going out of scope).
    pub fn unroot(&self) {
        self.heap.remove_root(self.id);
    }

    /// Re-add this object to the root set.
    pub fn root(&self) {
        self.heap.add_root(self.id);
    }
}

impl<T: Traceable + Send + Sync + 'static> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            heap: Arc::clone(&self.heap),
            _marker: PhantomData,
        }
    }
}

/// Internal representation of a GC-managed object.
struct GcObject {
    value: Mutex<std::cell::RefCell<Box<dyn Any + Send + Sync>>>,
    marked: AtomicBool,
}

/// The garbage collector heap.
///
/// Manages all GC allocations, the root set, and parent-child relationships.
/// A background thread periodically runs mark-sweep collection.
pub struct GcHeap {
    objects: RwLock<HashMap<usize, GcObject>>,
    children: RwLock<HashMap<usize, Vec<usize>>>,
    roots: RwLock<HashSet<usize>>,
    next_id: AtomicUsize,
    /// Set by the collector during sweep to pause mutator briefly.
    sweeping: AtomicBool,
    /// Configuration: trigger collection after this many allocations.
    alloc_threshold: AtomicUsize,
    /// Counter of allocations since last collection.
    alloc_count: AtomicUsize,
}

impl GcHeap {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            objects: RwLock::new(HashMap::new()),
            children: RwLock::new(HashMap::new()),
            roots: RwLock::new(HashSet::new()),
            next_id: AtomicUsize::new(1),
            sweeping: AtomicBool::new(false),
            alloc_threshold: AtomicUsize::new(1000),
            alloc_count: AtomicUsize::new(0),
        })
    }

    /// Allocate a new object on the heap. Returns its unique ID.
    fn allocate<T: Traceable + Send + Sync + 'static>(&self, value: T) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let obj = GcObject {
            value: Mutex::new(std::cell::RefCell::new(Box::new(value))),
            marked: AtomicBool::new(false),
        };
        self.objects.write().insert(id, obj);
        self.children.write().insert(id, Vec::new());
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
        id
    }

    fn add_root(&self, id: usize) {
        self.roots.write().insert(id);
    }

    fn remove_root(&self, id: usize) {
        self.roots.write().remove(&id);
    }

    /// Register a parent-child relationship (write barrier).
    fn register_child(&self, parent_id: usize, child_id: usize) {
        let mut children = self.children.write();
        children
            .entry(parent_id)
            .or_insert_with(Vec::new)
            .push(child_id);
    }

    /// Check if collection should be triggered.
    pub fn should_collect(&self) -> bool {
        self.alloc_count.load(Ordering::Relaxed) >= self.alloc_threshold.load(Ordering::Relaxed)
    }

    /// Run a mark-sweep collection cycle.
    /// This is called by the background collector thread.
    pub fn collect(&self) -> usize {
        // Phase 1: Mark
        self.mark();

        // Phase 2: Sweep
        let freed = self.sweep();

        // Reset allocation counter
        self.alloc_count.store(0, Ordering::Relaxed);

        freed
    }

    /// Mark all reachable objects starting from roots.
    fn mark(&self) {
        // Snapshot roots
        let roots: Vec<usize> = self.roots.read().iter().copied().collect();

        // BFS mark
        let mut queue: VecDeque<usize> = VecDeque::new();
        for root in &roots {
            queue.push_back(*root);
        }

        let objects = self.objects.read();
        let children = self.children.read();

        while let Some(id) = queue.pop_front() {
            if let Some(obj) = objects.get(&id) {
                // Already marked? Skip.
                if obj.marked.load(Ordering::Relaxed) {
                    continue;
                }
                obj.marked.store(true, Ordering::Relaxed);

                // Enqueue children
                if let Some(child_ids) = children.get(&id) {
                    for child_id in child_ids {
                        if !objects
                            .get(child_id)
                            .map_or(false, |o| o.marked.load(Ordering::Relaxed))
                        {
                            queue.push_back(*child_id);
                        }
                    }
                }
            }
        }
    }

    /// Sweep unmarked objects. Returns the number of freed objects.
    fn sweep(&self) -> usize {
        let mut objects = self.objects.write();
        let mut children = self.children.write();

        let before = objects.len();

        // Collect IDs of unmarked objects
        let unmarked: Vec<usize> = objects
            .iter()
            .filter(|(_, obj)| !obj.marked.load(Ordering::Relaxed))
            .map(|(id, _)| *id)
            .collect();

        // Remove unmarked objects
        for id in &unmarked {
            objects.remove(id);
            children.remove(id);
        }

        // Reset marks on surviving objects
        for obj in objects.values() {
            obj.marked.store(false, Ordering::Relaxed);
        }

        before - objects.len()
    }

    /// Get the number of live objects.
    pub fn object_count(&self) -> usize {
        self.objects.read().len()
    }

    /// Start a background collector thread.
    /// Returns a handle and a shutdown flag.
    pub fn start_collector(heap: Arc<Self>, interval: Duration) -> CollectorHandle {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let heap_clone = Arc::clone(&heap);

        let handle = thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                thread::sleep(interval);

                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }

                if heap_clone.should_collect() {
                    let _freed = heap_clone.collect();
                }
            }
        });

        CollectorHandle {
            handle: Some(handle),
            running,
        }
    }
}

impl Default for GcHeap {
    fn default() -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            children: RwLock::new(HashMap::new()),
            roots: RwLock::new(HashSet::new()),
            next_id: AtomicUsize::new(1),
            sweeping: AtomicBool::new(false),
            alloc_threshold: AtomicUsize::new(1000),
            alloc_count: AtomicUsize::new(0),
        }
    }
}

/// Handle to the background collector thread.
pub struct CollectorHandle {
    handle: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl CollectorHandle {
    /// Stop the collector thread and wait for it to finish.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for CollectorHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Traceable implementation for common container types.
impl<T: Traceable> Traceable for Vec<T> {
    fn trace(&self, children: &mut Vec<usize>) {
        for item in self {
            item.trace(children);
        }
    }
}

impl<K: Traceable, V: Traceable> Traceable for HashMap<K, V> {
    fn trace(&self, children: &mut Vec<usize>) {
        for (k, v) in self {
            k.trace(children);
            v.trace(children);
        }
    }
}

impl<T: Traceable> Traceable for Option<T> {
    fn trace(&self, children: &mut Vec<usize>) {
        if let Some(val) = self {
            val.trace(children);
        }
    }
}

impl<T: Traceable, E: Traceable> Traceable for Result<T, E> {
    fn trace(&self, children: &mut Vec<usize>) {
        match self {
            Ok(val) => val.trace(children),
            Err(err) => err.trace(children),
        }
    }
}

impl Traceable for String {}
impl Traceable for i64 {}
impl Traceable for f64 {}
impl Traceable for bool {}
impl Traceable for char {}
impl Traceable for usize {}
impl Traceable for () {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    struct Simple {
        value: i64,
    }
    impl Traceable for Simple {}

    #[derive(Debug)]
    struct WithChild {
        value: i64,
        child: Option<usize>, // Gc ID of child
    }
    impl Traceable for WithChild {
        fn trace(&self, children: &mut Vec<usize>) {
            if let Some(id) = self.child {
                children.push(id);
            }
        }
    }

    #[test]
    fn test_basic_alloc_and_borrow() {
        let heap = GcHeap::new();
        let gc = Gc::new(&heap, Simple { value: 42 });
        gc.with(|v| assert_eq!(v.value, 42));
    }

    #[test]
    fn test_mut_borrow() {
        let heap = GcHeap::new();
        let gc = Gc::new(&heap, Simple { value: 0 });
        gc.with_mut(|v| *v = Simple { value: 99 });
        gc.with(|v| assert_eq!(v.value, 99));
    }

    #[test]
    fn test_gc_collects_unreachable() {
        let heap = GcHeap::new();

        // Allocate and immediately drop the handle (unroot)
        {
            let gc = Gc::new(&heap, Simple { value: 1 });
            gc.unroot();
        }

        assert_eq!(heap.object_count(), 1);

        let freed = heap.collect();
        assert_eq!(freed, 1);
        assert_eq!(heap.object_count(), 0);
    }

    #[test]
    fn test_gc_preserves_rooted() {
        let heap = GcHeap::new();

        let gc = Gc::new(&heap, Simple { value: 1 });

        let freed = heap.collect();
        assert_eq!(freed, 0);
        gc.with(|v| assert_eq!(v.value, 1));
    }

    #[test]
    fn test_gc_follows_children() {
        let heap = GcHeap::new();

        let child = Gc::new(&heap, Simple { value: 100 });
        let parent = Gc::new(
            &heap,
            WithChild {
                value: 1,
                child: Some(child.id()),
            },
        );
        parent.store(child.id());
        child.unroot(); // child only reachable via parent

        let freed = heap.collect();
        assert_eq!(freed, 0);
        child.with(|v| assert_eq!(v.value, 100));
    }

    #[test]
    fn test_gc_collects_orphaned_child() {
        let heap = GcHeap::new();

        let child = Gc::new(&heap, Simple { value: 100 });
        let parent = Gc::new(
            &heap,
            WithChild {
                value: 1,
                child: Some(child.id()),
            },
        );
        parent.store(child.id());
        child.unroot();

        // Remove parent root → both become unreachable
        parent.unroot();

        let freed = heap.collect();
        assert_eq!(freed, 2);
        assert_eq!(heap.object_count(), 0);
    }

    #[test]
    fn test_background_collector() {
        let heap = GcHeap::new();
        let mut collector = GcHeap::start_collector(Arc::clone(&heap), Duration::from_millis(10));

        // Allocate many objects without rooting
        for i in 0..2000 {
            let gc = Gc::new(&heap, Simple { value: i });
            gc.unroot();
        }

        // Give collector time to run
        thread::sleep(Duration::from_millis(100));

        // Objects should be collected
        assert_eq!(heap.object_count(), 0);

        collector.shutdown();
    }

    #[test]
    fn test_cycle_detection_via_roots() {
        // Cycles are handled because we use explicit roots + children,
        // not reference counting. An unreachable cycle gets collected.
        let heap = GcHeap::new();

        let a = Gc::new(
            &heap,
            WithChild {
                value: 1,
                child: None,
            },
        );
        let b = Gc::new(
            &heap,
            WithChild {
                value: 2,
                child: Some(a.id()),
            },
        );

        // Create cycle: a -> b -> a
        a.with_mut(|a_mut| a_mut.child = Some(b.id()));
        a.store(b.id());
        b.store(a.id());

        // Both are rooted, so both survive
        let freed = heap.collect();
        assert_eq!(freed, 0);

        // Unroot both → cycle becomes unreachable
        a.unroot();
        b.unroot();

        let freed = heap.collect();
        assert_eq!(freed, 2);
    }
}
