use core::ops::{Deref, DerefMut};

#[derive(Debug, Eq, PartialEq)]
pub enum PossiblyUninit<T> {
    Init(T),
    Uninit,
}

#[allow(unused)]
impl<T> PossiblyUninit<T> {
    pub fn unwrap_ref(&self) -> &T {
        match self {
            PossiblyUninit::Init(v) => v,
            PossiblyUninit::Uninit => panic!("Use of uninitialized value"),
        }
    }

    pub fn unwrap_ref_mut(&mut self) -> &mut T {
        match self {
            PossiblyUninit::Init(v) => v,
            PossiblyUninit::Uninit => panic!("Use of uninitialized value"),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            PossiblyUninit::Init(v) => v,
            PossiblyUninit::Uninit => panic!("Use of uninitialized value"),
        }
    }

    pub fn is_init(&self) -> bool {
        matches!(self, Self::Init(_))
    }

    pub fn is_uninit(&self) -> bool {
        matches!(self, Self::Uninit)
    }
}

impl<T> Deref for PossiblyUninit<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.unwrap_ref()
    }
}

impl<T> DerefMut for PossiblyUninit<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.unwrap_ref_mut()
    }
}
