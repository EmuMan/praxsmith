use std::marker::PhantomData;

#[derive(Debug)]
pub struct Handle<T> {
    index: u32,
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Handle<T> {
    fn new(index: u32, generation: u32) -> Self {
        Handle {
            index,
            generation,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct StoreEntry<T> {
    item: Option<T>,
    current_generation: u32,
}

impl<T> Default for StoreEntry<T> {
    fn default() -> Self {
        StoreEntry {
            item: None,
            current_generation: 0,
        }
    }
}

impl<T> StoreEntry<T> {
    fn new(item: T) -> Self {
        StoreEntry {
            item: Some(item),
            current_generation: 0,
        }
    }
}

pub struct Store<T> {
    pub entries: Vec<StoreEntry<T>>,
    open_indices: Vec<usize>,
}

impl<T> Default for Store<T> {
    fn default() -> Self {
        Store {
            entries: Vec::new(),
            open_indices: Vec::new(),
        }
    }
}

impl<T> Store<T> {
    pub fn new() -> Self {
        Store::default()
    }

    fn get_open_entry_mut(&mut self) -> Option<(usize, &mut StoreEntry<T>)> {
        self.open_indices.pop().and_then(|entry_index| {
            self.entries
                .get_mut(entry_index)
                .map(|entry| (entry_index, entry))
        })
    }

    pub fn add(&mut self, item: T) -> Handle<T> {
        if let Some((entry_index, open_entry)) = self.get_open_entry_mut() {
            if open_entry.item.is_some() {
                panic!("Store attempted to override existing entry.")
            }
            open_entry.item = Some(item);
            open_entry.current_generation += 1;
            Handle::new(entry_index as u32, open_entry.current_generation)
        } else {
            self.entries.push(StoreEntry::new(item));
            Handle::new(self.entries.len() as u32 - 1, 0)
        }
    }

    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let entry = self.entries.get(handle.index as usize)?;
        if handle.generation == entry.current_generation {
            entry.item.as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        let entry = self.entries.get_mut(handle.index as usize)?;
        if handle.generation == entry.current_generation {
            entry.item.as_mut()
        } else {
            None
        }
    }

    pub fn remove(&mut self, handle: Handle<T>) -> Result<(), &'static str> {
        let entry = self
            .entries
            .get_mut(handle.index as usize)
            .ok_or("entry not found at index")?;
        if handle.generation != entry.current_generation {
            Err("entry generation does not match")
        } else {
            entry.item = None;
            self.open_indices.push(handle.index as usize);
            Ok(())
        }
    }
}
