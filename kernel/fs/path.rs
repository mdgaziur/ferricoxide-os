use core::fmt::{Display, Formatter};

/// This represents a path. Given path will be made absolute when
/// `new` is called.
///
/// # NOTE
/// This isn't meant to represent paths like `/../xyz/abc` or `../xyz/`.
/// Caller must include the path before`../`.
/// Meaning, an ideal path can look like this `/home/user/../xyz/abc`. That means, `..` isn't allowed
/// to be the first segment of a path.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Path {
    segments: Vec<String>,
}

impl Path {
    pub fn new(path: &str) -> Self {
        let segments = path
            .split("/")
            .filter(|segment| *segment != ".") // '.' is kinda "no op" so we ignore them
            .filter(|segment| *segment != "") // Hack to remove empty strings when path is split
            .map(|segment| String::from(segment))
            .collect::<Vec<_>>();
        assert_ne!(segments.first(), Some(&String::from("..")));
        let segments = Self::canonicalize(segments);

        Self { segments }
    }

    /// Takes path segments and makes the path canonical
    fn canonicalize(mut segments: Vec<String>) -> Vec<String> {
        // We are at the root
        if segments.is_empty() {
            return segments;
        }

        // go through the path and remove previous segment if current segment is ".."
        let mut new_segment = vec![];
        for segment in segments {
            if segment == ".." {
                new_segment.pop();
            } else {
                new_segment.push(segment);
            }
        }

        new_segment
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn first_segment(&self) -> Path {
        self.path_from_range(0, 0)
    }

    pub fn path_from_idx(&self, idx: usize) -> Path {
        self.path_from_range(idx, self.segments.len() - 1)
    }

    pub fn path_from_range(&self, start: usize, end: usize) -> Path {
        let mut new_segment = vec![];

        let mut idx = start;
        while idx <= end {
            new_segment.push(self.segments[idx].clone());
            idx += 1;
        }

        Self {
            segments: new_segment,
        }
    }

    pub fn append(&self, segment: &str) -> Path {
        Path::new(&format!("{}/{}", self, segment))
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "/{}", self.segments.join("/"))
    }
}
