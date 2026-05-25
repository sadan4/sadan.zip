use std::{collections::HashMap, mem};

struct Entry {
	key: String,
	priority: i32,
}

pub struct QueueUnderflowError;

/// A min-priority queue data structure. This algorithm is derived from Cormen,
/// et al., "Introduction to Algorithms". The basic idea of a min-priority
/// queue is that you can efficiently (in O(1) time) get the smallest key in
/// the queue. Adding and removing elements takes O(log n) time. A key can
/// have its priority decreased in O(log n) time.
pub struct PriorityQueue {
	arr: Vec<Entry>,
	key_indices: HashMap<String, usize>,
}

impl PriorityQueue {
	pub fn new() -> Self {
		Self {
			arr: Vec::new(),
			key_indices: HashMap::new(),
		}
	}
	/// Returns the number of elements in the queue. Takes `O(1)` time.
	pub fn size(&self) -> usize {
		self.arr.len()
	}
	/// Returns the keys that are in the queue. Takes `O(n)` time.
	pub fn keys(&self) -> impl Iterator<Item = &String> {
		self.arr.iter().map(|entry| &entry.key)
	}
	/// Returns `true` if **key** is in the queue and `false` if not.
	pub fn has(&self, key: &str) -> bool {
		self.key_indices.contains_key(key)
	}
	/// Returns the priority for **key**. If **key** is not present in the queue
	/// then this function returns `undefined`. Takes `O(1)` time.
	pub fn priority(&self, key: &str) -> Option<i32> {
		self.key_indices
			.get(key)
			.map(|idx| self.arr[*idx].priority)
	}
	/// Returns the key for the minimum element in this queue. If the queue is
	/// empty this function throws an Error. Takes `O(1)` time.
	pub fn min(&self) -> Result<&String, QueueUnderflowError> {
		self.arr
			.first()
			.map_or(Err(QueueUnderflowError), |entry| Ok(&entry.key))
	}

	/// Inserts a new key into the priority queue. If the key already exists in
	/// the queue this function returns `false`; otherwise it will return `true`.
	///
	/// Takes `O(n)` time.
	pub fn add(&mut self, key: String, priority: i32) -> bool {
		if self.has(&key) {
			return false;
		}
		let idx = self.arr.len();
		self.key_indices
			.insert(key.clone(), idx);
		self.arr.push(Entry { key, priority });
		self.decrease_(idx);
		true
	}
	/// Removes and returns the smallest key in the queue. Takes `O(log n)` time.
	pub fn remove_min(&mut self) -> String {
		self.swap(0, self.arr.len() - 1);
		let min = self.arr.pop().unwrap();
		self.key_indices.remove(&min.key);
		self.heapify(0);
		min.key
	}
	/// Decreases the priority for **key** to **priority**. If the new priority is
	/// greater than the previous priority, this function will throw an Error.
	pub fn decrease(&mut self, key: &str, priority: i32) {
		let Some(idx) = self.key_indices.get(key) else {
			panic!("Key not found: {key}");
		};
		let current_priority = self.arr[*idx].priority;
		if priority > current_priority {
			panic!(
				"New priority is greater than current priority. Key: {key} Old: {current_priority} New: {priority}"
			)
		}
		self.arr[*idx].priority = priority;
		self.decrease_(*idx);
	}
	fn heapify(&mut self, i: usize) {
		let l = 2 * i;
		let r = l + 1;
		let mut largest = i;

		if l < self.arr.len() {
			largest = if self.arr[l].priority < self.arr[largest].priority {
				l
			} else {
				largest
			};
			if r < self.arr.len() {
				largest = if self.arr[r].priority < self.arr[largest].priority {
					r
				} else {
					largest
				}
			}
			if largest != i {
				self.swap(i, largest);
				self.heapify(largest);
			}
		}
	}
	fn decrease_(&mut self, mut idx: usize) {
		let priority = self.arr[idx].priority;
		let mut parent;
		while idx != 0 {
			parent = idx >> 1;
			if self.arr[parent].priority < priority {
				break;
			}
			self.swap(idx, parent);
			idx = parent;
		}
	}
	fn swap(&mut self, i: usize, j: usize) {
		self.arr.swap(i, j);
		*self
			.key_indices
			.get_mut(&self.arr[i].key)
			.unwrap() = i;
		*self
			.key_indices
			.get_mut(&self.arr[j].key)
			.unwrap() = j;
	}
}
