//! Safe translation of preshed 3.0.13 terminal-key iteration. See third-party notices.
use super::PhraseRuleId;
#[derive(Clone, Default)]
enum Cell {
    #[default]
    Empty,
    Deleted,
    Key(u64, PhraseRuleId),
}
pub(super) struct Terminal {
    cells: Vec<Cell>,
    filled: usize,
    special: [Option<PhraseRuleId>; 2],
}
impl Default for Terminal {
    fn default() -> Self {
        Self {
            cells: vec![Cell::Empty; 8],
            filled: 0,
            special: [None, None],
        }
    }
}
impl Terminal {
    pub fn insert(&mut self, key: u64, rule: PhraseRuleId) {
        if key < 2 {
            self.special[key as usize] = Some(rule);
            return;
        }
        let mask = self.cells.len() - 1;
        let mut index = key as usize & mask;
        let mut deleted = None;
        loop {
            match &self.cells[index] {
                Cell::Empty => break,
                Cell::Key(k, _) if *k == key => break,
                Cell::Deleted => deleted = Some(index),
                _ => (),
            }
            index = (index + 1) & mask;
        }
        if let Some(slot) = deleted {
            if matches!(self.cells[index], Cell::Key(_, _)) {
                self.cells[index] = Cell::Deleted;
            }
            index = slot;
        }
        if matches!(self.cells[index], Cell::Empty) {
            self.filled += 1;
        }
        self.cells[index] = Cell::Key(key, rule);
        if (self.filled + 1) * 5 >= self.cells.len() * 3 {
            let size = self.cells.len() * 2;
            let old = std::mem::replace(&mut self.cells, vec![Cell::Empty; size]);
            self.filled = 0;
            for cell in old {
                if let Cell::Key(k, r) = cell {
                    self.insert(k, r);
                }
            }
        }
    }
    pub fn remove(&mut self, key: u64) {
        if key < 2 {
            self.special[key as usize] = None;
            return;
        }
        let mask = self.cells.len() - 1;
        let mut index = key as usize & mask;
        loop {
            match self.cells[index] {
                Cell::Empty => return,
                Cell::Key(k, _) if k == key => {
                    self.cells[index] = Cell::Deleted;
                    return;
                }
                _ => index = (index + 1) & mask,
            }
        }
    }
    pub fn rules(&self) -> impl Iterator<Item = &PhraseRuleId> {
        self.cells
            .iter()
            .filter_map(|cell| match cell {
                Cell::Key(_, rule) => Some(rule),
                _ => None,
            })
            .chain(self.special.iter().flatten())
    }
}
