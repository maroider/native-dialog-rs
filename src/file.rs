use std::path::Path;

pub struct OpenSingleFile<'a> {
    pub dir: Option<&'a Path>,
    pub filter: Option<&'a [&'a str]>,
}

pub struct OpenMultipleFile<'a> {
    pub dir: Option<&'a Path>,
    pub filter: Option<&'a [&'a str]>,
}

pub struct OpenSingleDir<'a> {
    pub dir: Option<&'a Path>,
}

pub struct SaveFile<'a> {
    pub dir: Option<&'a Path>,
    pub name: &'a str,
}
