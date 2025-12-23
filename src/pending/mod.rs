mod terminator;

pub use terminator::{terminate_markdown, TerminatorOptions};

pub(crate) use terminator::fix_incomplete_link_or_image;
