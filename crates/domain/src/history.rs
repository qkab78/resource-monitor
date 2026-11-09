use std::collections::VecDeque;

/// Dernières valeurs d'une mesure, les plus anciennes étant oubliées.
#[derive(Clone, Debug, PartialEq)]
pub struct History {
    capacity: usize,
    values: VecDeque<f64>,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            values: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.values.len() == self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = f64> + '_ {
        self.values.iter().copied()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_valeurs_les_plus_anciennes_sont_oubliees() {
        let mut history = History::new(2);
        for value in [1.0, 2.0, 3.0] {
            history.push(value);
        }
        assert_eq!(history.values().collect::<Vec<_>>(), [2.0, 3.0]);
    }
}
