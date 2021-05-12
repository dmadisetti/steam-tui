use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use tui::widgets::ListState;

pub trait Named {
    fn get_name(&self) -> String;
    fn is_valid(&self) -> bool;
}

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    pub query: String,
}

impl<T: Named> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
            query: "".to_string(),
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
            query: "".to_string(),
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().map(|i| {
            *self
                .activated()
                .get(i)
                .expect("Index is guarded by next, previous. This is safe.")
        })
    }

    pub fn activated(&self) -> Vec<&T> {
        let matcher = SkimMatcherV2::default();
        self.items
            .iter()
            .filter(|nameable| {
                matcher
                    .fuzzy_match(&nameable.get_name(), &self.query)
                    .is_some()
            })
            .filter(|nameable| nameable.is_valid())
            .collect::<Vec<_>>()
    }

    pub fn restart(&mut self) {
        if self.activated().is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.activated().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.activated().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

impl<T: Named> Default for StatefulList<T> {
    fn default() -> Self {
        Self::new()
    }
}
