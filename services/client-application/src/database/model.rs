// Misc models

pub struct PageSubset<T> {
    total: usize,
    items_subset: Vec<T>,
}

impl<T> PageSubset<T> {
    pub const fn new(total: usize, items_subset: Vec<T>) -> Self {
        Self {
            total,
            items_subset,
        }
    }

    pub const fn total(&self) -> usize {
        self.total
    }

    pub const fn items_subset(&self) -> &Vec<T> {
        &self.items_subset
    }
}
