use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use core::fmt::Display;
use core::ops::{Div, Index, IndexMut};

#[derive(Ord, Eq, PartialEq, PartialOrd, Debug, Clone, Hash)]
pub struct Path {
    components: VecDeque<String>,
    is_absolute: bool,
    is_dir: bool,
}

impl Path {
    pub fn new(mut path: &str) -> Self {
        let is_absolute = path.starts_with("/");
        let is_dir = path.ends_with("/");

        path = path.strip_prefix("/").unwrap_or(path);
        path = path.strip_suffix("/").unwrap_or(path);

        Self {
            components: if path.is_empty() {
                VecDeque::new()
            } else {
                path.split('/')
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            },
            is_absolute,
            is_dir,
        }
    }

    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    pub fn set_is_dir(&mut self, is_dir: bool) {
        self.is_dir = is_dir;
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn last_component(&self) -> Option<&String> {
        self.components.back()
    }

    pub fn strip_prefix(&self, prefix: &Path) -> Result<Path, ()> {
        if self.is_relative_to(prefix) {
            Ok(Path {
                components: self
                    .components
                    .iter()
                    .skip(prefix.components.len())
                    .cloned()
                    .collect(),
                is_absolute: false,
                is_dir: self.is_dir,
            })
        } else {
            Err(())
        }
    }

    pub fn relative_to(&self, prefix: &Path) -> Path {
        if self.is_relative_to(prefix) {
            Path {
                components: self
                    .components
                    .iter()
                    .skip(prefix.components.len())
                    .cloned()
                    .collect(),
                is_absolute: false,
                is_dir: self.is_dir,
            }
        } else {
            panic!("relative_to() called on a non-root path")
        }
    }

    pub fn is_relative_to(&self, prefix: &Path) -> bool {
        self.is_absolute == prefix.is_absolute
            && self
                .components
                .iter()
                .zip(prefix.components.iter())
                .all(|(a, b)| a == b)
            && self.components.len() >= prefix.components.len()
    }

    pub fn pop_front(&mut self) -> Option<String> {
        self.components.pop_front()
    }

    pub fn push_front(&mut self, component: String) {
        self.components.insert(0, component);
    }

    pub fn pop_back(&mut self) -> Option<String> {
        self.is_dir = true; // popping the back component always results in a directory path, since the popped component is always the last component
        self.components.pop_back()
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &String> {
        self.components.iter()
    }
}

impl Index<usize> for Path {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        &self.components[index]
    }
}

impl IndexMut<usize> for Path {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.components[index]
    }
}

impl Div for Path {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut new_components = self.components.clone();
        new_components.extend(rhs.components);
        Self {
            components: new_components,
            is_absolute: self.is_absolute,
            is_dir: rhs.is_dir,
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_absolute {
            write!(f, "/")?;
        }

        for (idx, component) in self.components.iter().enumerate() {
            if idx > 0 {
                write!(f, "/")?;
            }

            write!(f, "{}", component)?;
        }

        if self.is_dir && !(self.is_empty() && self.is_absolute) {
            write!(f, "/")?;
        }

        Ok(())
    }
}
