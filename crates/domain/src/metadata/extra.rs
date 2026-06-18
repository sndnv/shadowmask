#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraKind {
    Trailer,
    Featurette,
    BehindTheScenes,
}

#[derive(Debug, Clone)]
pub struct Extra {
    pub kind: ExtraKind,
    pub title: String,
    pub path: String,
}
